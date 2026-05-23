use vurst_html_node::{sanitize_rss_html_sync, SanitizeRssHtmlOptions};

fn sanitize(html: &str) -> String {
    sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default()).html
}

// === Dangerous URL schemes ===

#[test]
fn strips_javascript_href() {
    let html = r#"<a href="javascript:alert('xss')">Click me</a>"#;
    let result = sanitize(html);
    assert!(result.contains("Click me"));
    assert!(!result.contains("javascript:"));
    assert!(!result.contains("alert"));
}

#[test]
fn strips_data_href() {
    let html = r#"<a href="data:text/html,<script>alert('xss')</script>">Link</a>"#;
    let result = sanitize(html);
    assert!(result.contains("Link"));
    assert!(!result.contains("data:"));
}

#[test]
fn strips_vbscript_href() {
    let html = r#"<a href="vbscript:MsgBox('xss')">Link</a>"#;
    let result = sanitize(html);
    assert!(result.contains("Link"));
    assert!(!result.contains("vbscript:"));
}

#[test]
fn strips_javascript_src_on_img() {
    let html = r#"<img src="javascript:alert('xss')" alt="broken">"#;
    let result = sanitize(html);
    assert!(!result.contains("javascript:"));
    assert!(result.contains(r#"alt="broken""#));
}

#[test]
fn strips_data_src_on_img() {
    let html = r#"<img src="data:image/svg+xml,<svg onload=alert(1)>" alt="evil">"#;
    let result = sanitize(html);
    assert!(!result.contains("data:"));
}

#[test]
fn handles_whitespace_before_dangerous_scheme() {
    let html = r#"<a href="  javascript:alert(1)">Link</a>"#;
    let result = sanitize(html);
    assert!(!result.contains("javascript:"));
}

#[test]
fn strips_embedded_tab_in_javascript_href() {
    // Literal tab, as would appear after scraper decodes `&#x09;`
    let html = "<a href=\"jav\tascript:alert(1)\">Tab</a>";
    let result = sanitize(html);
    assert!(result.contains("Tab"));
    assert!(!result.contains("javascript"));
    assert!(!result.contains("ascript:"));
    assert!(!result.contains("alert"));
}

#[test]
fn strips_scattered_whitespace_in_javascript_href() {
    // Mix of TAB, LF, CR — covers all three WHATWG-stripped chars in one test
    let html = "<a href=\"j\ta\nv\ra\tscript:alert(1)\">Link</a>";
    let result = sanitize(html);
    assert!(result.contains("Link"));
    assert!(!result.contains("alert"));
    assert!(!result.contains("script:"));
}

#[test]
fn strips_form_feed_in_javascript_href() {
    // Form feed (U+000C) — covered by is_ascii_whitespace, defensive filtering
    let html = "<a href=\"jav\x0Cascript:alert(1)\">FF</a>";
    let result = sanitize(html);
    assert!(result.contains("FF"));
    assert!(!result.contains("alert"));
}

#[test]
fn strips_c0_controls_in_javascript_href() {
    // C0 control chars like SOH (U+0001) are ignored by browsers in URLs
    let html = "<a href=\"jav\x01ascript:alert(1)\">C0</a>";
    let result = sanitize(html);
    assert!(result.contains("C0"));
    assert!(!result.contains("alert"));
    assert!(!result.contains("javascript"));
}

#[test]
fn strips_mixed_case_with_whitespace_in_javascript_href() {
    let html = "<a href=\"Ja\tVaScRiPt:alert(1)\">Mixed</a>";
    let result = sanitize(html);
    assert!(result.contains("Mixed"));
    assert!(!result.contains("alert"));
}

#[test]
fn strips_embedded_whitespace_in_data_scheme() {
    let html = "<a href=\"da\tta:text/html,<script>alert(1)</script>\">Data</a>";
    let result = sanitize(html);
    assert!(result.contains("Data"));
    assert!(!result.contains("alert"));
    assert!(!result.contains("data:"));
}

#[test]
fn strips_embedded_whitespace_in_vbscript_scheme() {
    let html = "<a href=\"vb\tscript:MsgBox(1)\">VB</a>";
    let result = sanitize(html);
    assert!(result.contains("VB"));
    assert!(!result.contains("MsgBox"));
    assert!(!result.contains("vbscript"));
}

#[test]
fn strips_embedded_tab_in_img_src_javascript() {
    let html = "<img src=\"jav\tascript:alert(1)\" alt=\"broken\">";
    let result = sanitize(html);
    assert!(result.contains("alt=\"broken\""));
    assert!(!result.contains("alert"));
}

#[test]
fn preserves_mailto_after_whitespace_fix() {
    // Ensures the new filter does not mutate href values — it only checks them
    let html = r#"<a href="mailto:user@example.com">Email</a>"#;
    let result = sanitize(html);
    assert!(result.contains("mailto:user@example.com"));
}

#[test]
fn preserves_https_url_with_path_unchanged() {
    let html = r#"<a href="https://example.com/path?q=1">Link</a>"#;
    let result = sanitize(html);
    assert!(result.contains("https://example.com/path?q=1"));
}

#[test]
fn preserves_safe_hrefs() {
    let html = r#"<a href="https://example.com">HTTPS</a><a href="http://example.com">HTTP</a><a href="/relative">Relative</a><a href="mailto:user@example.com">Email</a>"#;
    let result = sanitize(html);
    assert!(result.contains("https://example.com"));
    assert!(result.contains("http://example.com"));
    assert!(result.contains("/relative"));
    assert!(result.contains("mailto:user@example.com"));
}

// === Attribute additions ===

#[test]
fn adds_rel_and_target_to_links() {
    let html = r#"<a href="https://example.com">Link</a>"#;
    let result = sanitize(html);
    assert!(result.contains(r#"rel="nofollow noopener""#));
    assert!(result.contains(r#"target="_blank""#));
    assert!(result.contains(r#"href="https://example.com""#));
    assert!(result.contains("Link"));
}

#[test]
fn overwrites_existing_rel_and_target_on_links() {
    let html = r#"<a href="https://example.com" rel="dofollow" target="_self">Link</a>"#;
    let result = sanitize(html);
    assert!(result.contains(r#"rel="nofollow noopener""#));
    assert!(result.contains(r#"target="_blank""#));
    assert!(!result.contains("dofollow"));
    assert!(!result.contains("_self"));
}

#[test]
fn adds_performance_attrs_to_images() {
    let html = r#"<img src="photo.jpg" alt="A photo">"#;
    let result = sanitize(html);
    assert!(result.contains(r#"loading="lazy""#));
    assert!(result.contains(r#"fetchpriority="low""#));
    assert!(result.contains(r#"decoding="async""#));
    assert!(result.contains(r#"src="photo.jpg""#));
    assert!(result.contains(r#"alt="A photo""#));
}

// === Preserved elements ===

#[test]
fn preserves_headings() {
    let html =
        r#"<h1>Title</h1><h2>Subtitle</h2><h3>Section</h3><h4>Sub</h4><h5>Minor</h5><h6>Tiny</h6>"#;
    let result = sanitize(html);
    assert!(result.contains("<h1>Title</h1>"));
    assert!(result.contains("<h2>Subtitle</h2>"));
    assert!(result.contains("<h3>Section</h3>"));
    assert!(result.contains("<h4>Sub</h4>"));
    assert!(result.contains("<h5>Minor</h5>"));
    assert!(result.contains("<h6>Tiny</h6>"));
}

#[test]
fn preserves_paragraph_and_formatting() {
    let html = r#"<p>A <strong>bold</strong> and <em>italic</em> paragraph.</p>"#;
    let result = sanitize(html);
    assert!(result.contains("<p>"));
    assert!(result.contains("<strong>bold</strong>"));
    assert!(result.contains("<em>italic</em>"));
}

#[test]
fn preserves_lists() {
    let html = r#"<ul><li>Item 1</li><li>Item 2</li></ul><ol><li>First</li></ol>"#;
    let result = sanitize(html);
    assert!(result.contains("<ul>"));
    assert!(result.contains("<li>Item 1</li>"));
    assert!(result.contains("<ol>"));
}

#[test]
fn preserves_blockquote() {
    let html = r#"<blockquote>A wise quote</blockquote>"#;
    let result = sanitize(html);
    assert!(result.contains("<blockquote>A wise quote</blockquote>"));
}

#[test]
fn preserves_tables() {
    let html = r#"<table><tr><th>Header</th></tr><tr><td>Cell</td></tr></table>"#;
    let result = sanitize(html);
    assert!(result.contains("<table>"));
    assert!(result.contains("<th>Header</th>"));
    assert!(result.contains("<td>Cell</td>"));
}

#[test]
fn strips_unknown_tags_but_preserves_text() {
    let html = r#"<custom-card data-kind="promo"><p>Useful content</p></custom-card>"#;
    let result = sanitize(html);
    assert!(result.contains("<p>Useful content</p>"));
    assert!(!result.contains("custom-card"));
    assert!(!result.contains("data-kind"));
}
