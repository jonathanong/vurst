use vurst_html_node::sanitize_prompt_injection_sync;

#[test]
fn test_fast_path_coverage() {
    let _ = sanitize_prompt_injection_sync("\u{E000}", false);
    let _ = sanitize_prompt_injection_sync("\u{E001}", false);
    let _ = sanitize_prompt_injection_sync("\u{E000}\u{E001}", false);

    // system message boundary uses E000 and E001
    let _ = sanitize_prompt_injection_sync("system: test\u{E000}", false);
    let _ = sanitize_prompt_injection_sync("system: test\u{E001}", false);
    let _ = sanitize_prompt_injection_sync("system: test\u{E000}\u{E001}", false);
}
