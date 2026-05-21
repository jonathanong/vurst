use vurst_markdown_node::markdown_to_html::{
    extract_markdown_urls_sync, render_markdown_to_html_with_options, MarkdownRenderOptions,
};

#[test]
fn covers_markdown_edge_paths() {
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
}
