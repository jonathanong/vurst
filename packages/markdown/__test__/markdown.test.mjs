import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  renderMarkdownToHtml,
  renderMarkdownToHtmlBatch,
  extractMarkdownUrls,
} from '../index.js'

test('renderMarkdownToHtml renders basic markdown', async () => {
  const html = (await renderMarkdownToHtml(Buffer.from('# Hello\n\nWorld'))).toString('utf8')
  assert.match(html, /<h1>Hello<\/h1>/)
  assert.match(html, /<p>World<\/p>/)
})

test('renderMarkdownToHtml proxies external images by default', async () => {
  const html = (
    await renderMarkdownToHtml(Buffer.from('![alt](https://example.com/a.png)'))
  ).toString('utf8')
  assert.match(html, /\/proxy\//)
})

test('renderMarkdownToHtml signs proxy URLs when keys are provided', async () => {
  const key = 'deadbeef'.repeat(8)
  const html = (
    await renderMarkdownToHtml(Buffer.from('![alt](https://example.com/a.png)'), {
      imageProxySigningKeys: [key],
    })
  ).toString('utf8')
  assert.match(html, /\?sig=[0-9a-f]{64}/)
})

test('renderMarkdownToHtmlBatch returns one HTML buffer per input', async () => {
  const out = await renderMarkdownToHtmlBatch([
    Buffer.from('**bold**'),
    Buffer.from('*italic*'),
  ])
  assert.equal(out.length, 2)
  assert.match(out[0].toString('utf8'), /<strong>bold<\/strong>/)
  assert.match(out[1].toString('utf8'), /<em>italic<\/em>/)
})

test('extractMarkdownUrls separates link and image URLs', async () => {
  const urls = await extractMarkdownUrls(
    Buffer.from('[site](https://example.com) ![img](https://example.com/a.png)'),
  )
  assert.deepEqual(urls.linkUrls, ['https://example.com'])
  assert.deepEqual(urls.imageUrls, ['https://example.com/a.png'])
})

test('renderMarkdownToHtml rejects input exceeding size limit', async () => {
  const largeBuffer = Buffer.alloc(10 * 1024 * 1024 + 1)
  await assert.rejects(
    renderMarkdownToHtml(largeBuffer),
    /Input too large/,
  )
})

test('renderMarkdownToHtmlBatch rejects input exceeding size limit', async () => {
  const largeBuffer = Buffer.alloc(10 * 1024 * 1024 + 1)
  await assert.rejects(
    renderMarkdownToHtmlBatch([largeBuffer]),
    /Input too large/,
  )
})

test('renderMarkdownToHtmlBatch rejects total input exceeding size limit', async () => {
  const maxBytes = 10 * 1024 * 1024
  const buf1 = Buffer.alloc(maxBytes / 2 + 100, 'a')
  const buf2 = Buffer.alloc(maxBytes / 2 + 100, 'b')
  await assert.rejects(
    renderMarkdownToHtmlBatch([buf1, buf2]),
    /Input too large/,
  )
})

test('extractMarkdownUrls rejects input exceeding size limit', async () => {
  const largeBuffer = Buffer.alloc(10 * 1024 * 1024 + 1)
  await assert.rejects(
    extractMarkdownUrls(largeBuffer),
    /Input too large/,
  )
})
