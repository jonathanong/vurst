import { test } from 'node:test'
import assert from 'node:assert/strict'
import { sanitizeRssHtml, sanitizeRssHtmlBatch } from '../index.js'

test('sanitizeRssHtml strips dangerous tags and captures first image src', async () => {
  const result = await sanitizeRssHtml(
    Buffer.from('<p>Hello</p><script>bad()</script><img src="https://example.com/a.png">'),
  )
  const html = result.html.toString('utf8')
  assert.match(html, /<p>Hello<\/p>/)
  assert.doesNotMatch(html, /<script/)
  assert.equal(result.firstImageSrc, 'https://example.com/a.png')
})

test('sanitizeRssHtml proxies external image URLs when proxyImages is true', async () => {
  const result = await sanitizeRssHtml(
    Buffer.from('<img src="https://example.com/a.png">'),
    { proxyImages: true },
  )
  const html = result.html.toString('utf8')
  assert.match(html, /\/proxy\//)
  assert.doesNotMatch(html, /example\.com\/a\.png/)
})

test('sanitizeRssHtml honors a custom imageProxyUrlPrefix', async () => {
  const result = await sanitizeRssHtml(
    Buffer.from('<img src="https://example.com/a.png">'),
    { proxyImages: true, imageProxyUrlPrefix: '/i/' },
  )
  const html = result.html.toString('utf8')
  assert.match(html, /\/i\//)
})

test('sanitizeRssHtmlBatch returns one result per input', async () => {
  const batch = await sanitizeRssHtmlBatch([
    Buffer.from('<p>One</p>'),
    Buffer.from('<script>bad()</script><p>Two</p>'),
  ])
  assert.equal(batch.length, 2)
  assert.match(batch[0].html.toString('utf8'), /<p>One<\/p>/)
  assert.doesNotMatch(batch[1].html.toString('utf8'), /<script/)
})

test('sanitizeRssHtml rejects oversized input', async () => {
  const oversized = Buffer.alloc(10 * 1024 * 1024 + 1, 0x20)
  await assert.rejects(sanitizeRssHtml(oversized), /Input too large/)
})

test('sanitizeRssHtml rejects invalid UTF-8 input', async () => {
  const invalidUtf8 = Buffer.from([0xff, 0xfe, 0xfd])
  await assert.rejects(sanitizeRssHtml(invalidUtf8), /Invalid UTF-8/)
})

test('sanitizeRssHtmlBatch rejects invalid UTF-8 input', async () => {
  const invalidUtf8 = Buffer.from([0xff, 0xfe, 0xfd])
  await assert.rejects(sanitizeRssHtmlBatch([invalidUtf8]), /Invalid UTF-8/)
})
