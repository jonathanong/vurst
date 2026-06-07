use vurst_html_node::{sanitize_rss_html_sync, SanitizeRssHtmlOptions};

#[test]
fn test_sanitize_xss_bypass() {
    let opts = SanitizeRssHtmlOptions {
        proxy_images: false,
        image_proxy_url_prefix: "".to_string(),
        image_proxy_signing_keys: vec![],
    };

    // Standard javascript scheme
    let html = "<a href=\"javascript:alert(1)\">click</a>";
    let res = sanitize_rss_html_sync(html, &opts);
    assert_eq!(
        res.html,
        "<a target=\"_blank\" rel=\"nofollow noopener\">click</a>"
    );

    // HTML entity encoded colon
    let html = "<a href=\"javascript&#58;alert(1)\">click</a>";
    let res = sanitize_rss_html_sync(html, &opts);
    assert_eq!(
        res.html,
        "<a target=\"_blank\" rel=\"nofollow noopener\">click</a>"
    );

    // HTML entity named colon
    let html = "<a href=\"javascript&colon;alert(1)\">click</a>";
    let res = sanitize_rss_html_sync(html, &opts);
    assert_eq!(
        res.html,
        "<a target=\"_blank\" rel=\"nofollow noopener\">click</a>"
    );

    // Fully entity encoded javascript
    let html = "<a href=\"&#106;&#97;&#118;&#97;&#115;&#99;&#114;&#105;&#112;&#116;&#58;alert(1)\">click</a>";
    let res = sanitize_rss_html_sync(html, &opts);
    assert_eq!(
        res.html,
        "<a target=\"_blank\" rel=\"nofollow noopener\">click</a>"
    );

    // Encoded mixed with whitespace
    let html = "<a href=\"java&#9;script&#58;alert(1)\">click</a>";
    let res = sanitize_rss_html_sync(html, &opts);
    assert_eq!(
        res.html,
        "<a target=\"_blank\" rel=\"nofollow noopener\">click</a>"
    );
}
