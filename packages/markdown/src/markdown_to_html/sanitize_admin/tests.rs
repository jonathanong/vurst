use super::*;

#[test]
fn sanitizer_private_helpers_cover_empty_and_attr_paths() {
    assert_eq!(
        sanitize_admin_html_with_options("", &AdminHtmlOptions::default()),
        ""
    );
    assert_eq!(
        sanitize_admin_html_with_options(
            "<script>bad()</script><img src=\"javascript:bad\" alt=\"x\"><a href=\"javascript:bad\">x</a><br><p>ok</p><!-- comment -->",
            &AdminHtmlOptions::default(),
        ),
        "<img alt=\"x\"><a>x</a><br><p>ok</p>"
    );
    assert_eq!(
        sanitize_admin_html_with_options("<unknown>x</unknown>", &AdminHtmlOptions::default()),
        "x"
    );
    assert_eq!(
        sanitize_admin_html_with_options("<hr>", &AdminHtmlOptions::default()),
        "<hr>"
    );
    assert_eq!(
        sanitize_admin_html_with_options(
            "<a title=\"details\">x</a>",
            &AdminHtmlOptions::default(),
        ),
        "<a title=\"details\">x</a>"
    );
    assert_eq!(
        sanitize_admin_html_with_options(
            "<p class=\"kept\" id=\"copy\" style=\"color:red\" onclick=\"bad()\" aria-label=\"skip\">ok</p>",
            &AdminHtmlOptions::default(),
        ),
        "<p class=\"kept\" id=\"copy\">ok</p>"
    );
    assert_eq!(
        sanitize_admin_html_with_options(
            "<img src=\"/local.png\">",
            &AdminHtmlOptions {
                nofollow_links: false,
                proxy_images: true,
                image_proxy_signing_keys: Vec::new(),
                ..AdminHtmlOptions::default()
            },
        ),
        "<img src=\"/local.png\">"
    );
    let fragment = Html::parse_fragment("<p>ok</p>");
    assert_eq!(
        render_node(fragment.tree.root(), &AdminHtmlOptions::default()),
        "<p>ok</p>"
    );
}

#[test]
fn sanitizer_options_apply_link_and_image_policy_during_render() {
    let html = sanitize_admin_html_with_options(
        "<a href=\"https://example.com\" rel=\"author\" target=\"_self\">x</a><img src=\"https://example.com/a.png\">",
        &AdminHtmlOptions {
            nofollow_links: true,
            proxy_images: true,
            image_proxy_signing_keys: Vec::new(),
            ..AdminHtmlOptions::default()
        },
    );

    assert!(html.contains(r#"rel="nofollow ugc noopener" target="_blank""#));
    assert!(!html.contains(r#"target="_self""#));
    assert!(html.contains(r#"src="/proxy/"#));
    assert!(!html.contains("https://example.com/a.png"));

    let dofollow_html = sanitize_admin_html_with_options(
        "<a href=\"https://example.com\">x</a>",
        &AdminHtmlOptions {
            nofollow_links: false,
            proxy_images: false,
            image_proxy_signing_keys: Vec::new(),
            ..AdminHtmlOptions::default()
        },
    );
    assert!(dofollow_html.contains(r#"rel="noopener" target="_blank""#));
    assert!(!dofollow_html.contains("nofollow"));
}

#[test]
fn sanitize_admin_escape_text() {
    assert_eq!(escape_text("hello world"), "hello world");
    assert_eq!(escape_text("a & b"), "a &amp; b");
    assert_eq!(escape_text("1 < 2"), "1 &lt; 2");
    assert_eq!(escape_text("2 > 1"), "2 &gt; 1");
    assert_eq!(escape_text("a & b < c > d"), "a &amp; b &lt; c &gt; d");
    assert_eq!(escape_text("<>&"), "&lt;&gt;&amp;");
}
