use vurst_html_node::{apply_dom_removals_to_html, ExtractDomRemovalsResult};
use napi::bindgen_prelude::Buffer;
use tokio::runtime::Runtime;

#[test]
fn test_invalid_utf8_in_apply_dom_removals() {
    let rt = Runtime::new().unwrap();
    let invalid_utf8_bytes = vec![0xFF, 0xFF, 0xFF];
    let buffer = Buffer::from(invalid_utf8_bytes);
    let removals = ExtractDomRemovalsResult {
        css_selectors_to_remove: vec![],
        html_to_remove: vec![],
    };

    let result = rt.block_on(apply_dom_removals_to_html(buffer, removals));
    match result {
        Ok(_) => panic!("Expected error for invalid UTF-8"),
        Err(err) => {
            assert!(err.reason.contains("Invalid UTF-8 in HTML"));
        }
    }
}
