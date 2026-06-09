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

#[test]
fn test_is_safe_image_url_exhaustive() {
    // Allowed schemes
    assert!(is_safe_image_url("http://example.com/image.png"));
    assert!(is_safe_image_url("https://example.com/image.png"));
    assert!(is_safe_image_url("HTTPS://example.com/image.png"));
    assert!(is_safe_image_url("https://example.com:8080/image.png"));

    // Disallowed schemes (especially link-only schemes)
    assert!(!is_safe_image_url("mailto:test@example.com"));
    assert!(!is_safe_image_url("tel:+123456789"));
    assert!(!is_safe_image_url("javascript:alert(1)"));
    assert!(!is_safe_image_url("data:image/png;base64,iVBORw0KGgo"));
    assert!(!is_safe_image_url("ftp://example.com/image.png"));
    assert!(!is_safe_image_url("file:///etc/passwd"));
    assert!(!is_safe_image_url("vbscript:msgbox(\"x\")"));
    assert!(!is_safe_image_url("1http://example.com/image.png"));

    // Valid relative URLs
    assert!(is_safe_image_url("/path.png"));
    assert!(is_safe_image_url("./path.png"));
    assert!(is_safe_image_url("../path.png"));
    assert!(is_safe_image_url("image.png"));
    assert!(is_safe_image_url("image.png?query=1"));
    assert!(is_safe_image_url("image.png#fragment"));
    assert!(is_safe_image_url("/login?url=http://example.com"));
    assert!(is_safe_image_url("?url=http://example.com"));
    assert!(is_safe_image_url("./path:to:image.png"));

    // Malicious protocol-relative URLs
    assert!(!is_safe_image_url("//evil.com/image.png"));
    assert!(!is_safe_image_url("/\\evil.com/image.png"));
    assert!(!is_safe_image_url("\\/evil.com/image.png"));
    assert!(!is_safe_image_url("\\\\evil.com/image.png"));
    assert!(!is_safe_image_url("\\evil.com/image.png"));

    // Malicious URLs obfuscated with whitespace/control characters (slow path)
    assert!(!is_safe_image_url("java\x09script:alert(1)"));
    assert!(!is_safe_image_url("\x0Bjavascript:alert(1)"));
    assert!(!is_safe_image_url("\x01javascript:alert(1)"));
    assert!(!is_safe_image_url("\x09//evil.com"));
    assert!(!is_safe_image_url("\x09\\evil.com"));

    // Empty URL
    assert!(!is_safe_image_url(""));
}

#[test]
fn decode_url_html_entities_covers_borrowed_invalid_and_multiple_refs() {
    assert_eq!(
        decode_url_html_entities("https://example.com").as_ref(),
        "https://example.com"
    );
    assert_eq!(decode_url_html_entities("a&#oops").as_ref(), "a&#oops");
    assert_eq!(
        decode_url_html_entities("java&#115cript&#58alert(1)").as_ref(),
        "javascript:alert(1)"
    );
}

#[test]
fn decode_numeric_char_ref_covers_decimal_hex_and_invalid_refs() {
    assert_eq!(decode_numeric_char_ref("&#58alert"), Some((':', 4)));
    assert_eq!(decode_numeric_char_ref("&#58;alert"), Some((':', 5)));
    assert_eq!(decode_numeric_char_ref("&#x3cscript"), Some(('<', 5)));
    assert_eq!(decode_numeric_char_ref("plain"), None);
    assert_eq!(decode_numeric_char_ref("&#x;"), None);
    assert_eq!(decode_numeric_char_ref("&#99999999;"), None);
}

#[test]
fn extract_bare_domains_skips_email_at_prefix() {
    // The '@' guard inside extract_bare_domains is unreachable via
    // extract_markdown_urls_sync because comrak GFM-autolinks emails into
    // Link nodes before the text walk.  Test it directly.
    let mut links = Vec::new();
    extract_bare_domains("user@example.com", &mut links);
    assert!(
        links.is_empty(),
        "email address should not produce a bare-domain entry: {links:?}"
    );
}

#[test]
fn extract_bare_domains_skips_bare_psl_suffix() {
    // "co.uk" is itself a public suffix (no registrable domain label before
    // it), so psl::domain() returns None — exercises that else-continue branch.
    let mut links = Vec::new();
    extract_bare_domains("visit co.uk today", &mut links);
    assert!(
        links.is_empty(),
        "bare PSL suffix should not produce a link: {links:?}"
    );
}
