import { test } from 'node:test'
import assert from 'node:assert/strict'
import { chunk } from '../index.js'

test('chunk produces semantic chunks from markdown', async () => {
  const chunks = await chunk(
    Buffer.from(
      '# Title\n\nIntro paragraph.\n\n## Section A\n\nContent for A.\n\n## Section B\n\nContent for B.',
    ),
  )
  assert.ok(chunks.length >= 1)
  for (const c of chunks) {
    assert.equal(typeof c.text, 'string')
    assert.equal(typeof c.breadcrumb, 'string')
    assert.equal(typeof c.length, 'number')
  }
})

test('chunk respects chunkOptions', async () => {
  const chunks = await chunk(Buffer.from('# H\n\nbody'), {
    minLength: 1,
    maxLength: 1000,
    phase: 0,
    title: 'Doc',
  })
  assert.ok(chunks.length >= 1)
})

test('chunk rejects input exceeding maximum size', async () => {
  const largeBuffer = Buffer.alloc(10 * 1024 * 1024 + 1)
  await assert.rejects(
    chunk(largeBuffer),
    /Input too large/
  )
})
