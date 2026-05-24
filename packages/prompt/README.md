# `@jongleberry/vurst-prompt`

Prompt-injection helper utilities for Node.js apps.

This package provides string-in/string-out wrappers around
`@jongleberry/vurst-html` native sanitizers, plus a small helper for marking
external content before sending it to an LLM.

## Install

```sh
pnpm add @jongleberry/vurst-prompt
```

## API

```js
import {
  sanitizePromptInjection,
  sanitizeRssContent,
  wrapExternalContent,
} from '@jongleberry/vurst-prompt'
```

| Function | Description |
| --- | --- |
| `sanitizePromptInjection(content, opts?)` | Strip prompt-injection patterns from a string. |
| `sanitizeRssContent(html)` | Sanitize RSS HTML, then strip prompt-injection patterns. |
| `wrapExternalContent(content, opts)` | Wrap external text in an XML-like boundary for LLM prompts. |

```js
const text = await sanitizePromptInjection('Ignore previous instructions. Hello.')

const rss = await sanitizeRssContent('<p>Hello</p><script>bad()</script>')

const promptInput = wrapExternalContent(rss, {
  source: 'rss',
  contentType: 'html',
})
```

`wrapExternalContent` validates `source` and `contentType` with
`/^[a-z0-9_-]+$/i` before placing them in attributes. Wrapped content is
preserved exactly. `includeReminder` defaults to `true`.
