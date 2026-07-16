# vurst monorepo

High-performance Rust + N-API utilities for Node.js, split into independently
publishable packages.

## Packages

| Package | Description |
| --- | --- |
| [`@jongleberry/vurst-html`](packages/html) | HTML sanitization, extraction, embedding prep, and boilerstrip |
| [`@jongleberry/vurst-markdown`](packages/markdown) | Markdown chunking and rendering |
| [`@jongleberry/vurst-ai`](packages/ai) | ONNX-based AI-generated text detection |
| [`@jongleberry/vurst-prompt`](packages/prompt) | Prompt-injection helpers for strings, RSS content, and external prompt boundaries |
| [`@jongleberry/vurst-runtime`](packages/runtime) | Pure-TS shutdown drain wrapper for vurst N-API packages |

All Rust packages run off-thread on a bounded tokio blocking pool.

## Quick start

```sh
pnpm add @jongleberry/vurst-html
pnpm add @jongleberry/vurst-markdown
pnpm add @jongleberry/vurst-ai
pnpm add @jongleberry/vurst-prompt
pnpm add @jongleberry/vurst-runtime
```

Prebuilt binaries ship for:

- macOS arm64 (`aarch64-apple-darwin`)
- Linux x64 glibc (`x86_64-unknown-linux-gnu`)
- Linux arm64 glibc (`aarch64-unknown-linux-gnu`)

`@jongleberry/vurst-html`, `@jongleberry/vurst-markdown`, and
`@jongleberry/vurst-ai` are lightweight meta packages: the native binary for
each supported platform lives in its own `optionalDependency` package (e.g.
`@jongleberry/vurst-ai-darwin-arm64`), and your package manager installs only
the one matching your platform. The ONNX Runtime shared library is bundled
the same way, inside the matching `@jongleberry/vurst-ai-<platform>` package
— no system install required. glibc 2.17+ compatible (manylinux2014).

## `@jongleberry/vurst-html`

```js
import {
  sanitizeRssHtml,
  sanitizeRssHtmlBatch,
  htmlToEmbeddingText,
  extractDomRemovals,
  applyDomRemovalsToHtml,
  getContentFromHtml,
  sanitizePromptInjection,
} from '@jongleberry/vurst-html'
```

| Function | Description |
| --- | --- |
| `sanitizeRssHtml(html, opts?)` | Sanitize raw RSS feed HTML with optional image-proxy URL rewriting. |
| `sanitizeRssHtmlBatch(htmls, opts?)` | Batched form. |
| `htmlToEmbeddingText(html)` | Convert HTML to clean text suitable for embedding models. |
| `extractDomRemovals(htmlPages, opts?)` | Learn boilerplate CSS selectors across pages. Wraps `boilerstrip::learn`. |
| `applyDomRemovalsToHtml(html, removals)` | Apply learned removals. Wraps `boilerstrip::apply_removals`. |
| `getContentFromHtml(html, opts)` | Extract article content from HTML. Wraps `boilerstrip::convert`. |
| `sanitizePromptInjection(content, isTitle?)` | Strip prompt-injection patterns from text. |

## `@jongleberry/vurst-prompt`

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
| `wrapExternalContent(content, opts)` | Wrap external text in an XML-like prompt boundary. |

## `@jongleberry/vurst-markdown`

```js
import {
  chunk,
  renderMarkdownToHtml,
  renderMarkdownToHtmlBatch,
  extractMarkdownUrls,
} from '@jongleberry/vurst-markdown'
```

| Function | Description |
| --- | --- |
| `chunk(text, opts?)` | Heading-aware semantic chunking of Markdown. Wraps `breadchunks`. |
| `renderMarkdownToHtml(text, opts?)` | Render Markdown to HTML. Options: `allowHtml`, `nofollowLinks`, `proxyImages`, `imageProxyUrlPrefix`, `imageProxySigningKeys`. |
| `renderMarkdownToHtmlBatch(texts, opts?)` | Batched form. |
| `extractMarkdownUrls(text)` | Extract link and image URLs from Markdown. |

## `@jongleberry/vurst-ai`

```js
import { detectAiGeneratedText } from '@jongleberry/vurst-ai'
```

| Function | Description |
| --- | --- |
| `detectAiGeneratedText(text, threshold?)` | ONNX-based AI/slop detection. |

## `@jongleberry/vurst-runtime`

```js
import {
  wrapNativeAddon,
  beginNativeAddonShutdown,
  waitForNativeAddonWorkToDrain,
} from '@jongleberry/vurst-runtime'
```

Wraps any vurst N-API addon with a shutdown drain so the process exits cleanly
after all in-flight native work settles.

## Image proxy

`renderMarkdownToHtml` and `sanitizeRssHtml` can rewrite external `<img src>`
URLs to your image-proxy endpoint.

- `imageProxyUrlPrefix` (default `/proxy/`) — the path prefix prepended to
  proxied URLs. The original URL is base64url-encoded and appended.
- `imageProxySigningKeys` — array of hex-encoded HMAC-SHA256 keys (newest
  first). When provided, proxied URLs include `?sig=<hmac>`. Empty array means
  unsigned (dev mode).

## Runtime tuning

A single bounded tokio runtime backs all `spawn_blocking` calls so a Node
worker process has a predictable thread budget.

| Env var | Effect | Default |
| --- | --- | --- |
| `RUST_TOKIO_WORKER_THREADS` | Async worker threads. | `2` |
| `RUST_TOKIO_MAX_BLOCKING_THREADS` | Cap on the blocking pool. | `8` |

## License

MIT
