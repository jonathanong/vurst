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
