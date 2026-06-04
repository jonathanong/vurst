use super::{sanitize_rss_html_sync, SanitizeRssHtmlOptions, SanitizeRssHtmlResult};

fn sanitize(html: &str) -> SanitizeRssHtmlResult {
    sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default())
}

fn sanitize_with_proxy(html: &str, keys: Vec<String>) -> SanitizeRssHtmlResult {
    sanitize_rss_html_sync(
        html,
        &SanitizeRssHtmlOptions {
            proxy_images: true,
            image_proxy_signing_keys: keys,
            ..SanitizeRssHtmlOptions::default()
        },
    )
}

#[test]
fn test_empty_input() {
    let result = sanitize("");
    assert_eq!(result.html, "");
    assert!(result.first_image_src.is_none());
}

#[test]
fn test_first_image_src_captured() {
    let html = r#"<p>Text</p><img src="https://example.com/photo.jpg" alt="photo">"#;
    let result = sanitize(html);
    assert_eq!(
        result.first_image_src.as_deref(),
        Some("https://example.com/photo.jpg")
    );
}

#[test]
fn test_first_image_src_skips_relative() {
    let html = r#"<img src="/relative.jpg"><img src="https://example.com/photo.jpg">"#;
    let result = sanitize(html);
    assert_eq!(
        result.first_image_src.as_deref(),
        Some("https://example.com/photo.jpg")
    );
}

#[test]
fn test_first_image_src_none_when_no_external_img() {
    let html = r"<p>No images</p>";
    let result = sanitize(html);
    assert!(result.first_image_src.is_none());
}

#[test]
fn test_proxy_off_preserves_src() {
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize(html);
    assert!(result.html.contains("https://example.com/photo.jpg"));
}

#[test]
fn test_proxy_on_rewrites_src() {
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize_with_proxy(html, vec![]);
    assert!(!result.html.contains("https://example.com"));
    assert!(result.html.contains("/proxy/"));
}

#[test]
fn test_proxy_on_with_key_adds_sig() {
    let key = "deadbeef".repeat(8); // 64 hex chars = 32 bytes
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize_with_proxy(html, vec![key]);
    assert!(result.html.contains("sig="));
}

#[test]
fn test_proxy_skips_relative_src() {
    let html = r#"<img src="/local/image.jpg">"#;
    let result = sanitize_with_proxy(html, vec![]);
    assert!(result.html.contains("/local/image.jpg"));
    assert!(!result.html.contains("/proxy/"));
}

#[test]
fn test_proxy_skips_already_proxied() {
    let html = r#"<img src="/proxy/abc123">"#;
    let result = sanitize_with_proxy(html, vec![]);
    // Should NOT be double-proxied
    assert!(!result.html.contains("/proxy//proxy/"));
    assert_eq!(result.html.matches("/proxy/").count(), 1);
}

#[test]
fn test_first_image_src_is_original_before_proxy() {
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize_with_proxy(html, vec![]);
    // first_image_src should be the original URL, not the proxied one
    assert_eq!(
        result.first_image_src.as_deref(),
        Some("https://example.com/photo.jpg")
    );
    // but the rendered HTML should have the proxied src
    assert!(result.html.contains("/proxy/"));
}

#[test]
fn test_dangerous_elements_removed() {
    let html = r"<p>Hello</p><script>alert(1)</script>";
    let result = sanitize(html);
    assert!(!result.html.contains("<script>"));
    assert!(result.html.contains("<p>Hello</p>"));
}

#[test]
fn test_lazy_loading_added() {
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize(html);
    assert!(result.html.contains("loading=\"lazy\""));
    assert!(result.html.contains("fetchpriority=\"low\""));
    assert!(result.html.contains("decoding=\"async\""));
}

#[test]
fn test_proxy_on_handles_img_without_src() {
    let result = sanitize_with_proxy("<img alt=\"empty\">", vec![]);
    assert!(result.html.contains("alt=\"empty\""));
    assert!(!result.html.contains("/proxy/"));
}

#[test]
fn sanitizes_links_and_detaches_empty_containers() {
    let result = sanitize(
        r#"<section><a href="/safe" rel="old" target="_self">Safe</a></section><div><span> </span></div>"#,
    );

    assert!(result.html.contains(r#"rel="nofollow noopener""#));
    assert!(result.html.contains(r#"target="_blank""#));
    assert!(!result.html.contains("<div>"));
    assert!(!result.html.contains("<span>"));
}

#[test]
fn empty_container_preflight_skips_non_empty_fragments() {
    assert!(!super::sanitize::may_have_empty_container(
        "<p>Hello</p><img src=\"photo.jpg\">"
    ));
    assert!(!super::sanitize::may_have_empty_container("<div"));
    assert!(!super::sanitize::may_have_empty_container("<div>&#;</div>"));
    // Non-ASCII non-whitespace content: the leading U+00E9 byte (0xC3) is not
    // ASCII, so the slow chars().next() path is taken; since é is not whitespace,
    // the !ch.is_whitespace() branch breaks immediately.
    assert!(!super::sanitize::may_have_empty_container(
        "<p>\u{00e9}content</p>"
    ));
    // Non-ASCII whitespace followed by non-ASCII non-whitespace: exercises the
    // slow chars() loop iteration across both the continue and the break arm.
    assert!(!super::sanitize::may_have_empty_container(
        "<div>\u{2003}\u{00e9}</div>"
    ));
}

#[test]
fn empty_container_preflight_detects_cleanup_candidates() {
    assert!(super::sanitize::may_have_empty_container("<div></div>"));
    assert!(super::sanitize::may_have_empty_container("<span> </span>"));
    assert!(super::sanitize::may_have_empty_container(
        "<section>\n</section>"
    ));
    assert!(super::sanitize::may_have_empty_container("<p>\t</p>"));
    assert!(super::sanitize::may_have_empty_container(
        "<article data-origin=\"rss\"> \n\t</article>"
    ));
    assert!(super::sanitize::may_have_empty_container(
        "<div data-note=\">\"> </div>"
    ));
    assert!(super::sanitize::may_have_empty_container(
        "<span title='>'> </span>"
    ));
    assert!(super::sanitize::may_have_empty_container(
        "<figure>\n \t </figure>"
    ));
    assert!(super::sanitize::may_have_empty_container("<DIV></div>"));
    assert!(super::sanitize::may_have_empty_container(
        "<div>\u{00a0}</div>"
    ));
    assert!(super::sanitize::may_have_empty_container(
        "<div>\u{2003}</div>"
    ));
    assert!(super::sanitize::may_have_empty_container(
        "<section>&nbsp;</section>"
    ));
    assert!(super::sanitize::may_have_empty_container(
        "<article>&#8195;</article>"
    ));
    assert!(super::sanitize::may_have_empty_container("<p>&#x2003;</p>"));
    // Vertical tab (0x0B): ASCII byte, but char::is_whitespace treats it as
    // whitespace while u8::is_ascii_whitespace does not.  The fast ASCII path
    // uses (b as char).is_whitespace() to preserve this parity.
    assert!(super::sanitize::may_have_empty_container("<div>\x0b</div>"));
    // Long ASCII whitespace run exercises multiple iterations of the fast-path.
    assert!(super::sanitize::may_have_empty_container(
        "<div>          \n\n\t\t\r\n</div>"
    ));
    // Decimal HTML entity for space (U+0020, codepoint 32): confirms the
    // numeric-reference branch handles whitespace codepoints correctly.
    assert!(super::sanitize::may_have_empty_container(
        "<section>&#32;</section>"
    ));
    // Mixed non-ASCII whitespace and ASCII whitespace: toggles between the slow
    // chars() path and the fast ASCII path within a single call.
    assert!(super::sanitize::may_have_empty_container(
        "<div>\u{2003} \u{00a0}</div>"
    ));
}
