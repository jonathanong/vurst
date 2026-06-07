use super::*;

#[test]
fn scheme_and_url_helpers_cover_invalid_paths() {
    assert!(is_safe_link_url("https://example.com"));
    assert!(is_safe_link_url("mailto:test@example.com"));
    assert!(is_safe_link_url("tel:+123456789"));
    assert!(is_safe_link_url("/relative"));
    assert!(is_safe_link_url("/login?url=http://example.com"));
    assert!(is_safe_link_url("./path:with:colons"));
    assert!(is_safe_link_url("?url=http://example.com"));
    assert!(is_safe_link_url("#redirect:https://example.com"));
    assert!(!is_safe_link_url(":bad"));
    assert!(!is_safe_link_url("bad space:"));
    assert!(!is_safe_link_url("1bad:"));
    assert!(!is_safe_link_url("javascript:alert(1)"));
    assert!(!is_safe_link_url("javascript&#58;alert(1)"));
    assert!(!is_safe_link_url("javascript&#0000058;alert(1)"));
    assert!(!is_safe_link_url("javascript&colon;alert(1)"));
    assert!(!is_safe_link_url("\x0Bjavascript:alert(1)"));
    assert!(!is_safe_link_url("\x01javascript:alert(1)"));
    assert!(!is_safe_link_url("java\x09script:alert(1)"));
    assert!(!is_safe_link_url("\x01//evil.com"));
    assert!(!is_safe_link_url("\\\\evil.com"));
    assert!(!is_safe_link_url("/\\evil.com"));
    assert!(!is_safe_link_url("\\/evil.com"));
    assert!(!is_safe_link_url("\x00"));
    assert!(!is_safe_link_url("path:with:colons"));
    assert!(is_safe_image_url("https://example.com/image.png"));
    assert!(is_safe_image_url("/relative.png"));
    assert!(is_safe_image_url("./image:variant.png"));
    assert!(!is_safe_image_url("1bad:"));
    assert!(!is_safe_image_url("mailto:test@example.com"));
}

#[test]
fn fast_path_security_and_slow_path_no_scheme() {
    // Fast path: protocol-relative `//` (no control chars → no allocation)
    assert!(!is_safe_link_url("//evil.com"));
    // Fast path: single backslash before a non-slash/backslash char
    assert!(!is_safe_link_url("\\evil.com"));
    // Slow path: control char stripped, leaving a schemeless relative URL →
    // scheme_candidate returns None → return true (covers the uncovered branch)
    assert!(is_safe_link_url("\x09/relative"));
    // Slow path: control char stripped, leaving a backslash-prefixed URL
    assert!(!is_safe_link_url("\x09\\evil.com"));
}
