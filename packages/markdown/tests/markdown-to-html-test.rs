use vurst_markdown_node::markdown_to_html::{
    extract_markdown_urls_sync, render_markdown_to_html_with_options, MarkdownRenderOptions,
};

fn default_opts() -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        proxy_images: false,
        ..MarkdownRenderOptions::default()
    }
}

fn admin_opts() -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        allow_html: true,
        nofollow_links: false,
        proxy_images: false,
        ..MarkdownRenderOptions::default()
    }
}

#[test]
fn renders_basic_paragraph() {
    let result = render_markdown_to_html_with_options("Hello world", &default_opts());
    assert!(result.contains("<p>Hello world</p>"));
}

#[test]
fn escapes_raw_html() {
    let result = render_markdown_to_html_with_options("<strong>Hi</strong>", &default_opts());
    assert!(!result.contains("<strong>Hi</strong>"));
    assert!(result.contains("&lt;strong&gt;Hi&lt;/strong&gt;"));
}

#[test]
fn admin_passes_raw_html() {
    let result = render_markdown_to_html_with_options("<strong>Hi</strong>", &admin_opts());
    assert!(result.contains("<strong>Hi</strong>"));
}

#[test]
fn external_link_gets_rel_and_target() {
    let result =
        render_markdown_to_html_with_options("[Example](https://example.com)", &default_opts());
    assert!(result.contains(r#"rel="nofollow ugc noopener""#));
    assert!(result.contains(r#"target="_blank""#));
    assert!(result.contains(r#"href="https://example.com""#));
}

#[test]
fn internal_link_no_rel() {
    let result = render_markdown_to_html_with_options("[Page](/path)", &default_opts());
    assert!(result.contains(r#"href="/path""#));
    assert!(!result.contains("nofollow"));
    assert!(!result.contains("target="));
}

#[test]
fn relative_links_with_colons_after_boundaries_are_preserved() {
    let result = render_markdown_to_html_with_options(
        "[redirect](/login?url=http://example.com) [path](./path:with:colons) [fragment](#redirect:https://example.com)",
        &default_opts(),
    );
    assert!(result.contains(r#"href="/login?url=http://example.com""#));
    assert!(result.contains(r#"href="./path:with:colons""#));
    assert!(result.contains(r##"href="#redirect:https://example.com""##));
}

#[test]
fn bare_unknown_colon_scheme_href_removed() {
    let result = render_markdown_to_html_with_options("[path](path:with:colons)", &default_opts());
    assert!(result.contains("path"));
    assert!(!result.contains(r#"href=""#));
}

#[test]
fn javascript_link_href_removed() {
    let result =
        render_markdown_to_html_with_options("[Click](javascript:alert(1))", &default_opts());
    assert!(result.contains("Click"));
    assert!(!result.contains("javascript:"));
    assert!(!result.contains(r#"href=""#));
}

#[test]
fn ws_link_href_removed() {
    let result = render_markdown_to_html_with_options("[WS](ws://evil.com)", &default_opts());
    assert!(result.contains("WS"));
    assert!(!result.contains("ws://"));
}

#[test]
fn data_image_src_removed() {
    let result =
        render_markdown_to_html_with_options("![img](data:image/png;base64,abc)", &default_opts());
    assert!(!result.contains("data:"));
}

#[test]
fn https_image_src_preserved() {
    let result = render_markdown_to_html_with_options(
        "![img](https://example.com/img.png)",
        &default_opts(),
    );
    assert!(result.contains(r#"src="https://example.com/img.png""#));
}

#[test]
fn extracts_link_urls() {
    let result = extract_markdown_urls_sync(
        "Visit [Example](https://example.com) and [Other](https://other.com)",
    );
    assert_eq!(result.link_urls.len(), 2);
    assert!(result
        .link_urls
        .contains(&"https://example.com".to_string()));
    assert!(result.link_urls.contains(&"https://other.com".to_string()));
}

#[test]
fn extracts_image_urls() {
    let result = extract_markdown_urls_sync("![img](https://example.com/img.png)");
    assert_eq!(result.image_urls.len(), 1);
    assert!(result
        .image_urls
        .contains(&"https://example.com/img.png".to_string()));
}

#[test]
fn excludes_dangerous_urls_from_extraction() {
    let result =
        extract_markdown_urls_sync("[evil](javascript:alert(1)) [good](https://example.com)");
    assert_eq!(result.link_urls.len(), 1);
    assert!(result
        .link_urls
        .contains(&"https://example.com".to_string()));
}

#[test]
fn extracts_relative_urls_with_colons_after_boundaries() {
    let result = extract_markdown_urls_sync(
        "[redirect](/login?url=http://example.com) [path](./path:with:colons)",
    );
    assert_eq!(
        result.link_urls,
        vec![
            "/login?url=http://example.com".to_string(),
            "./path:with:colons".to_string()
        ]
    );
}

#[test]
fn digit_first_scheme_href_removed() {
    let result =
        render_markdown_to_html_with_options("[link](1abc://attacker.com)", &default_opts());
    assert!(
        !result.contains("href="),
        "digit-first scheme should be rejected"
    );
}

#[test]
fn protocol_relative_link_href_removed() {
    let result =
        render_markdown_to_html_with_options("[link](//attacker.com/path)", &default_opts());
    assert!(
        !result.contains("href="),
        "protocol-relative URL should be rejected"
    );
}

#[test]
fn protocol_relative_image_src_removed() {
    let result =
        render_markdown_to_html_with_options("![img](//attacker.com/img.jpg)", &default_opts());
    assert!(
        !result.contains("src="),
        "protocol-relative image URL should be rejected"
    );
}

#[test]
fn bang_prefixed_same_site_post_url_keeps_bang_and_autolinks_url() {
    let result = render_markdown_to_html_with_options(
        "!https://example.com/discussion/test-post",
        &default_opts(),
    );
    assert!(result.contains("!<a "));
    assert!(result.contains(r#"href="https://example.com/discussion/test-post""#));
}

#[test]
fn bang_prefixed_same_site_comment_url_keeps_bang_and_autolinks_url() {
    let result = render_markdown_to_html_with_options(
        "!https://example.com/discussion/root-post/comment/019c64e6-f720-7001-a001-000000000010",
        &default_opts(),
    );
    assert!(result.contains("!<a "));
    assert!(result.contains(
        r#"href="https://example.com/discussion/root-post/comment/019c64e6-f720-7001-a001-000000000010""#,
    ));
}

#[test]
fn protocol_relative_bypass_href_removed() {
    let result =
        render_markdown_to_html_with_options("[link](\\\\\\attacker.com/path)", &default_opts());
    assert!(
        !result.contains("href="),
        "protocol-relative URL bypass should be rejected"
    );
}
