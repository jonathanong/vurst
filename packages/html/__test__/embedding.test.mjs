import { test } from 'node:test'
import assert from 'node:assert/strict'
import { htmlToEmbeddingText } from '../index.js'

test('htmlToEmbeddingText strips URLs and keeps text content', async () => {
  const text = await htmlToEmbeddingText(
    Buffer.from('<p>Hello <a href="https://example.com">world</a></p>'),
  )
  assert.match(text, /Hello/)
  assert.match(text, /world/)
  assert.doesNotMatch(text, /example\.com/)
})

test('htmlToEmbeddingText rejects oversized input', async () => {
  const oversized = Buffer.alloc(10 * 1024 * 1024 + 1, 0x20)
  await assert.rejects(htmlToEmbeddingText(oversized), /Input too large/)
})
