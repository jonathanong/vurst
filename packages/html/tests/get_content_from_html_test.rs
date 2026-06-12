use napi::bindgen_prelude::Buffer;
use vurst_html_node::{get_content_from_html, CrawlerHtmlToMarkdownOptions};

#[test]
fn test_get_content_from_html_input_size_limit() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let huge_buffer = Buffer::from(vec![0; 10 * 1024 * 1024 + 1]);
        let result = get_content_from_html(
            huge_buffer,
            CrawlerHtmlToMarkdownOptions {
                css_selectors_to_remove: None,
                content_selectors: None,
                link_text_content_to_remove: None,
                link_hrefs_to_remove: None,
                link_rel_tokens_to_remove: None,
                use_text_density_filter: None,
            },
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.reason.contains("Input too large"));
    });
}
