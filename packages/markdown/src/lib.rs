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

use napi::bindgen_prelude::*;
use napi_derive::napi;

use vurst_runtime_rs as runtime;

pub mod image_proxy {
    pub use vurst_shared::image_proxy::*;
}
pub mod markdown_to_html;

use breadchunks::chunk as breadchunks_chunk;
pub use markdown_to_html::{
    extract_markdown_urls_sync, render_markdown_to_html_with_options, MarkdownRenderOptions,
    MarkdownUrlsResult,
};

pub use breadchunks::{default_length_counter, Chunk, ChunkOptions};
pub use image_proxy::DEFAULT_IMAGE_PROXY_URL_PREFIX;

const ZERO_WIDTH_SPACE: char = '\u{200B}';

pub fn chunk(text: &str, options: Option<ChunkOptions>) -> Vec<Chunk> {
    let mut chunks = breadchunks_chunk(text, options.as_ref());

    if chunks.is_empty() && !text.trim().is_empty() {
        let dummy_text = format!("{text}\n\n{ZERO_WIDTH_SPACE}");
        chunks = breadchunks_chunk(&dummy_text, options.as_ref());
        if let Some(last_chunk) = chunks.last_mut() {
            if last_chunk.text.ends_with(ZERO_WIDTH_SPACE) {
                let _ = last_chunk.text.pop();
                let full_text = if last_chunk.breadcrumb.is_empty() {
                    last_chunk.text.clone()
                } else {
                    format!("{}\n\n{}", last_chunk.breadcrumb, last_chunk.text)
                };
                last_chunk.length = default_length_counter(&full_text);
            }
        }
    }

    chunks
}

/// Maximum input size for all functions (10 MiB). Batch functions check the
/// total bytes across all inputs. This bounds blocking-pool exposure to large
/// or adversarial inputs.
const SANITIZE_MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;

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
    let decoded = String::from_utf8(text.into())
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
    let decoded = String::from_utf8(text.into())
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
// chunk
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug)]
pub struct NapiChunk {
    pub level: u32,
    pub header: Option<String>,
    pub headers: Vec<Option<String>>,
    pub breadcrumb: String,
    pub text: String,
    pub length: u32,
}

impl From<Chunk> for NapiChunk {
    fn from(c: Chunk) -> Self {
        NapiChunk {
            level: c.level,
            header: c.header.as_ref().map(ToString::to_string),
            headers: c
                .headers
                .iter().map(|s| s.as_ref().map(ToString::to_string))
                .collect(),
            breadcrumb: c.breadcrumb.to_string(),
            text: c.text,
            // Safe: input is bounded by SANITIZE_MAX_INPUT_BYTES (10 MiB ≈ 10M chars),
            // which is well below u32::MAX (≈4.3B).
            length: c.length as u32,
        }
    }
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct NapiChunkOptions {
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub phase: Option<u32>,
    pub title: Option<String>,
}

impl From<NapiChunkOptions> for ChunkOptions {
    fn from(opts: NapiChunkOptions) -> Self {
        ChunkOptions {
            min_length: opts.min_length,
            max_length: opts.max_length,
            phase: opts.phase,
            title: opts.title,
        }
    }
}

#[napi(js_name = "chunk")]
pub async fn chunk_napi(text: Buffer, options: Option<NapiChunkOptions>) -> Result<Vec<NapiChunk>> {
    if text.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            text.len()
        )));
    }
    let decoded = String::from_utf8(text.into())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in text: {e}")))?;

    runtime::await_blocking(runtime::spawn_blocking(move || {
        let internal_options = options.map(std::convert::Into::into);
        chunk(&decoded, internal_options)
            .into_iter()
            .map(Into::into)
            .collect::<Vec<NapiChunk>>()
    }))
    .await
}
