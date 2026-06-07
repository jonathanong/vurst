use vurst_html_node::sanitize_html::{sanitize_rss_html_sync, SanitizeRssHtmlOptions};
use vurst_html_node::sanitize_prompt_injection::sanitize_prompt_injection_sync;
use vurst_runtime_rs::image_proxy::rewrite_image_to_proxy;

#[test]
fn covers_sanitizer_edge_paths() {
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
