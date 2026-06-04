use super::*;

const PREFIX: &str = DEFAULT_IMAGE_PROXY_URL_PREFIX;

#[test]
fn test_is_external_http_url() {
    assert!(is_external_http_url("http://example.com/img.jpg"));
    assert!(is_external_http_url("https://example.com/img.jpg"));
    assert!(is_external_http_url("HTTP://example.com/img.jpg"));
    assert!(!is_external_http_url("/relative/path.jpg"));
    assert!(!is_external_http_url("//protocol-relative.com/img.jpg"));
    assert!(!is_external_http_url("data:image/png;base64,abc"));
    assert!(!is_external_http_url(""));
}

#[test]
fn test_is_relative_url() {
    assert!(is_relative_url("/relative/path.jpg"));
    assert!(is_relative_url("../relative.jpg"));
    assert!(!is_relative_url("http://example.com/img.jpg"));
    assert!(!is_relative_url("//protocol-relative.com/img.jpg"));
    assert!(!is_relative_url("\\\\protocol-relative.com/img.jpg"));
    assert!(!is_relative_url("/\\protocol-relative.com/img.jpg"));
    assert!(!is_relative_url("\\/protocol-relative.com/img.jpg"));
    assert!(!is_relative_url("\\attacker.com"));
    assert!(!is_relative_url(""));
}

#[test]
fn test_is_proxy_url() {
    assert!(is_proxy_url("/proxy/abc123", PREFIX));
    assert!(!is_proxy_url("https://example.com/img.jpg", PREFIX));
    assert!(!is_proxy_url("/images/abc.jpg", PREFIX));
    // Empty prefix never matches.
    assert!(!is_proxy_url("/proxy/abc", ""));
    // Custom prefix.
    assert!(is_proxy_url("/img-proxy/abc", "/img-proxy/"));
}

#[test]
fn test_should_proxy_image() {
    assert!(should_proxy_image("https://example.com/img.jpg", PREFIX));
    assert!(should_proxy_image("http://example.com/img.jpg", PREFIX));
    assert!(!should_proxy_image("/relative.jpg", PREFIX));
    assert!(!should_proxy_image("/proxy/abc123", PREFIX));
    assert!(!should_proxy_image("data:image/png;base64,abc", PREFIX));
    assert!(!should_proxy_image("", PREFIX));
}

#[test]
fn test_rewrite_image_to_proxy_no_keys() {
    let result = rewrite_image_to_proxy("https://example.com/img.jpg", PREFIX, &[]);
    assert!(result.starts_with(PREFIX));
    assert!(!result.contains("sig="));
    assert!(!result.contains('?'));
}

#[test]
fn test_rewrite_image_to_proxy_with_key() {
    let key = "deadbeef".repeat(8); // 64 hex chars = 32 bytes
    let result = rewrite_image_to_proxy("https://example.com/img.jpg", PREFIX, &[key]);
    assert!(result.starts_with(PREFIX));
    assert!(result.contains("?sig="));
}

#[test]
fn test_rewrite_image_is_base64url() {
    let url = "https://example.com/image with spaces.jpg";
    let result = rewrite_image_to_proxy(url, PREFIX, &[]);
    let encoded_part = result.trim_start_matches(PREFIX).split('?').next().unwrap();
    // base64url characters only (no +, /, or =)
    assert!(encoded_part
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
}

#[test]
fn test_rewrite_with_custom_prefix() {
    let result = rewrite_image_to_proxy("https://example.com/img.jpg", "/i/", &[]);
    assert!(result.starts_with("/i/"));
}
