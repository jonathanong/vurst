use vurst_html_node::html_to_embedding_text;

#[test]
fn test_strips_link_url_keeps_text() {
    let html = "<p>Visit <a href=\"https://example.com\">our website</a> for more info.</p>";
    let result = html_to_embedding_text(html);
    assert!(result.contains("our website"), "link text should be kept");
    assert!(
        result.contains("for more info"),
        "surrounding text should be kept"
    );
    assert!(
        !result.contains("https://example.com"),
        "URL should be stripped"
    );
    assert!(
        !result.contains("href"),
        "HTML attribute should be stripped"
    );
}

#[test]
fn test_strips_image_url_keeps_alt_text() {
    let html = "<p><img src=\"https://example.com/photo.jpg\" alt=\"A beautiful sunset\"></p>";
    let result = html_to_embedding_text(html);
    assert!(
        result.contains("A beautiful sunset"),
        "alt text should be kept"
    );
    assert!(
        !result.contains("https://example.com"),
        "image URL should be stripped"
    );
    assert!(
        !result.contains("src="),
        "HTML attribute should be stripped"
    );
}

#[test]
fn test_strips_image_with_empty_alt() {
    let html = "<p><img src=\"https://example.com/img.png\" alt=\"\"></p>";
    let result = html_to_embedding_text(html);
    assert!(
        !result.contains("https://example.com"),
        "image URL should be stripped"
    );
}

#[test]
fn test_strips_image_without_alt() {
    let html = "<p><img src=\"https://example.com/img.png\"></p>";
    let result = html_to_embedding_text(html);
    assert!(
        !result.contains("https://example.com"),
        "image URL should be stripped"
    );
}

#[test]
fn test_preserves_heading_text() {
    let html = "<h1>Article Title</h1><p>Some body content here.</p>";
    let result = html_to_embedding_text(html);
    assert!(result.contains("Article Title"), "heading should be kept");
    assert!(
        result.contains("Some body content here"),
        "body text should be kept"
    );
}

#[test]
fn test_preserves_paragraph_text() {
    let html = "<p>First paragraph.</p><p>Second paragraph.</p>";
    let result = html_to_embedding_text(html);
    assert!(
        result.contains("First paragraph"),
        "first para should be kept"
    );
    assert!(
        result.contains("Second paragraph"),
        "second para should be kept"
    );
}

#[test]
fn test_empty_html() {
    assert_eq!(html_to_embedding_text(""), "");
}

#[test]
fn test_plain_text_passthrough() {
    let result = html_to_embedding_text("hello world");
    assert!(
        result.contains("hello world"),
        "plain text should pass through"
    );
}

#[test]
fn test_multiple_links_stripped() {
    let html =
        "<p>See <a href=\"https://a.com\">link A</a> and <a href=\"https://b.com\">link B</a>.</p>";
    let result = html_to_embedding_text(html);
    assert!(result.contains("link A"), "link A text should be kept");
    assert!(result.contains("link B"), "link B text should be kept");
    assert!(
        !result.contains("https://a.com"),
        "URL A should be stripped"
    );
    assert!(
        !result.contains("https://b.com"),
        "URL B should be stripped"
    );
}

#[test]
fn test_image_before_link_in_same_content() {
    // Ensures image regex runs before link regex to avoid link pattern consuming image alt text
    let html = "<p><img src=\"img.jpg\" alt=\"photo\"> and <a href=\"url\">text</a></p>";
    let result = html_to_embedding_text(html);
    assert!(result.contains("photo"), "image alt should be kept");
    assert!(result.contains("text"), "link text should be kept");
    assert!(!result.contains("img.jpg"), "image src should be stripped");
    assert!(!result.contains("url"), "link url should be stripped");
}
