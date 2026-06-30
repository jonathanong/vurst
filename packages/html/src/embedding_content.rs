//! Utilities for preparing HTML content for embedding models.
//!
//! Converts HTML to clean text by stripping non-semantic markup (link URLs,
//! image URLs) while preserving text content and alt text. This produces
//! compact, semantically rich text suitable as input to vector embedding APIs.

use boilerstrip::{convert, ConvertOptions};
use regex::Regex;
use std::borrow::Cow;
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
    let markdown = convert(html, &ConvertOptions::default()).content;

    // ⚡ Bolt: Fast-path skip markdown regexes for strings lacking bracket syntax
    if !markdown.contains('[') {
        return markdown.trim().to_string();
    }

    let mut current = Cow::Borrowed(markdown.as_str());

    if current.contains("![") {
        if let Cow::Owned(replaced) = IMAGE_REGEX.replace_all(&current, "$1") {
            current = Cow::Owned(replaced);
        }
    }

    if current.contains("](") {
        if let Cow::Owned(replaced) = LINK_REGEX.replace_all(&current, "$1") {
            current = Cow::Owned(replaced);
        }
    }

    if current.contains("]:") {
        if let Cow::Owned(replaced) = REF_LINK_DEF_REGEX.replace_all(&current, "") {
            current = Cow::Owned(replaced);
        }
    }

    current.trim().to_string()
}
