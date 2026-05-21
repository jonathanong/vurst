use vurst_html_node::{sanitize_rss_html_sync, SanitizeRssHtmlOptions};

fn sanitize(html: &str) -> String {
    sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default()).html
}

// === Element removal ===

#[test]
fn removes_script_elements() {
    let html = r#"<p>Content</p><script>alert('xss')</script><p>More</p>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(result.contains("More"));
    assert!(!result.contains("script"));
    assert!(!result.contains("alert"));
}

#[test]
fn removes_style_elements() {
    let html = r#"<p>Content</p><style>.foo { color: red; }</style>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("style"));
    assert!(!result.contains("color"));
}

#[test]
fn removes_iframe_elements() {
    let html = r#"<p>Content</p><iframe src="https://evil.com"></iframe>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("iframe"));
    assert!(!result.contains("evil.com"));
}

#[test]
fn removes_form_and_input_elements() {
    let html = r#"<p>Content</p><form action="/submit"><input type="text"><button>Submit</button><select><option>A</option></select><textarea>notes</textarea></form>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("form"));
    assert!(!result.contains("input"));
    assert!(!result.contains("button"));
    assert!(!result.contains("select"));
    assert!(!result.contains("textarea"));
}

#[test]
fn removes_object_and_embed_elements() {
    let html = r#"<p>Content</p><object data="movie.swf"></object><embed src="plugin.swf">"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("object"));
    assert!(!result.contains("embed"));
    assert!(!result.contains("swf"));
}

// === Attribute stripping ===

#[test]
fn strips_style_attribute() {
    let html = r#"<p style="color: red; font-size: 14px;">Text</p>"#;
    let result = sanitize(html);
    assert!(result.contains("<p>"));
    assert!(result.contains("Text"));
    assert!(!result.contains("style"));
    assert!(!result.contains("color"));
}

#[test]
fn strips_class_attribute() {
    let html = r#"<div class="wp-block-paragraph has-large-font-size">Text</div>"#;
    let result = sanitize(html);
    assert!(result.contains("Text"));
    assert!(!result.contains("class"));
    assert!(!result.contains("wp-block"));
}

#[test]
fn strips_id_attribute() {
    let html = r#"<div id="main-content">Text</div>"#;
    let result = sanitize(html);
    assert!(result.contains("Text"));
    assert!(!result.contains("id="));
    assert!(!result.contains("main-content"));
}

#[test]
fn strips_data_attributes() {
    let html = r#"<div data-widget-id="123" data-tracking="abc">Text</div>"#;
    let result = sanitize(html);
    assert!(result.contains("Text"));
    assert!(!result.contains("data-"));
    assert!(!result.contains("widget"));
    assert!(!result.contains("tracking"));
}

#[test]
fn strips_event_handlers() {
    let html = r#"<img src="photo.jpg" onerror="alert('xss')" onclick="track()" onload="init()">"#;
    let result = sanitize(html);
    assert!(result.contains("src=\"photo.jpg\""));
    assert!(!result.contains("onerror"));
    assert!(!result.contains("onclick"));
    assert!(!result.contains("onload"));
    assert!(!result.contains("alert"));
}

#[test]
fn strips_img_srcset_sizes_width_height() {
    let html = r#"<img src="photo.jpg" srcset="photo-2x.jpg 2x" sizes="(max-width: 600px) 100vw" width="800" height="600">"#;
    let result = sanitize(html);
    assert!(result.contains("src=\"photo.jpg\""));
    assert!(!result.contains("srcset"));
    assert!(!result.contains("sizes"));
    assert!(!result.contains("width"));
    assert!(!result.contains("height"));
}

// === Additional dangerous elements ===

#[test]
fn removes_svg_elements() {
    let html = r#"<p>Content</p><svg><script>alert('xss')</script></svg>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("svg"));
    assert!(!result.contains("alert"));
}

#[test]
fn removes_base_elements() {
    let html = r#"<base href="https://evil.com"><p>Content</p>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("base"));
    assert!(!result.contains("evil.com"));
}

#[test]
fn removes_meta_http_equiv() {
    let html = r#"<meta http-equiv="refresh" content="0;url=https://evil.com"><p>Content</p>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("meta"));
    assert!(!result.contains("refresh"));
}

// === Empty element cleanup ===

#[test]
fn removes_empty_divs_and_spans() {
    let html = r#"<p>Content</p><div></div><span></span>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("<div"));
    assert!(!result.contains("<span"));
}

#[test]
fn removes_nested_empty_containers() {
    let html = r#"<p>Content</p><div><span></span></div>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("<div"));
    assert!(!result.contains("<span"));
}

#[test]
fn removes_containers_emptied_by_sanitization() {
    // After removing the script, the div becomes empty
    let html = r#"<p>Content</p><div><script>evil()</script></div>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    assert!(!result.contains("<div"));
    assert!(!result.contains("script"));
}

#[test]
fn preserves_br_hr_img() {
    let html = r#"<p>Line 1<br>Line 2</p><hr><img src="photo.jpg">"#;
    let result = sanitize(html);
    assert!(result.contains("<br>"));
    assert!(result.contains("<hr>"));
    assert!(result.contains("<img"));
}

#[test]
fn preserves_divs_with_content() {
    let html = r#"<div>Has content</div>"#;
    let result = sanitize(html);
    assert!(result.contains("<div>Has content</div>"));
}

// === Edge cases ===

#[test]
fn handles_empty_input() {
    let result = sanitize_rss_html_sync("", &SanitizeRssHtmlOptions::default());
    assert_eq!(result.html, "");
    assert!(result.first_image_src.is_none());
}

#[test]
fn handles_plain_text() {
    let result = sanitize("Just plain text, no HTML");
    assert!(result.contains("Just plain text, no HTML"));
}

#[test]
fn handles_unicode_content() {
    let html = r#"<p>日本語のコンテンツ 🎉 café résumé</p>"#;
    let result = sanitize(html);
    assert!(result.contains("日本語のコンテンツ"));
    assert!(result.contains("café"));
    assert!(result.contains("résumé"));
}

#[test]
fn handles_deeply_nested_dangerous_elements() {
    let html = r#"<div><div><div><script>deep_evil()</script></div></div></div><p>Safe</p>"#;
    let result = sanitize(html);
    assert!(result.contains("Safe"));
    assert!(!result.contains("script"));
    assert!(!result.contains("deep_evil"));
}
