use data_encoding::BASE64URL_NOPAD;
use vurst_markdown_node::markdown_to_html::{render_markdown_to_html_with_options, MarkdownRenderOptions};

// === Non-admin mode (default options) ===

#[test]
fn nonadmin_escapes_html() {
    let result = render_markdown_to_html_with_options(
        "<b>bold</b>",
        &MarkdownRenderOptions {
            allow_html: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(result.contains("&lt;b&gt;bold&lt;/b&gt;"));
}

#[test]
fn nonadmin_external_links_get_nofollow() {
    let result = render_markdown_to_html_with_options(
        "[Link](https://example.com)",
        &MarkdownRenderOptions::default(),
    );
    assert!(result.contains(r#"rel="nofollow ugc noopener""#));
    assert!(result.contains(r#"target="_blank""#));
}

// === Admin mode ===

#[test]
fn admin_allows_safe_html() {
    let result = render_markdown_to_html_with_options(
        "<b>bold</b> and <em>italic</em>",
        &MarkdownRenderOptions {
            allow_html: true,
            nofollow_links: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(result.contains("<b>bold</b>"));
    assert!(result.contains("<em>italic</em>"));
}

#[test]
fn admin_strips_script_tags() {
    let result = render_markdown_to_html_with_options(
        "safe <script>alert('xss')</script>",
        &MarkdownRenderOptions {
            allow_html: true,
            nofollow_links: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(!result.contains("<script>"));
    assert!(!result.contains("alert"));
}

#[test]
fn admin_strips_iframe() {
    let result = render_markdown_to_html_with_options(
        "text <iframe src=\"https://evil.com\"></iframe>",
        &MarkdownRenderOptions {
            allow_html: true,
            nofollow_links: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(!result.contains("<iframe"));
    assert!(!result.contains("evil.com"));
}

#[test]
fn admin_strips_event_handlers() {
    let result = render_markdown_to_html_with_options(
        "<p onclick=\"alert('xss')\">text</p>",
        &MarkdownRenderOptions {
            allow_html: true,
            nofollow_links: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(!result.contains("onclick"));
    assert!(result.contains("<p>text</p>"));
}

#[test]
fn admin_external_links_dofollow() {
    let result = render_markdown_to_html_with_options(
        "[Link](https://example.com)",
        &MarkdownRenderOptions {
            allow_html: true,
            nofollow_links: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(result.contains(r#"rel="noopener""#));
    assert!(!result.contains("nofollow"));
    assert!(result.contains(r#"target="_blank""#));
}

#[test]
fn admin_html_external_links_can_get_nofollow() {
    let result = render_markdown_to_html_with_options(
        "<a href=\"https://example.com\">Link</a>",
        &MarkdownRenderOptions {
            allow_html: true,
            nofollow_links: true,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(result.contains(r#"rel="nofollow ugc noopener""#));
    assert!(result.contains(r#"target="_blank""#));
}

// === Image proxying ===

#[test]
fn proxy_external_image() {
    let result = render_markdown_to_html_with_options(
        "![img](https://example.com/photo.jpg)",
        &MarkdownRenderOptions::default(),
    );
    assert!(result.contains(r#"src="/proxy/"#));
    assert!(!result.contains("example.com/photo.jpg"));
}

#[test]
fn proxy_external_image_trims_whitespace() {
    let result = render_markdown_to_html_with_options(
        "![img]( https://example.com/photo.jpg )",
        &MarkdownRenderOptions::default(),
    );
    assert!(result.contains(r#"src="/proxy/"#));

    let encoded = result
        .split(r#"src="/proxy/"#)
        .nth(1)
        .and_then(|part| part.split('"').next())
        .expect("expected image proxy src");
    let decoded = String::from_utf8(
        BASE64URL_NOPAD
            .decode(encoded.as_bytes())
            .expect("expected valid base64url"),
    )
    .expect("expected UTF-8");

    assert_eq!(decoded, "https://example.com/photo.jpg");
}

#[test]
fn proxy_does_not_rewrite_relative_images() {
    let result = render_markdown_to_html_with_options(
        "![img](/images/local.png)",
        &MarkdownRenderOptions::default(),
    );
    assert!(result.contains(r#"src="/images/local.png""#));
    assert!(!result.contains("/proxy/"));
}

#[test]
fn proxy_does_not_double_rewrite_image_proxy() {
    let result = render_markdown_to_html_with_options(
        "![img](/proxy/abc123)",
        &MarkdownRenderOptions::default(),
    );
    assert!(result.contains(r#"src="/proxy/abc123""#));
}

#[test]
fn proxy_disabled_preserves_external_images() {
    let result = render_markdown_to_html_with_options(
        "![img](https://example.com/photo.jpg)",
        &MarkdownRenderOptions {
            proxy_images: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(result.contains(r#"src="https://example.com/photo.jpg""#));
    assert!(!result.contains("/proxy/"));
}

#[test]
fn proxy_with_admin_html_images() {
    let result = render_markdown_to_html_with_options(
        "<img src=\"https://example.com/admin.png\" alt=\"admin\">",
        &MarkdownRenderOptions {
            allow_html: true,
            nofollow_links: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(result.contains(r#"src="/proxy/"#));
    assert!(!result.contains("example.com/admin.png"));

    let local_result = render_markdown_to_html_with_options(
        "<img src=\"/images/admin.png\" alt=\"admin\">",
        &MarkdownRenderOptions {
            allow_html: true,
            nofollow_links: false,
            ..MarkdownRenderOptions::default()
        },
    );
    assert!(local_result.contains(r#"src="/images/admin.png""#));
    assert!(!local_result.contains("/proxy/"));
}

// === Default options ===

#[test]
fn default_options_are_non_admin() {
    let opts = MarkdownRenderOptions::default();
    assert!(!opts.allow_html);
    assert!(opts.nofollow_links);
    assert!(opts.proxy_images);
}
