use vurst::image_proxy::rewrite_image_to_proxy;
use vurst::markdown_to_html::{
    extract_markdown_urls_sync, render_markdown_to_html_with_options, MarkdownRenderOptions,
};
use vurst::sanitize_html::{sanitize_rss_html_sync, SanitizeRssHtmlOptions};
use vurst::sanitize_prompt_injection::sanitize_prompt_injection_sync;
use vurst::slop_detection::detect_ai_generated_text;

#[test]
fn covers_markdown_and_sanitizer_edge_paths() {
    assert_eq!(
        render_markdown_to_html_with_options("", &MarkdownRenderOptions::default()),
        ""
    );
    let html = render_markdown_to_html_with_options(
        "[bad](1bad:) ![bad](mailto:test@example.com)",
        &MarkdownRenderOptions::default(),
    );
    assert!(!html.contains("1bad:"));
    assert!(!html.contains("mailto:test"));
    let urls = extract_markdown_urls_sync("[bad](1bad:) ![bad](mailto:test@example.com)");
    assert!(urls.link_urls.is_empty());
    assert!(urls.image_urls.is_empty());

    assert_eq!(
        sanitize_rss_html_sync("", &SanitizeRssHtmlOptions::default()).html,
        ""
    );
    let sanitized = sanitize_rss_html_sync(
        "<img src=\"/relative.png\"><a href=\"/safe\">safe</a><p style=\"x\">Text</p>",
        &SanitizeRssHtmlOptions {
            proxy_images: true,
            ..SanitizeRssHtmlOptions::default()
        },
    );
    assert!(sanitized.html.contains("/relative.png"));
    assert!(sanitized.html.contains("nofollow noopener"));

    assert_eq!(
        sanitize_prompt_injection_sync(
            "bad &#xD800; &#xzz; &#99999999; &#999999999999999999999;",
            false,
        ),
        "bad &#xD800; &#xzz; &#99999999; &#999999999999999999999;"
    );
    // Misconfigured (non-hex) signing keys fall back to the unsigned proxy path.
    let unsigned = rewrite_image_to_proxy(
        "https://example.com/a.png",
        "/proxy/",
        &["not-hex".to_string()],
    );
    assert!(unsigned.starts_with("/proxy/"));
    assert!(!unsigned.contains("sig="));
}

#[test]
fn covers_slop_edge_paths() {
    let slop = detect_ai_generated_text("generic marketing paragraph", 0.0)
        .expect("threshold zero should classify as AI");
    assert!(slop.flagged);
}
