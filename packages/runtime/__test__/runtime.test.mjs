import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  createNativeRuntime,
  wrapNativeAddon,
  beginNativeAddonShutdown,
  isNativeAddonShuttingDown,
  waitForNativeAddonWorkToDrain,
} from '../index.js'

// ============================================================================
// createNativeRuntime — isolated instances
// ============================================================================

test('initial state: pendingCount=0, not shutting down', () => {
  const rt = createNativeRuntime()
  assert.equal(rt.getPendingCount(), 0)
  assert.equal(rt.isShuttingDown(), false)
})

test('runNativeFunction tracks a synchronous result', () => {
  const rt = createNativeRuntime()
  const result = rt.runNativeFunction(() => 42, null, [])
  assert.equal(result, 42)
  assert.equal(rt.getPendingCount(), 0)
})

test('runNativeFunction tracks an async result and increments/decrements pending count', async () => {
  const rt = createNativeRuntime()
  let resolveInner
  const inner = new Promise(resolve => {
    resolveInner = resolve
  })
  const trackingPromise = rt.runNativeFunction(() => inner, null, [])
  assert.equal(rt.getPendingCount(), 1)
  resolveInner('done')
  await trackingPromise
  assert.equal(rt.getPendingCount(), 0)
})

test('waitForDrain resolves immediately when no pending work', async () => {
  const rt = createNativeRuntime()
  await assert.doesNotReject(rt.waitForDrain())
})

test('waitForDrain resolves after all pending work completes', async () => {
  const rt = createNativeRuntime()
  let resolveInner
  const inner = new Promise(resolve => {
    resolveInner = resolve
  })
  rt.runNativeFunction(() => inner, null, [])
  assert.equal(rt.getPendingCount(), 1)

  const drainPromise = rt.waitForDrain()
  resolveInner()
  await drainPromise
  assert.equal(rt.getPendingCount(), 0)
})

test('beginShutdown rejects new native work', () => {
  const rt = createNativeRuntime()
  rt.beginShutdown()
  assert.equal(rt.isShuttingDown(), true)
  assert.throws(() => rt.runNativeFunction(() => {}, null, []), /shutting down/)
})

test('resetForTests clears shutdown and pending state', () => {
  const rt = createNativeRuntime()
  rt.beginShutdown()
  rt.resetForTests()
  assert.equal(rt.isShuttingDown(), false)
  assert.equal(rt.getPendingCount(), 0)
  // Should not throw after reset
  assert.doesNotThrow(() => rt.runNativeFunction(() => {}, null, []))
})

// ============================================================================
// wrapNativeAddon
// ============================================================================

test('wrapNativeAddon proxies function calls through runtime', async () => {
  const rt = createNativeRuntime()
  rt.resetForTests()
  let resolveInner
  const inner = new Promise(resolve => {
    resolveInner = resolve
  })
  const addon = { doWork: () => inner }
  const wrapped = wrapNativeAddon(addon, rt)
  const result = wrapped.doWork()
  assert.equal(rt.getPendingCount(), 1)
  resolveInner('ok')
  assert.equal(await result, 'ok')
  assert.equal(rt.getPendingCount(), 0)
})

test('wrapNativeAddon passes non-function properties through unchanged', () => {
  const rt = createNativeRuntime()
  const addon = { version: '1.0.0', doWork: () => {} }
  const wrapped = wrapNativeAddon(addon, rt)
  assert.equal(wrapped.version, '1.0.0')
})

test('wrapNativeAddon caches wrapped functions', () => {
  const rt = createNativeRuntime()
  const addon = { doWork: () => {} }
  const wrapped = wrapNativeAddon(addon, rt)
  assert.equal(wrapped.doWork, wrapped.doWork)
})

// ============================================================================
// Shared singleton exports
// ============================================================================

test('shared singleton: beginNativeAddonShutdown and isNativeAddonShuttingDown', () => {
  // We can only test the state transition once since module state persists.
  // Skip if already shut down from a previous test run in the same process.
  if (isNativeAddonShuttingDown()) return

  assert.equal(isNativeAddonShuttingDown(), false)
  beginNativeAddonShutdown()
  assert.equal(isNativeAddonShuttingDown(), true)
})

test('shared singleton: waitForNativeAddonWorkToDrain resolves', async () => {
  await assert.doesNotReject(waitForNativeAddonWorkToDrain())
})
