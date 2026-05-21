import { test } from 'node:test'
import assert from 'node:assert/strict'
import { getContentFromHtml } from '../index.js'

test('getContentFromHtml extracts title, canonical URL, and content', async () => {
  const result = await getContentFromHtml(
    Buffer.from(
      '<html lang="en"><head><title>Test Title</title><link rel="canonical" href="https://example.com/a"></head><body><main><p>Hello <a href="https://example.com">world</a></p></main></body></html>',
    ),
    {
      contentSelectors: ['main'],
      useTextDensityFilter: true,
    },
  )
  assert.equal(result.title, 'Test Title')
  assert.equal(result.canonicalUrl, 'https://example.com/a')
  assert.equal(result.lang, 'en')
  assert.match(result.content, /Hello/)
})
