//! # `vurst` - High-performance text processing utilities
//!
//! A Rust library for semantic chunking of text, sanitizing content, and
//! embedding-content preparation. Designed for CPU-intensive web crawling and
//! content processing workflows.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::needless_continue)]
#![allow(clippy::format_push_string)]
#![allow(clippy::must_use_candidate)]

pub mod embedding_content;
pub mod image_proxy;
pub mod markdown_to_html;
pub mod sanitize_html;
pub mod sanitize_prompt_injection;
pub mod slop_detection;

pub use breadchunks::{chunk, default_length_counter, Chunk, ChunkOptions};
pub use embedding_content::html_to_embedding_text;
pub use markdown_to_html::{
    extract_markdown_urls_sync, render_markdown_to_html_with_options, MarkdownRenderOptions,
    MarkdownUrlsResult,
};
pub use sanitize_html::{sanitize_rss_html_sync, SanitizeRssHtmlOptions, SanitizeRssHtmlResult};
pub use sanitize_prompt_injection::sanitize_prompt_injection_sync;
pub use slop_detection::{detect_ai_generated_text, SlopClassification, SlopDetectionResult};

/// Serialize a parsed HTML fragment's body without `<html>` wrapper tags.
/// `Html::parse_fragment` wraps content in `<html>...</html>` — this strips those wrappers.
pub(crate) fn serialize_fragment_body(fragment: &scraper::Html) -> String {
    let full_html = fragment.html();
    let stripped = full_html.strip_prefix("<html>").unwrap_or(&full_html);
    stripped
        .strip_suffix("</html>")
        .unwrap_or(stripped)
        .to_string()
}
