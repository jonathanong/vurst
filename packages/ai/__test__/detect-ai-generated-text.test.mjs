import { test } from 'node:test'
import assert from 'node:assert/strict'
import { detectAiGeneratedText } from '../index.js'

test('detectAiGeneratedText returns a valid result shape', async () => {
  const result = await detectAiGeneratedText(Buffer.from('This is some sample text.'))
  assert.equal(typeof result.flagged, 'boolean')
  assert.equal(typeof result.confidenceScore, 'number')
  assert.equal(typeof result.confidenceThreshold, 'number')
  assert.ok(result.confidenceScore >= 0 && result.confidenceScore <= 1)
  assert.ok(result.confidenceThreshold >= 0 && result.confidenceThreshold <= 1)
  assert.ok(['ai', 'human'].includes(result.classification))
  assert.equal(typeof result.detector, 'string')
  assert.equal(typeof result.detectorModelVersion, 'string')
})

test('detectAiGeneratedText rejects an invalid confidence threshold', async () => {
  await assert.rejects(
    detectAiGeneratedText(Buffer.from('text'), 1.5),
    /confidenceThreshold must be between 0.0 and 1.0/,
  )
})
