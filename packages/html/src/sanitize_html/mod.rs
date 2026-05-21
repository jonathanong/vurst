//! HTML sanitization for RSS feed items.
//!
//! Applies universal, fixed sanitization rules to clean raw HTML from RSS feeds
//! into safe HTML suitable for web rendering.

pub mod sanitize;

pub use sanitize::{sanitize_rss_html_sync, SanitizeRssHtmlOptions, SanitizeRssHtmlResult};

#[cfg(test)]
mod tests;
