use super::*;
use comrak::nodes::AstNode;

fn first_link_node<'a>(node: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
    if matches!(node.data().value, NodeValue::Link(_)) {
        return Some(node);
    }

    node.children().find_map(first_link_node)
}

#[test]
fn rendered_image_src_only_proxies_when_enabled_and_external() {
    let opts = MarkdownHtmlFormatOptions {
        nofollow_links: true,
        proxy_images: true,
        image_proxy_url_prefix: crate::image_proxy::DEFAULT_IMAGE_PROXY_URL_PREFIX,
        image_proxy_signing_keys: &[],
    };

    assert!(rendered_image_src("https://example.com/a.png", &opts).starts_with("/proxy/"));
    assert_eq!(rendered_image_src("/local/a.png", &opts), "/local/a.png");

    let opts = MarkdownHtmlFormatOptions {
        proxy_images: false,
        ..opts
    };
    assert_eq!(
        rendered_image_src("https://example.com/a.png", &opts),
        "https://example.com/a.png"
    );
}

#[test]
fn formatter_options_copy_render_policy() {
    let opts = MarkdownRenderOptions {
        allow_html: true,
        nofollow_links: false,
        proxy_images: false,
        image_proxy_signing_keys: vec!["abc".to_string()],
        ..MarkdownRenderOptions::default()
    };

    let formatter_opts = MarkdownHtmlFormatOptions::from(&opts);

    assert!(!formatter_opts.nofollow_links);
    assert!(!formatter_opts.proxy_images);
    assert_eq!(
        formatter_opts.image_proxy_signing_keys,
        vec!["abc".to_string()]
    );
}

#[test]
fn should_render_link_when_relaxed_autolinks_are_disabled() {
    let mut options = Options::default();
    options.parse.relaxed_autolinks = false;
    let arena = Arena::new();
    let root = parse_document(&arena, "[link](https://example.com)", &options);
    let link = first_link_node(root).expect("expected parsed link");

    assert!(should_render_nested_link(link, &options));
}

#[test]
fn should_render_regular_and_root_nodes_when_relaxed_autolinks_are_enabled() {
    let mut options = Options::default();
    options.parse.relaxed_autolinks = true;
    let arena = Arena::new();
    let root = parse_document(&arena, "[link](https://example.com)", &options);
    let link = first_link_node(root).expect("expected parsed link");

    assert!(should_render_nested_link(link, &options));
    assert!(should_render_nested_link(root, &options));
}

#[test]
fn extracts_urls_from_markdown_sync() {
    let md = r#"
Here is a [link](https://example.com) and another [one](http://test.com).
Here is an image: ![image](https://example.com/img.png).
And an unsafe link: [unsafe](javascript:alert(1)).
"#;
    let result = extract_markdown_urls_sync(md);

    assert_eq!(
        result.link_urls,
        vec![
            "https://example.com".to_string(),
            "http://test.com".to_string()
        ]
    );
    assert_eq!(
        result.image_urls,
        vec!["https://example.com/img.png".to_string()]
    );
}

#[test]
fn extracts_urls_empty_markdown() {
    let md = "";
    let result = extract_markdown_urls_sync(md);

    assert!(result.link_urls.is_empty());
    assert!(result.image_urls.is_empty());
}

#[test]
fn extracts_urls_no_links() {
    let md = "Just some text with no links or images.";
    let result = extract_markdown_urls_sync(md);

    assert!(result.link_urls.is_empty());
    assert!(result.image_urls.is_empty());
}

#[test]
fn should_not_render_nested_link_when_parent_is_link() {
    let mut options = Options::default();
    options.parse.relaxed_autolinks = true;
    let arena = Arena::new();

    let root1 = parse_document(&arena, "[parent](https://a.com)", &options);
    let parent_link = first_link_node(root1).expect("expected parent link");

    let root2 = parse_document(&arena, "[child](https://b.com)", &options);
    let child_link = first_link_node(root2).expect("expected child link");

    parent_link.append(child_link);

    assert!(!should_render_nested_link(child_link, &options));
}
