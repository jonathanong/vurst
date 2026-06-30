use napi::bindgen_prelude::*;

#[test]
fn test_sanitize_rss_html_invalid_utf8() {
    let invalid_utf8 = vec![0xff, 0xff, 0xff];
    let buffer = Buffer::from(invalid_utf8);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let err = rt
        .block_on(vurst_html_node::sanitize_rss_html(buffer, None))
        .err()
        .expect("Expected invalid UTF-8 error");
    assert!(err.reason.contains("Invalid UTF-8 in HTML"));
}
