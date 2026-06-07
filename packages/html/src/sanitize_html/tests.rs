use super::{sanitize_rss_html_sync, SanitizeRssHtmlOptions, SanitizeRssHtmlResult};

fn sanitize(html: &str) -> SanitizeRssHtmlResult {
    sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default())
}

fn sanitize_with_proxy(html: &str, keys: Vec<String>) -> SanitizeRssHtmlResult {
    sanitize_rss_html_sync(
        html,
        &SanitizeRssHtmlOptions {
            proxy_images: true,
            image_proxy_signing_keys: std::sync::Arc::from(keys),
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
fn detects_dangerous_url_scheme_fast_paths() {
    assert!(super::sanitize::has_dangerous_url_scheme(
        "da\tta:text/html"
    ));
    assert!(super::sanitize::has_dangerous_url_scheme(
        "vb\tscript:MsgBox(1)"
    ));
    assert!(!super::sanitize::has_dangerous_url_scheme(
        "java-safe:alert(1)"
    ));
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
    // Trailing ASCII whitespace with no closing tag: all remaining bytes after
    // the opening tag are whitespace, so iter().position returns None and the
    // None arm sets i = bytes.len() before breaking.
    assert!(!super::sanitize::may_have_empty_container("<div>   "));
    // Same None arm via vertical tab (0x0B): treated as whitespace by the
    // predicate (b == 0x0B) but not by u8::is_ascii_whitespace.
    assert!(!super::sanitize::may_have_empty_container("<p>\x0b\x0b"));
    // None arm reached after consuming a whitespace entity: the spaces after
    // &nbsp; scan to None since they run to end-of-input with no < present.
    assert!(!super::sanitize::may_have_empty_container(
        "<div>  &nbsp;   "
    ));
    // Non-ASCII, non-whitespace content (e.g. CJK): the fast-path byte scan
    // returns Some(0), the byte is not ASCII, and the Unicode decode finds a
    // non-whitespace char, exercising the `!ch.is_whitespace()` break arm.
    assert!(!super::sanitize::may_have_empty_container(
        "<div>\u{4e2d}</div>"
    ));
    assert!(!super::sanitize::may_have_empty_container(
        "<p>\u{00e9}</p>"
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
    // Vertical tab (0x0B) as whitespace: treated by predicate (b == 0x0B) but
    // NOT by u8::is_ascii_whitespace, so it doesn't trigger the fast None arm.
    assert!(super::sanitize::may_have_empty_container("<div>\x0b</div>"));
    // Long ASCII whitespace run: exercises multiple bytes in the fast-path
    // position scan before finding the closing-tag <.
    assert!(super::sanitize::may_have_empty_container(
        "<div>          \n\n\t\t\r\n</div>"
    ));
    // Decimal entity &#32; (space): whitespace codepoint via numeric entity.
    assert!(super::sanitize::may_have_empty_container(
        "<section>&#32;</section>"
    ));
}
