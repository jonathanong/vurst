use super::*;
use crate::markdown_to_html::helpers::is_safe_link_url;
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
fn extract_markdown_urls_sync_basic() {
    let result = extract_markdown_urls_sync(
        "Here is a [link](https://example.com) and an ![image](https://example.com/image.png).",
    );
    assert_eq!(
        result,
        MarkdownUrlsResult {
            link_urls: vec!["https://example.com".to_string()],
            image_urls: vec!["https://example.com/image.png".to_string()],
        }
    );
}

#[test]
fn extract_markdown_urls_sync_autolinks() {
    let result = extract_markdown_urls_sync(
        "Check out https://example.com/autolink and <http://example.com/angle>",
    );
    assert_eq!(
        result,
        MarkdownUrlsResult {
            link_urls: vec![
                "https://example.com/autolink".to_string(),
                "http://example.com/angle".to_string(),
            ],
            image_urls: vec![],
        }
    );
}

#[test]
fn extract_markdown_urls_sync_plain_text_has_no_urls() {
    let result = extract_markdown_urls_sync("Just some plain text.");
    assert_eq!(
        result,
        MarkdownUrlsResult {
            link_urls: vec![],
            image_urls: vec![],
        }
    );
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

#[test]
fn test_is_safe_link_url() {
    // Valid schemes
    assert!(is_safe_link_url("http://example.com"));
    assert!(is_safe_link_url("https://example.com"));
    assert!(is_safe_link_url("mailto:test@example.com"));
    assert!(is_safe_link_url("tel:+1234567890"));
    assert!(is_safe_link_url("HTTP://EXAMPLE.COM"));

    // Empty URL
    assert!(!is_safe_link_url(""));

    // Relative paths without schemes
    assert!(is_safe_link_url("/path/to/resource"));
    assert!(is_safe_link_url("?query=1"));
    assert!(is_safe_link_url("#fragment"));
    assert!(is_safe_link_url("file.txt"));

    // Dangerous prefixes
    assert!(!is_safe_link_url("//example.com"));
    assert!(!is_safe_link_url("/\\example.com"));
    assert!(!is_safe_link_url("\\/example.com"));
    assert!(!is_safe_link_url("\\\\example.com"));
    assert!(!is_safe_link_url("\\example.com"));

    // Disallowed protocols
    assert!(!is_safe_link_url("javascript:alert(1)"));
    assert!(!is_safe_link_url("data:text/html,<html>"));
    assert!(!is_safe_link_url("file:///etc/passwd"));
    assert!(!is_safe_link_url("vbscript:msgbox(1)"));

    // Obfuscation attempts
    assert!(!is_safe_link_url("java\nscript:alert(1)"));
    assert!(!is_safe_link_url("java\tscript:alert(1)"));
    assert!(!is_safe_link_url("java\rscript:alert(1)"));
    assert!(!is_safe_link_url("java\x0Bscript:alert(1)"));
    assert!(!is_safe_link_url(" javascript:alert(1)"));
    assert!(!is_safe_link_url("\tjavascript:alert(1)"));
}
