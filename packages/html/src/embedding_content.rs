//! Utilities for preparing HTML content for embedding models.
//!
//! Converts HTML to clean text by stripping non-semantic markup (link URLs,
//! image URLs) while preserving text content and alt text. This produces
//! compact, semantically rich text suitable as input to vector embedding APIs.

use boilerstrip::{convert, ConvertOptions};
use regex::Regex;
use std::sync::LazyLock;

/// Matches inline markdown images: `![alt](url)` — capture group 1 is alt text.
/// Must be applied before `LINK_REGEX` to prevent the link pattern from
/// consuming the alt-text brackets inside an image.
static IMAGE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"!\[([^\]]*)\]\([^)]*\)").expect("BUG: invalid IMAGE_REGEX"));

/// Matches inline markdown links: `[text](url)` — capture group 1 is link text.
static LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\([^)]*\)").expect("BUG: invalid LINK_REGEX"));

/// Matches reference-style link definitions at the start of a line:
/// `[label]: url` or `[label]: url "title"`.
static REF_LINK_DEF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\[[^\]]+\]:[^\n]*\n?").expect("BUG: invalid REF_LINK_DEF_REGEX")
});

/// Convert HTML to clean text suitable for vector embedding.
///
/// Pipeline:
/// 1. Convert HTML to markdown via `boilerstrip::convert`
/// 2. Replace `![alt](url)` with `alt` (keep image alt text, drop URL)
/// 3. Replace `[text](url)` with `text` (keep link text, drop URL)
/// 4. Remove reference-style link definitions `[ref]: url`
///
/// URLs are noise for semantic similarity — stripping them reduces token
/// usage and improves embedding quality.
pub fn html_to_embedding_text(html: &str) -> String {
    let mut current = convert(html, &ConvertOptions::default()).content;

    // Fast-path: avoid expensive regex if structural characters aren't present
    if current.contains("![") {
        // Images must be replaced before links to prevent the link pattern
        // from consuming the alt-text brackets in `![alt](url)`.
        current = IMAGE_REGEX.replace_all(&current, "$1").into_owned();
    }

    if current.contains("](") {
        current = LINK_REGEX.replace_all(&current, "$1").into_owned();
    }

    if current.contains("]:") {
        current = REF_LINK_DEF_REGEX.replace_all(&current, "").into_owned();
    }

    current.trim().to_string()
}
