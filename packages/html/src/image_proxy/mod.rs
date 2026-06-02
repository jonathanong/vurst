//! Image-proxy URL rewriting and HMAC signing utilities shared between the
//! markdown renderer and RSS sanitizer.
//!
//! External image URLs are rewritten to a host-defined path prefix with the
//! original URL base64url-encoded as the trailing path segment. The proxy
//! endpoint decodes the path segment to recover the original URL.

use data_encoding::BASE64URL_NOPAD;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Default URL path prefix prepended to proxied image URLs.
pub const DEFAULT_IMAGE_PROXY_URL_PREFIX: &str = "/proxy/";

/// Returns true if the URL scheme is http or https.
pub fn is_external_http_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Returns true if the URL is relative (no scheme, not protocol-relative).
pub fn is_relative_url(url: &str) -> bool {
    let url = url.trim();
    if url.is_empty()
        || (url.len() >= 2 && matches!(url.as_bytes()[..2], [b'/' | b'\\', b'/' | b'\\']))
    {
        return false;
    }
    !url.contains(':')
}

/// Returns true if the URL is already an image-proxy path with the given prefix.
pub fn is_proxy_url(url: &str, prefix: &str) -> bool {
    !prefix.is_empty() && url.starts_with(prefix)
}

/// Returns true if this image URL should be proxied through the given prefix.
pub fn should_proxy_image(url: &str, prefix: &str) -> bool {
    let url = url.trim();
    !url.is_empty()
        && !is_relative_url(url)
        && !is_proxy_url(url, prefix)
        && is_external_http_url(url)
}

/// Rewrite an external image URL to a proxy path with the given prefix.
///
/// Returns `{prefix}{base64url}?sig={hmac}` when a provided key can be decoded
/// and used for signing, or `{prefix}{base64url}` in dev mode (no keys) or
/// when no provided key is valid hex / usable for HMAC initialization
/// (defensive fallback — misconfigured signing keys should not crash the
/// process; the URL builder simply falls back to an unsigned path).
///
/// # Panics
///
/// Panics only if HMAC-SHA256 rejects a decoded key, which the HMAC
/// implementation documents as impossible for this algorithm.
pub fn rewrite_image_to_proxy(url: &str, prefix: &str, signing_keys: &[String]) -> String {
    let trimmed = url.trim();
    let encoded = BASE64URL_NOPAD.encode(trimmed.as_bytes());
    let path = format!("{prefix}{encoded}");
    for key_hex in signing_keys {
        if let Ok(key_bytes) = hex::decode(key_hex) {
            let mut mac = HmacSha256::new_from_slice(&key_bytes)
                .expect("BUG: HMAC-SHA256 accepts keys of any length");
            mac.update(path.as_bytes());
            let sig = hex::encode(mac.finalize().into_bytes());
            return format!("{path}?sig={sig}");
        }
    }
    path
}

#[cfg(test)]
mod tests;
