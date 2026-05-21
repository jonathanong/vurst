# @jongleberry/vurst

High-performance Rust + N-API utilities for Node.js: markdown chunking, HTML
sanitization, markdown → HTML rendering, AI-generated-text detection, and
content extraction. All operations run off-thread on a bounded tokio blocking
pool.

## Install

```sh
npm install @jongleberry/vurst
```

Prebuilt binaries ship for:

- macOS arm64 (`aarch64-apple-darwin`)
- Linux x64 glibc (`x86_64-unknown-linux-gnu`)
- Linux arm64 glibc (`aarch64-unknown-linux-gnu`)

The ONNX Runtime shared library is bundled inside the npm package — no system
install required. glibc 2.17+ compatible (manylinux2014).

## Quick start

```js
import { chunk, sanitizeRssHtml, detectAiGeneratedText } from '@jongleberry/vurst'

const chunks = await chunk(Buffer.from('# Hello\n\nWorld'))
const { html, firstImageSrc } = await sanitizeRssHtml(Buffer.from(rawHtml))
const slop = await detectAiGeneratedText(Buffer.from(text))
```

## Functions

All functions are async and accept inputs up to 10 MiB per call (batch
functions check the total across all inputs).

| Function | Description |
| --- | --- |
| `chunk(text, opts?)` | Heading-aware semantic chunking of Markdown. Wraps `breadchunks`. |
| `sanitizeRssHtml(html, opts?)` | Sanitize raw RSS feed HTML with optional image-proxy URL rewriting. |
| `sanitizeRssHtmlBatch(htmls, opts?)` | Batched form. |
| `renderMarkdownToHtml(text, opts?)` | Render Markdown to HTML. Options: `allowHtml`, `nofollowLinks`, `proxyImages`, `imageProxyUrlPrefix`, `imageProxySigningKeys`. |
| `renderMarkdownToHtmlBatch(texts, opts?)` | Batched form. |
| `extractMarkdownUrls(text)` | Extract link and image URLs from Markdown. |
| `htmlToEmbeddingText(html)` | Convert HTML to clean text suitable for embedding models. |
| `detectAiGeneratedText(text, threshold?)` | ONNX-based AI/slop detection. |
| `extractDomRemovals(htmlPages, opts?)` | Learn boilerplate CSS selectors across pages. Wraps `boilerstrip::learn`. |
| `applyDomRemovalsToHtml(html, removals)` | Apply learned removals. Wraps `boilerstrip::apply_removals`. |
| `getContentFromHtml(html, opts)` | Extract article content from HTML. Wraps `boilerstrip::convert`. |
| `sanitizePromptInjection(content, isTitle?)` | Strip prompt-injection patterns from text. |

## Image proxy

`renderMarkdownToHtml` and `sanitizeRssHtml` can rewrite external `<img src>`
URLs to your image-proxy endpoint.

- `imageProxyUrlPrefix` (default `/proxy/`) — the path prefix prepended to
  proxied URLs. The original URL is base64url-encoded and appended.
- `imageProxySigningKeys` — array of hex-encoded HMAC-SHA256 keys (newest
  first). When provided, proxied URLs include `?sig=<hmac>`. Empty array means
  unsigned (dev mode).

The proxy endpoint is expected to decode the base64url path segment to
recover the original URL.

## Runtime tuning

A single bounded tokio runtime backs all `spawn_blocking` calls so a Node
worker process has a predictable thread budget.

| Env var | Effect | Default |
| --- | --- | --- |
| `RUST_TOKIO_WORKER_THREADS` | Async worker threads. | `2` |
| `RUST_TOKIO_MAX_BLOCKING_THREADS` | Cap on the blocking pool. | `8` |

## License

MIT
