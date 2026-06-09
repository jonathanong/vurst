use super::*;
use crate::markdown_to_html::helpers::*;
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
        image_proxy_url_prefix: vurst_shared::image_proxy::DEFAULT_IMAGE_PROXY_URL_PREFIX,
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
fn extract_bare_domain_with_path() {
    let result = extract_markdown_urls_sync("join discord.gg/raid please");
    assert_eq!(result.link_urls, vec!["discord.gg/raid"]);
    assert!(result.image_urls.is_empty());
}

#[test]
fn extract_bare_domain_short_tld() {
    let result = extract_markdown_urls_sync("msg me on t.me/spam");
    assert_eq!(result.link_urls, vec!["t.me/spam"]);
}

#[test]
fn extract_bare_domain_common_tld() {
    let result = extract_markdown_urls_sync("see example.com/path for details");
    assert_eq!(result.link_urls, vec!["example.com/path"]);
}

#[test]
fn extract_bare_domain_url_shortener() {
    let result = extract_markdown_urls_sync("click bit.ly/x now");
    assert_eq!(result.link_urls, vec!["bit.ly/x"]);
}

#[test]
fn extract_bare_domain_without_path() {
    let result = extract_markdown_urls_sync("visit example.com today");
    assert_eq!(result.link_urls, vec!["example.com"]);
}

#[test]
fn bare_domain_not_extracted_from_file_extension_like_tokens() {
    // node.js has no registered TLD (.js is not in the PSL)
    let result = extract_markdown_urls_sync("node.js is fast");
    assert!(
        result.link_urls.is_empty(),
        "node.js should not match: {result:?}"
    );
}

#[test]
fn bare_domain_not_extracted_from_version_strings() {
    let result = extract_markdown_urls_sync("use version v1.0 today");
    assert!(
        result.link_urls.is_empty(),
        "v1.0 should not match: {result:?}"
    );
}

#[test]
fn bare_domain_not_extracted_from_abbreviations() {
    let result = extract_markdown_urls_sync("i.e. some clarification");
    assert!(
        result.link_urls.is_empty(),
        "i.e. should not match: {result:?}"
    );
}

#[test]
fn bare_domain_not_extracted_from_code_span() {
    // Inline code spans must not be scanned for links.
    let result = extract_markdown_urls_sync("run `example.com` in your browser");
    assert!(
        result.link_urls.is_empty(),
        "code span should not match: {result:?}"
    );
}

#[test]
fn bare_domain_not_extracted_from_code_fence() {
    let result = extract_markdown_urls_sync("```\nexample.com\n```");
    assert!(
        result.link_urls.is_empty(),
        "code fence should not match: {result:?}"
    );
}

#[test]
fn bare_domain_deduped_when_repeated() {
    let result = extract_markdown_urls_sync("visit discord.gg/raid or discord.gg/raid");
    assert_eq!(result.link_urls, vec!["discord.gg/raid"]);
}

#[test]
fn bare_domain_not_duplicated_when_markdown_link_already_present() {
    // Explicit markdown link + same bare-domain text — should appear once.
    let result = extract_markdown_urls_sync("[join](discord.gg/raid) or discord.gg/raid");
    assert_eq!(result.link_urls, vec!["discord.gg/raid"]);
}

#[test]
fn bare_domain_not_extracted_when_psl_returns_none() {
    // Tokens that match the regex but have no registered TLD in the PSL.
    let result = extract_markdown_urls_sync("see foo.notregistered for more");
    assert!(
        result.link_urls.is_empty(),
        "unregistered TLD should not match: {result:?}"
    );
}

#[test]
fn bare_domain_not_extracted_from_email_address() {
    // comrak autolinks user@example.com → mailto: link; the domain must not
    // also appear as a separate bare-domain entry in link_urls.
    let result = extract_markdown_urls_sync("contact user@example.com for info");
    assert!(
        !result.link_urls.iter().any(|u| u == "example.com"),
        "email domain should not be double-extracted: {result:?}"
    );
}

#[test]
fn bare_domain_path_trailing_punctuation_trimmed() {
    // Trailing sentence punctuation must be stripped from the path.
    let result = extract_markdown_urls_sync("see example.com/path. for details");
    assert_eq!(result.link_urls, vec!["example.com/path"]);
}

#[test]
fn rejects_semicolonless_decimal_entity_in_javascript_link() {
    assert!(!is_safe_link_url("javascript&#58alert(1)"));
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

#[test]
fn test_is_safe_image_url() {
    // Valid schemes for images
    assert!(is_safe_image_url("http://example.com/image.png"));
    assert!(is_safe_image_url("https://example.com/image.png"));

    // Disallowed schemes for images
    assert!(!is_safe_image_url("mailto:test@example.com"));
    assert!(!is_safe_image_url("tel:+1234567890"));
    assert!(!is_safe_image_url("javascript:alert(1)"));

    // Relative paths are allowed
    assert!(is_safe_image_url("/path/to/image.png"));

    // Dangerous prefixes are blocked
    assert!(!is_safe_image_url("//example.com/image.png"));
}
