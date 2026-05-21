'use strict'

// Shutdown drain wrapper for @jongleberry/vurst-* N-API packages.
//
// Provides a keep-alive handle while native work is in flight, shutdown
// signalling, and a drain-wait primitive for graceful process exit.

const SHUTDOWN_ERROR_MESSAGE =
  'Rust N-API runtime is shutting down; refusing to start new native work.'

/**
 * @param {unknown} value
 * @returns {value is PromiseLike<unknown>}
 */
function isPromiseLike(value) {
  if (value == null) return false
  if (typeof value !== 'object' && typeof value !== 'function') return false
  return typeof (/** @type {any} */ (value).then) === 'function'
}

/**
 * Create an isolated runtime instance.
 *
 * Returns an object with:
 *   - `beginShutdown()` — mark runtime as shutting down
 *   - `getPendingCount()` — number of in-flight native calls
 *   - `isShuttingDown()` — whether shutdown has been signalled
 *   - `resetForTests()` — reset all state (use only in tests)
 *   - `runNativeFunction(fn, receiver, args)` — track and call a native fn
 *   - `waitForDrain()` — Promise that resolves when pendingCount reaches 0
 */
function createNativeRuntime() {
  let pendingCount = 0
  let shuttingDown = false
  /** @type {NodeJS.Timeout | null} */
  let keepAliveHandle = null
  /** @type {Set<() => void>} */
  const drainResolvers = new Set()

  function ensureKeepAliveHandle() {
    if (keepAliveHandle != null) return
    // Keep the event loop alive while native work is in flight. Some N-API async
    // completions do not show up as ordinary JS handles, so without this guard a
    // worker can start tearing down before the Promise settles back into JS.
    keepAliveHandle = setInterval(() => {}, 60_000)
  }

  function releaseKeepAliveHandle() {
    if (pendingCount !== 0 || keepAliveHandle == null) return
    clearInterval(keepAliveHandle)
    keepAliveHandle = null
  }

  function resolveDrainWaiters() {
    if (pendingCount !== 0) return
    for (const resolve of drainResolvers) resolve()
    drainResolvers.clear()
  }

  /**
   * @template T
   * @param {PromiseLike<T>} promise
   * @returns {Promise<T>}
   */
  function trackPromise(promise) {
    pendingCount += 1
    ensureKeepAliveHandle()

    return Promise.resolve(promise).finally(() => {
      pendingCount -= 1
      releaseKeepAliveHandle()
      resolveDrainWaiters()
    })
  }

  /**
   * @param {Function} fn
   * @param {unknown} receiver
   * @param {unknown[]} args
   * @returns {unknown}
   */
  function runNativeFunction(fn, receiver, args) {
    if (shuttingDown) throw new Error(SHUTDOWN_ERROR_MESSAGE)
    const result = Reflect.apply(fn, receiver, args)
    if (!isPromiseLike(result)) return result
    return trackPromise(result)
  }

  /** @returns {Promise<void>} */
  function waitForDrain() {
    if (pendingCount === 0) return Promise.resolve()
    return new Promise(resolve => drainResolvers.add(resolve))
  }

  function beginShutdown() {
    shuttingDown = true
  }

  function resetForTests() {
    shuttingDown = false
    pendingCount = 0
    drainResolvers.clear()
    if (keepAliveHandle != null) clearInterval(keepAliveHandle)
    keepAliveHandle = null
  }

  return {
    beginShutdown,
    getPendingCount: () => pendingCount,
    isShuttingDown: () => shuttingDown,
    resetForTests,
    runNativeFunction,
    waitForDrain,
  }
}

const sharedRuntime = createNativeRuntime()

/**
 * Wrap a native N-API addon object so every function call is tracked through
 * the runtime for shutdown/drain handling.
 *
 * @template {Record<PropertyKey, unknown>} T
 * @param {T} addon
 * @param {ReturnType<typeof createNativeRuntime>} [runtime]
 * @returns {T}
 */
function wrapNativeAddon(addon, runtime) {
  const rt = runtime ?? sharedRuntime
  /** @type {Map<PropertyKey, { original: Function, wrapped: Function }>} */
  const wrappedFunctions = new Map()

  return new Proxy(addon, {
    get(target, prop, receiver) {
      const value = Reflect.get(target, prop, receiver)
      if (typeof value !== 'function') return value

      const cached = wrappedFunctions.get(prop)
      if (cached != null && cached.original === value) return cached.wrapped

      const wrapped = (...args) => rt.runNativeFunction(value, target, args)
      wrappedFunctions.set(prop, { original: value, wrapped })
      return wrapped
    },
  })
}

/** Signal the shared runtime to stop accepting new native work. */
function beginNativeAddonShutdown() {
  sharedRuntime.beginShutdown()
}

/** @returns {boolean} */
function isNativeAddonShuttingDown() {
  return sharedRuntime.isShuttingDown()
}

/** @returns {Promise<void>} */
function waitForNativeAddonWorkToDrain() {
  return sharedRuntime.waitForDrain()
}

module.exports.createNativeRuntime = createNativeRuntime
module.exports.wrapNativeAddon = wrapNativeAddon
module.exports.beginNativeAddonShutdown = beginNativeAddonShutdown
module.exports.isNativeAddonShuttingDown = isNativeAddonShuttingDown
module.exports.waitForNativeAddonWorkToDrain = waitForNativeAddonWorkToDrain
