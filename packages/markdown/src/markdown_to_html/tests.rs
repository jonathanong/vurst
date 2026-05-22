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
