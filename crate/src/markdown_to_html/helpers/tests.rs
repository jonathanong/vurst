use super::*;

#[test]
fn scheme_and_url_helpers_cover_invalid_paths() {
    assert!(extract_scheme("noscheme").is_none());
    assert!(extract_scheme(":bad").is_none());
    assert!(extract_scheme("bad space:").is_none());
    assert!(is_safe_link_url("/relative"));
    assert!(!is_safe_link_url("1bad:"));
    assert!(is_safe_image_url("/relative.png"));
    assert!(!is_safe_image_url("1bad:"));
    assert!(!is_safe_image_url("mailto:test@example.com"));
}
