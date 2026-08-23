import { test } from 'node:test'
import assert from 'node:assert/strict'
import { decodeHtml } from '../index.js'

test('decodeHtml exposes BOM, header, meta, and fallback charset decoding', async () => {
  assert.equal(
    await decodeHtml(Buffer.from([0xff, 0xfe, 0x41, 0x00]), 'text/html; charset=windows-1252'),
    'A',
  )
  assert.match(
    await decodeHtml(Buffer.from('<meta charset=windows-1252><p>\x93Hello\x94', 'latin1')),
    /“Hello”/,
  )
  assert.equal(await decodeHtml(Buffer.from([0x93, 0x48, 0x94])), '“H”')
  await assert.rejects(decodeHtml(Buffer.alloc(10 * 1024 * 1024 + 1)), /Input too large/)
})
