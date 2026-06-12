use napi::bindgen_prelude::Buffer;
use tokio::runtime::Runtime;
use vurst_html_node::{apply_dom_removals_to_html, ExtractDomRemovalsResult};

#[test]
fn test_invalid_utf8_in_apply_dom_removals() {
    let rt = Runtime::new().unwrap();
    let invalid_utf8_bytes = vec![0xFF, 0xFF, 0xFF];
    let buffer = Buffer::from(invalid_utf8_bytes);
    let removals = ExtractDomRemovalsResult {
        css_selectors_to_remove: vec![],
        html_to_remove: vec![],
    };

    match rt.block_on(apply_dom_removals_to_html(buffer, removals)) {
        Ok(_) => panic!("Expected error for invalid UTF-8"),
        Err(err) => {
            assert_eq!(
                err.reason,
                "Invalid UTF-8 in HTML: invalid utf-8 sequence of 1 bytes from index 0"
            );
        }
    }
}

#[test]
fn test_apply_dom_removals_to_html_size_limit() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let large_html = vec![b'a'; 10 * 1024 * 1024 + 1];
        let buffer = Buffer::from(large_html);
        let removals = ExtractDomRemovalsResult {
            css_selectors_to_remove: vec![],
            html_to_remove: vec![],
        };

        let result = apply_dom_removals_to_html(buffer, removals).await;
        assert!(result.is_err());
        match result {
            Err(e) => {
                assert!(e.reason.contains("Input too large"));
            }
            Ok(_) => panic!("Expected error due to large input"),
        }
    });
}
