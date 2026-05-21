#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(unused_doc_comments)]

// === N-API BRIDGE LAYER ===
//
// Thin translation layer between JavaScript and Rust. Mirrors each internal
// Rust type with a `#[napi]` type that can cross the JS/Rust boundary,
// converts between them via `From` impls, and runs CPU-intensive work on a
// bounded blocking pool (see `runtime`) so the Node.js event loop stays
// responsive.
//
// All operations are CPU-intensive. In production, call from background
// workers (e.g. a worker_threads pool or job queue) rather than from request
// handlers.

use napi::bindgen_prelude::*;
use napi_derive::napi;

mod runtime;

use boilerstrip::{apply_removals, convert, learn, ConvertOptions, LearnOptions, Removals};
use vurst::embedding_content;
use vurst::image_proxy::DEFAULT_IMAGE_PROXY_URL_PREFIX;
use vurst::markdown_to_html::{self, MarkdownRenderOptions};
use vurst::sanitize_html::{self, SanitizeRssHtmlOptions};
use vurst::sanitize_prompt_injection;
use vurst::slop_detection;
use vurst::{chunk as core_chunk, Chunk as InternalChunk, ChunkOptions as InternalChunkOptions};

/// Maximum input size for all functions (10 MiB). Batch functions check the
/// total bytes across all inputs. This bounds blocking-pool exposure to large
/// or adversarial inputs.
const SANITIZE_MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;

// ============================================================================
// sanitize_rss_html / sanitize_rss_html_batch
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct NapiSanitizeRssHtmlOptions {
    /// When `true`, rewrite external `<img src>` URLs through the configured proxy prefix.
    pub proxy_images: Option<bool>,
    /// URL path prefix prepended to proxied image URLs (default `/proxy/`).
    pub image_proxy_url_prefix: Option<String>,
    /// Hex-encoded HMAC-SHA256 signing keys (newest first). Empty = dev mode (no sig).
    pub image_proxy_signing_keys: Option<Vec<String>>,
}

impl NapiSanitizeRssHtmlOptions {
    fn into_sanitize_options(self) -> SanitizeRssHtmlOptions {
        SanitizeRssHtmlOptions {
            proxy_images: self.proxy_images.unwrap_or(false),
            image_proxy_url_prefix: self
                .image_proxy_url_prefix
                .unwrap_or_else(|| DEFAULT_IMAGE_PROXY_URL_PREFIX.to_string()),
            image_proxy_signing_keys: self.image_proxy_signing_keys.unwrap_or_default(),
        }
    }
}

#[napi(object)]
pub struct SanitizeRssHtmlResult {
    /// Sanitized HTML safe for rendering.
    pub html: Buffer,
    /// Raw `src` of the first external `<img>` found before proxying; `undefined` when none.
    pub first_image_src: Option<String>,
}

#[napi]
pub async fn sanitize_rss_html(
    html: Buffer,
    options: Option<NapiSanitizeRssHtmlOptions>,
) -> Result<SanitizeRssHtmlResult> {
    if html.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            html.len()
        )));
    }

    let decoded_html = String::from_utf8(html.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in HTML: {e}")))?;

    let opts = options.unwrap_or_default().into_sanitize_options();

    runtime::await_blocking(runtime::spawn_blocking(move || {
        let result = sanitize_html::sanitize_rss_html_sync(&decoded_html, &opts);
        SanitizeRssHtmlResult {
            html: Buffer::from(result.html.into_bytes()),
            first_image_src: result.first_image_src,
        }
    }))
    .await
}

#[napi]
pub async fn sanitize_rss_html_batch(
    inputs: Vec<Buffer>,
    options: Option<NapiSanitizeRssHtmlOptions>,
) -> Result<Vec<SanitizeRssHtmlResult>> {
    let total: usize = inputs.iter().map(|b| b.len()).sum();
    if total > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {total} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
        )));
    }

    let htmls = inputs
        .into_iter()
        .enumerate()
        .map(|(i, buf)| {
            String::from_utf8(buf.into())
                .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in inputs[{i}]: {e}")))
        })
        .collect::<Result<Vec<_>>>()?;

    let opts = options.unwrap_or_default().into_sanitize_options();

    runtime::await_blocking(runtime::spawn_blocking(move || {
        htmls
            .iter()
            .map(|html| {
                let result = sanitize_html::sanitize_rss_html_sync(html, &opts);
                SanitizeRssHtmlResult {
                    html: Buffer::from(result.html.into_bytes()),
                    first_image_src: result.first_image_src,
                }
            })
            .collect()
    }))
    .await
}

// ============================================================================
// extract_markdown_urls
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug)]
pub struct MarkdownUrls {
    pub link_urls: Vec<String>,
    pub image_urls: Vec<String>,
}

#[napi]
pub async fn extract_markdown_urls(text: Buffer) -> Result<MarkdownUrls> {
    if text.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            text.len()
        )));
    }
    let decoded = String::from_utf8(text.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in text: {e}")))?;
    runtime::await_blocking(runtime::spawn_blocking(move || {
        let result = markdown_to_html::extract_markdown_urls_sync(&decoded);
        MarkdownUrls {
            link_urls: result.link_urls,
            image_urls: result.image_urls,
        }
    }))
    .await
}

// ============================================================================
// render_markdown_to_html / render_markdown_to_html_batch
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct NapiMarkdownRenderOptions {
    pub allow_html: Option<bool>,
    pub nofollow_links: Option<bool>,
    pub proxy_images: Option<bool>,
    pub image_proxy_url_prefix: Option<String>,
    pub image_proxy_signing_keys: Option<Vec<String>>,
}

impl NapiMarkdownRenderOptions {
    fn into_render_options(self) -> MarkdownRenderOptions {
        MarkdownRenderOptions {
            allow_html: self.allow_html.unwrap_or(false),
            nofollow_links: self.nofollow_links.unwrap_or(true),
            proxy_images: self.proxy_images.unwrap_or(true),
            image_proxy_url_prefix: self
                .image_proxy_url_prefix
                .unwrap_or_else(|| DEFAULT_IMAGE_PROXY_URL_PREFIX.to_string()),
            image_proxy_signing_keys: self.image_proxy_signing_keys.unwrap_or_default(),
        }
    }
}

#[napi]
pub async fn render_markdown_to_html(
    text: Buffer,
    options: Option<NapiMarkdownRenderOptions>,
) -> Result<Buffer> {
    if text.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            text.len()
        )));
    }
    let decoded = String::from_utf8(text.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in text: {e}")))?;
    let opts = options.map_or_else(
        MarkdownRenderOptions::default,
        NapiMarkdownRenderOptions::into_render_options,
    );
    runtime::await_blocking(runtime::spawn_blocking(move || {
        let html = markdown_to_html::render_markdown_to_html_with_options(&decoded, &opts);
        Buffer::from(html.into_bytes())
    }))
    .await
}

#[napi]
pub async fn render_markdown_to_html_batch(
    inputs: Vec<Buffer>,
    options: Option<NapiMarkdownRenderOptions>,
) -> Result<Vec<Buffer>> {
    let total: usize = inputs.iter().map(|b| b.len()).sum();
    if total > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {total} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
        )));
    }
    let texts = inputs
        .into_iter()
        .enumerate()
        .map(|(i, buf)| {
            String::from_utf8(buf.into())
                .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in inputs[{i}]: {e}")))
        })
        .collect::<Result<Vec<_>>>()?;
    let opts = options.map_or_else(
        MarkdownRenderOptions::default,
        NapiMarkdownRenderOptions::into_render_options,
    );

    runtime::await_blocking(runtime::spawn_blocking(move || {
        texts
            .iter()
            .map(|text| {
                Buffer::from(
                    markdown_to_html::render_markdown_to_html_with_options(text, &opts)
                        .into_bytes(),
                )
            })
            .collect()
    }))
    .await
}

// ============================================================================
// html_to_embedding_text
// ============================================================================

#[napi]
pub async fn html_to_embedding_text(html: Buffer) -> Result<String> {
    if html.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            html.len()
        )));
    }
    let decoded = String::from_utf8(html.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in HTML: {e}")))?;
    runtime::await_blocking(runtime::spawn_blocking(move || {
        embedding_content::html_to_embedding_text(&decoded)
    }))
    .await
}

// ============================================================================
// detect_ai_generated_text
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug)]
pub struct SlopDetectionResult {
    pub flagged: bool,
    pub confidence_score: f64,
    pub confidence_threshold: f64,
    pub classification: String,
    pub detector: String,
    pub detector_model_version: String,
}

impl From<slop_detection::SlopDetectionResult> for SlopDetectionResult {
    fn from(result: slop_detection::SlopDetectionResult) -> Self {
        Self {
            flagged: result.flagged,
            confidence_score: f64::from(result.confidence_score),
            confidence_threshold: f64::from(result.confidence_threshold),
            classification: match result.classification {
                slop_detection::SlopClassification::Ai => "ai".to_string(),
                slop_detection::SlopClassification::Human => "human".to_string(),
            },
            detector: result.detector.to_string(),
            detector_model_version: result.detector_model_version.to_string(),
        }
    }
}

#[napi]
pub async fn detect_ai_generated_text(
    text: Buffer,
    confidence_threshold: Option<f64>,
) -> Result<SlopDetectionResult> {
    if text.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            text.len()
        )));
    }

    let decoded = String::from_utf8(text.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in text: {e}")))?;
    #[allow(clippy::cast_possible_truncation)] // 0.0..=1.0 fits losslessly in f32
    let threshold = confidence_threshold.unwrap_or(0.95) as f32;

    runtime::await_blocking_result(runtime::spawn_blocking(move || {
        slop_detection::detect_ai_generated_text(&decoded, threshold)
            .map(std::convert::Into::into)
            .map_err(Error::from_reason)
    }))
    .await
}

// ============================================================================
// chunk
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug)]
pub struct Chunk {
    pub level: u32,
    pub header: Option<String>,
    pub headers: Vec<Option<String>>,
    pub breadcrumb: String,
    pub text: String,
    pub length: u32,
}

impl From<InternalChunk> for Chunk {
    fn from(chunk: InternalChunk) -> Self {
        Chunk {
            level: chunk.level,
            header: chunk.header,
            headers: chunk.headers,
            breadcrumb: chunk.breadcrumb,
            text: chunk.text,
            // Safe: input is bounded by SANITIZE_MAX_INPUT_BYTES (10 MiB ≈ 10M chars),
            // which is well below u32::MAX (≈4.3B).
            length: chunk.length as u32,
        }
    }
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct ChunkOptions {
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub phase: Option<u32>,
    pub title: Option<String>,
}

impl From<ChunkOptions> for InternalChunkOptions {
    fn from(opts: ChunkOptions) -> Self {
        InternalChunkOptions {
            min_length: opts.min_length,
            max_length: opts.max_length,
            phase: opts.phase,
            title: opts.title,
        }
    }
}

#[napi]
pub async fn chunk(text: Buffer, options: Option<ChunkOptions>) -> Result<Vec<Chunk>> {
    if text.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            text.len()
        )));
    }
    let decoded = String::from_utf8(text.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in text: {e}")))?;

    runtime::await_blocking(runtime::spawn_blocking(move || {
        let internal_options = options.map(std::convert::Into::into);
        core_chunk(&decoded, internal_options)
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Chunk>>()
    }))
    .await
}

// ============================================================================
// extract_dom_removals / apply_dom_removals_to_html
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug)]
pub struct ExtractDomRemovalsOptions {
    pub boilerplate_patterns: Option<Vec<String>>,
}

impl From<ExtractDomRemovalsOptions> for LearnOptions {
    fn from(opts: ExtractDomRemovalsOptions) -> Self {
        LearnOptions {
            boilerplate_patterns: opts.boilerplate_patterns,
            ..LearnOptions::default()
        }
    }
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct ExtractDomRemovalsResult {
    pub css_selectors_to_remove: Vec<String>,
    pub html_to_remove: Vec<String>,
}

impl From<Removals> for ExtractDomRemovalsResult {
    fn from(result: Removals) -> Self {
        ExtractDomRemovalsResult {
            css_selectors_to_remove: result.css_selectors_to_remove,
            html_to_remove: result.html_to_remove,
        }
    }
}

impl From<ExtractDomRemovalsResult> for Removals {
    fn from(result: ExtractDomRemovalsResult) -> Self {
        Removals {
            css_selectors_to_remove: result.css_selectors_to_remove,
            html_to_remove: result.html_to_remove,
        }
    }
}

#[napi]
pub async fn extract_dom_removals(
    html_pages: Vec<Buffer>,
    options: Option<ExtractDomRemovalsOptions>,
) -> Result<ExtractDomRemovalsResult> {
    let total: usize = html_pages.iter().map(|b| b.len()).sum();
    if total > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {total} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
        )));
    }
    let decoded_pages = html_pages
        .into_iter()
        .enumerate()
        .map(|(index, html_page)| {
            String::from_utf8(html_page.to_vec()).map_err(|e| {
                Error::from_reason(format!(
                    "Invalid UTF-8 in html_pages[{index}]. Expected UTF-8 encoded HTML: {e}"
                ))
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let internal_options = options.map(std::convert::Into::into).unwrap_or_default();

    runtime::await_blocking_result(runtime::spawn_blocking(move || {
        learn(&decoded_pages, &internal_options)
            .map_err(|e| Error::from_reason(e.to_string()))
            .map(std::convert::Into::into)
    }))
    .await
}

#[napi]
pub async fn apply_dom_removals_to_html(
    html: Buffer,
    removals: ExtractDomRemovalsResult,
) -> Result<Buffer> {
    if html.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            html.len()
        )));
    }
    let decoded_html = String::from_utf8(html.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in HTML: {e}")))?;
    let internal_removals: Removals = removals.into();

    runtime::await_blocking(runtime::spawn_blocking(move || {
        let cleaned = apply_removals(&decoded_html, &internal_removals);
        Buffer::from(cleaned.into_bytes())
    }))
    .await
}

// ============================================================================
// get_content_from_html
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug)]
pub struct CrawlerHtmlToMarkdownOptions {
    pub css_selectors_to_remove: Option<Vec<String>>,
    pub content_selectors: Option<Vec<String>>,
    pub link_text_content_to_remove: Option<Vec<String>>,
    pub link_hrefs_to_remove: Option<Vec<String>>,
    pub link_rel_tokens_to_remove: Option<Vec<String>>,
    pub use_text_density_filter: Option<bool>,
}

impl From<CrawlerHtmlToMarkdownOptions> for ConvertOptions {
    fn from(opts: CrawlerHtmlToMarkdownOptions) -> Self {
        ConvertOptions {
            css_selectors_to_remove: opts.css_selectors_to_remove.unwrap_or_default(),
            content_selectors: opts.content_selectors.unwrap_or_default(),
            link_text_content_to_remove: opts.link_text_content_to_remove.unwrap_or_default(),
            link_hrefs_to_remove: opts.link_hrefs_to_remove.unwrap_or_default(),
            link_rel_tokens_to_remove: opts.link_rel_tokens_to_remove.unwrap_or_default(),
            use_text_density_filter: opts.use_text_density_filter.unwrap_or_default(),
            removals: None,
        }
    }
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct CrawlerHtmlToMarkdownResult {
    pub title: Option<String>,
    pub meta: serde_json::Map<String, serde_json::Value>,
    pub links: serde_json::Map<String, serde_json::Value>,
    pub content: String,
    pub canonical_url: Option<String>,
    pub lang: Option<String>,
}

impl From<boilerstrip::ConvertResult> for CrawlerHtmlToMarkdownResult {
    fn from(result: boilerstrip::ConvertResult) -> Self {
        CrawlerHtmlToMarkdownResult {
            title: result.title,
            meta: result.meta,
            links: result.link,
            content: result.content,
            canonical_url: result.canonical_url,
            lang: result.lang,
        }
    }
}

#[napi]
pub async fn get_content_from_html(
    html_buffer: Buffer,
    options: CrawlerHtmlToMarkdownOptions,
) -> Result<CrawlerHtmlToMarkdownResult> {
    if html_buffer.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            html_buffer.len()
        )));
    }
    let html = String::from_utf8(html_buffer.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in HTML: {e}")))?;

    let internal_options: ConvertOptions = options.into();

    runtime::await_blocking(runtime::spawn_blocking(move || {
        convert(&html, &internal_options).into()
    }))
    .await
}

// ============================================================================
// sanitize_prompt_injection
// ============================================================================

#[napi(js_name = "sanitizePromptInjection")]
pub async fn sanitize_prompt_injection_napi(
    content: Buffer,
    is_title: Option<bool>,
) -> Result<Buffer> {
    if content.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            content.len()
        )));
    }

    let decoded = String::from_utf8(content.to_vec())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in content: {e}")))?;

    runtime::await_blocking(runtime::spawn_blocking(move || {
        let sanitized = sanitize_prompt_injection::sanitize_prompt_injection_sync(
            &decoded,
            is_title.unwrap_or(false),
        );
        Buffer::from(sanitized.into_bytes())
    }))
    .await
}
