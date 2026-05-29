use std::fs;
use vurst_markdown_node::{chunk, ChunkOptions};

fn read_fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{}", name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("Failed to read fixture {path}: {err}"))
}

// Fixture-based tests
#[test]
fn test_frequent_flyer_basic_chunking() {
    let text = read_fixture("frequent-flyer.md");
    let chunks = chunk(&text, None);

    assert!(!chunks.is_empty(), "Should produce at least one chunk");
    assert!(
        chunks.iter().all(|c| c.length > 0),
        "All chunks should have character lengths"
    );
    assert!(
        chunks
            .iter()
            .all(|c| !c.breadcrumb.is_empty() || c.header.is_none()),
        "Chunks with headers should have breadcrumbs"
    );
    assert!(
        chunks.iter().all(|c| !c.text.is_empty()),
        "All chunks should have text"
    );
}

#[test]
fn test_frequent_flyer_preface_chunk() {
    let text = read_fixture("frequent-flyer.md");
    let chunks = chunk(&text, None);

    let first_chunk = &chunks[0];
    assert!(
        first_chunk.header.is_none() || first_chunk.text.to_lowercase().contains("back to top"),
        "First chunk should be preface or contain navigation"
    );
}

#[test]
fn test_frequent_flyer_header_hierarchy() {
    let text = read_fixture("frequent-flyer.md");
    let chunks = chunk(&text, None);

    let h2_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.breadcrumb.contains(" > "))
        .collect();
    assert!(
        !h2_chunks.is_empty(),
        "Should have chunks with nested headers"
    );

    for chunk in h2_chunks {
        let parts: Vec<_> = chunk.breadcrumb.split(" > ").collect();
        assert!(parts.len() >= 2, "Breadcrumb should have at least 2 levels");
        assert!(
            chunk.headers.len() >= 2,
            "Headers array should have at least 2 levels"
        );
    }
}

#[test]
fn test_frequent_flyer_max_length() {
    let text = read_fixture("frequent-flyer.md");
    let chunks = chunk(
        &text,
        Some(ChunkOptions {
            max_length: Some(2000),
            ..Default::default()
        }),
    );

    let over_limit = chunks.iter().filter(|c| c.length > 2000).count();
    assert!(
        over_limit < chunks.len() / 10,
        "Most chunks should respect maxLength"
    );
}

#[test]
fn test_world_of_hyatt_basic() {
    let text = read_fixture("world-of-hyatt.md");
    let chunks = chunk(&text, None);

    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|c| c.length > 0));
    assert!(chunks.iter().all(|c| !c.breadcrumb.is_empty()));
}

#[test]
fn test_world_of_hyatt_long_paragraphs() {
    let text = read_fixture("world-of-hyatt.md");
    let chunks = chunk(
        &text,
        Some(ChunkOptions {
            min_length: Some(200),
            max_length: Some(800),
            ..Default::default()
        }),
    );

    assert!(chunks.len() > 1);
    let valid_chunks: Vec<_> = chunks.iter().filter(|c| !c.text.is_empty()).collect();
    let reasonable: Vec<_> = valid_chunks
        .iter()
        .filter(|c| c.length >= 120 && c.length <= 1200)
        .collect();
    assert!(reasonable.len() >= valid_chunks.len() * 6 / 10);
}

// Simple text tests
#[test]
fn test_simple_h1_h2_h3() {
    let text = "\n# H1\n\nParagraph 1\n\n## H2\n\nParagraph 2\n\n## H3\n\nParagraph 3\n";
    let chunks = chunk(text, None);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].breadcrumb.as_ref().as_str(), "H1");
    assert!(chunks[0].length > 0);
}

#[test]
fn test_with_hashtags() {
    let text = "\n# H1\n\n[#123](https://example.com)\n";
    let chunks = chunk(text, None);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].breadcrumb.as_ref().as_str(), "H1");
    assert_eq!(chunks[0].text, "[#123](https://example.com)");
}

#[test]
fn test_no_header_with_title() {
    let text = "\nthis is a test article without a header\n\nthis is a test article without a header\n\nthis is a test article without a header\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            title: Some("test title".to_string()),
            ..Default::default()
        }),
    );
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].breadcrumb.as_ref().as_str(), "test title");
    assert_eq!(chunks[0].header, Some("test title".to_string()));
}

#[test]
fn test_nested_headers_h2_reset() {
    let text = "\n# H1\n\nParagraph 1\n\n## H2\n\nParagraph 2\n\n### H3\n\nParagraph 3\n\n## H2-2\n\nParagraph 4\n";
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
    let h2_chunk = chunks
        .iter()
        .find(|c| c.breadcrumb.contains("H2-2") || c.text.contains("Paragraph 4"));
    assert!(h2_chunk.is_some());
    assert!(h2_chunk.unwrap().breadcrumb.contains("H1"));
}

#[test]
fn test_whitespace_only() {
    let chunks = chunk("   \n\n  ", None);
    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_custom_min_max() {
    let text = "\n# H1\n\nShort paragraph.\n\n## H2\n\nAnother short paragraph.\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            min_length: Some(40),
            max_length: Some(400),
            ..Default::default()
        }),
    );
    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|c| c.length <= 400));
}

#[test]
fn test_multiple_paragraphs_same_header() {
    let text = "\n# H1\n\nFirst paragraph.\n\nSecond paragraph.\n\nThird paragraph.\n";
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
    let h1_chunks: Vec<_> = chunks
        .iter()
        .filter(|c| c.breadcrumb.as_ref() == "H1")
        .collect();
    assert!(!h1_chunks.is_empty());
}

// Code block tests
#[test]
fn test_code_blocks_no_fake_headers() {
    let text = "\n# Code Block Test\n\nParagraph before code.\n\n```\n# not a heading\nconst value = 42\n```\n\nParagraph after code.\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            min_length: Some(10),
            max_length: Some(500),
            ..Default::default()
        }),
    );

    let code_chunk = chunks.iter().find(|c| c.text.contains("```"));
    assert!(code_chunk.is_some());
    assert!(code_chunk.unwrap().text.contains("# not a heading"));
    assert!(!chunks
        .iter()
        .any(|c| c.breadcrumb.contains("not a heading")));
}

#[test]
fn test_fenced_code_with_language() {
    let text = "\n# JavaScript Example\n\n```javascript\nfunction test() {\n  console.log(\"# Not a header\")\n}\n```\n";
    let chunks = chunk(text, None);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("```javascript"));
    assert!(chunks[0].text.contains("function test()"));
}

#[test]
fn test_inline_code_hashtags() {
    let text = "\n# H1\n\nUse `git commit -m \"#123 fix\"` for commits.\n";
    let chunks = chunk(text, None);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("`git commit"));
}
