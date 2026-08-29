import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { test } from 'node:test'
import { createMarkdownStreamBuffer } from '../streaming-buffer.js'

function assertPushes(buffer, chunks) {
  for (const [input, output] of chunks) {
    assert.deepEqual(buffer.push(input), output)
  }
}

test('flushes safe text and complete Markdown constructs immediately', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [
    ['Hello world', ['Hello world']],
    ['[link](url)', ['[link](url)']],
    ['<div>', ['<div>']],
    ['`code`', ['`code`']],
  ])
})

test('returns no output for an empty push and preserves Unicode text', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [
    ['', []],
    ['Hello 🌍!', ['Hello 🌍!']],
  ])
})

test('holds incomplete Markdown constructs and emits a safe prefix', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [
    ['Some text [link', ['Some text ']],
    [' text', []],
    ['](url)', ['[link text](url)']],
    ['<div', []],
  ])
  assert.equal(buffer.flush(), '<div')
})

test('holds incomplete images, trailing escapes, and unmatched trailing backticks', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [['![alt](https://example.com', []]])
  assert.equal(buffer.flush(), '![alt](https://example.com')
  assertPushes(buffer, [['text\\', ['text']]])
  assert.equal(buffer.flush(), '\\')
  assertPushes(buffer, [['code```', ['code']]])
  assert.equal(buffer.flush(), '```')
})

test('emits escaped literal brackets and complete nested-label links', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [
    ['\\[literal]', ['\\[literal]']],
    ['[outer [inner]](url)', ['[outer [inner]](url)']],
  ])
})

test('holds a complete label until the next chunk establishes whether it starts a destination', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [
    ['[label]', []],
    [' ordinary text', ['[label] ordinary text']],
  ])
})

test('holds nested and escaped destination parentheses until the outer destination closes', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [
    ['[link](https://example.com/a_(b', []],
    ['))', ['[link](https://example.com/a_(b))']],
    ['[link](url\\)still)', ['[link](url\\)still)']],
  ])
})

test('holds and flushes text before a single unmatched backtick', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [['text`', ['text']]])
  assert.equal(buffer.flush(), '`')
})

test('emits complete links surrounded by safe text', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [['see [link](url) here', ['see [link](url) here']]])
})

test('holds from the first unclosed bracket even when a later link is complete', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [['[unclosed and [closed](url)', []]])
  assert.equal(buffer.flush(), '[unclosed and [closed](url)')
})

test('flush returns pending content and resets state', () => {
  const buffer = createMarkdownStreamBuffer()

  assertPushes(buffer, [['[incomplete', []]])
  assert.equal(buffer.flush(), '[incomplete')
  assert.equal(buffer.flush(), '')
  assertPushes(buffer, [['safe', ['safe']]])
})

test('expires held content using the injected clock without sleeping', () => {
  let currentTime = 100
  const buffer = createMarkdownStreamBuffer({
    maxHoldMs: 50,
    now: () => currentTime,
  })

  assertPushes(buffer, [['[incomplete', []]])
  currentTime += 49
  assertPushes(buffer, [['', []]])
  currentTime += 1
  assertPushes(buffer, [
    ['', ['[incomplete']],
    ['safe', ['safe']],
  ])
})

test('a zero hold duration flushes incomplete content immediately', () => {
  const buffer = createMarkdownStreamBuffer({ maxHoldMs: 0, now: () => 0 })

  assertPushes(buffer, [['[incomplete', ['[incomplete']]])
})

test('validates public inputs and options', () => {
  assert.throws(() => createMarkdownStreamBuffer(null), TypeError)
  assert.throws(() => createMarkdownStreamBuffer({ maxHoldMs: -1 }), RangeError)
  assert.throws(() => createMarkdownStreamBuffer({ maxHoldMs: Infinity }), RangeError)
  assert.throws(() => createMarkdownStreamBuffer({ now: 'clock' }), TypeError)

  const buffer = createMarkdownStreamBuffer()
  assert.throws(() => buffer.push(Buffer.from('text')), TypeError)
})

test('the streaming-buffer subpath loads without the native binding in ESM and CommonJS', () => {
  const commands = [
    [
      '--input-type=module',
      '--eval',
      "import { createMarkdownStreamBuffer } from '@jongleberry/vurst-markdown/streaming-buffer'; const buffer = createMarkdownStreamBuffer(); if (buffer.push('safe')[0] !== 'safe') process.exit(1)",
    ],
    [
      '--eval',
      "const { createMarkdownStreamBuffer } = require('@jongleberry/vurst-markdown/streaming-buffer'); const buffer = createMarkdownStreamBuffer(); if (buffer.push('safe')[0] !== 'safe') process.exit(1)",
    ],
  ]

  for (const args of commands) {
    const result = spawnSync(process.execPath, args, {
      cwd: new URL('..', import.meta.url),
      encoding: 'utf8',
    })
    assert.equal(result.status, 0, result.stderr)
  }
})
