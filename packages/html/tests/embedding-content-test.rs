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

#[test]
fn test_strips_reference_style_link_definitions() {
    let html = "<p>Here is some text.</p>\n<p>[ref]: https://example.com \"Title\"</p>";
    let result = html_to_embedding_text(html);
    assert!(
        result.contains("Here is some text."),
        "normal text should be kept"
    );
    assert!(
        !result.contains("[ref]:"),
        "reference definition should be stripped"
    );
    assert!(
        !result.contains("https://example.com"),
        "reference URL should be stripped"
    );
    assert!(
        !result.contains("Title"),
        "reference title should be stripped"
    );
}

#[test]
fn test_strips_reference_style_link_definitions_no_title() {
    let html = "<p>Here is some text.</p>\n<p>[ref]: https://example.com</p>";
    let result = html_to_embedding_text(html);
    assert!(
        result.contains("Here is some text."),
        "normal text should be kept"
    );
    assert!(
        !result.contains("[ref]:"),
        "reference definition should be stripped"
    );
    assert!(
        !result.contains("https://example.com"),
        "reference URL should be stripped"
    );
}

#[test]
fn test_whitespace_only() {
    let html = "   \n\t  ";
    assert_eq!(html_to_embedding_text(html), "");
}

#[test]
fn test_only_tags() {
    let html = "<div><span><br></span></div>";
    assert_eq!(html_to_embedding_text(html), "");
}

#[test]
fn test_invalid_html() {
    let html = "<div unclosed tag << > text";
    let result = html_to_embedding_text(html);
    // boilerstrip tries its best to parse it
    assert!(
        result.contains("text"),
        "text should be recovered from invalid html"
    );
}

#[test]
fn test_deeply_nested_html() {
    let mut html = String::new();
    // html5ever aborts nested parsing silently for depths > 200 to prevent stack overflows,
    // which results in empty output. This tests the maximum parsed depth (200).
    let depth = 200;
    for _ in 0..depth {
        html.push_str("<div>");
    }
    html.push_str("deep content");
    for _ in 0..depth {
        html.push_str("</div>");
    }
    let result = html_to_embedding_text(&html);
    assert!(
        result.contains("deep content"),
        "content should be retained at max depth"
    );
}

#[test]
fn test_huge_input() {
    let snippet = "<p>Some text with <a href=\"https://example.com\">a link</a> and <img src=\"img.jpg\" alt=\"an image\">.</p>\n";
    let html = snippet.repeat(10000); // Create a string ~1MB in size

    let result = html_to_embedding_text(&html);

    // Check that we successfully processed the huge input and output scaled accordingly
    assert!(result.len() > 100_000);
    assert_eq!(result.matches("Some text with").count(), 10_000);
    assert!(!result.contains("example.com"));
    assert!(!result.contains("img.jpg"));
    assert!(result.contains("Some text with"));
}

#[test]
fn test_strips_reference_style_link_definitions_with_brackets() {
    let html = "<p>Here is some text.</p>\n<p>[ref]: &lt;https://example.com&gt;</p>";
    let result = html_to_embedding_text(html);
    assert!(
        result.contains("Here is some text."),
        "normal text should be kept"
    );
    assert!(
        !result.contains("[ref]:"),
        "reference definition should be stripped"
    );
    assert!(
        !result.contains("https://example.com"),
        "reference URL should be stripped"
    );
}
