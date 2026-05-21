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
