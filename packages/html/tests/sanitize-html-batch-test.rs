use napi::bindgen_prelude::Buffer;
use vurst_html_node::sanitize_rss_html_batch;

#[test]
fn handles_invalid_utf8() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Create an invalid UTF-8 byte sequence
    let invalid_utf8 = vec![0xff, 0xfe, 0xfd];
    let buffers = vec![
        Buffer::from(b"<p>Valid</p>".to_vec()),
        Buffer::from(invalid_utf8),
    ];

    let result = rt.block_on(sanitize_rss_html_batch(buffers, None));

    match result {
        Ok(_) => panic!("Expected error for invalid UTF-8"),
        Err(e) => {
            assert!(e.reason.contains("Invalid UTF-8 in inputs[1]:"));
        }
    }
}
