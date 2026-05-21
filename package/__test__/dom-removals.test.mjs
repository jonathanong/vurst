import { test } from 'node:test'
import assert from 'node:assert/strict'
import { extractDomRemovals, applyDomRemovalsToHtml } from '../index.js'

test('extractDomRemovals + applyDomRemovalsToHtml strips shared boilerplate', async () => {
  const removals = await extractDomRemovals(
    [
      Buffer.from(
        '<html><body><nav class="shared">Shared boilerplate text that should be removed from every page</nav><main><h1>A</h1><p>Article one</p></main></body></html>',
      ),
      Buffer.from(
        '<html><body><nav class="shared">Shared boilerplate text that should be removed from every page</nav><main><h1>B</h1><p>Article two</p></main></body></html>',
      ),
    ],
    { boilerplatePatterns: ['boilerplate'] },
  )
  assert.ok(
    removals.cssSelectorsToRemove.length > 0 || removals.htmlToRemove.length > 0,
    'expected at least one removal',
  )

  const cleaned = await applyDomRemovalsToHtml(
    Buffer.from('<main><p>Keep</p><aside>Drop</aside></main>'),
    { cssSelectorsToRemove: ['aside'], htmlToRemove: [] },
  )
  const html = cleaned.toString('utf8')
  assert.match(html, /Keep/)
  assert.doesNotMatch(html, /Drop/)
})
