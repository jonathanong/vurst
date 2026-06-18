use vurst_markdown_node::{chunk, ChunkOptions};

#[test]
fn test_empty_text() {
    let chunks = chunk("", None);
    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_no_headers() {
    let text = "This is a simple paragraph without any headers.";
    let chunks = chunk(text, None);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].level, 0);
    assert_eq!(chunks[0].text, text);
}

#[test]
fn test_single_header() {
    let text = "# Header 1\n\nThis is content under header 1.";
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
    assert_eq!(chunks[0].level, 1);
    assert_eq!(
        chunks[0].header.as_deref().map(|s| s.as_str()),
        Some("Header 1")
    );
}

#[test]
fn test_multiple_headers() {
    let text = "# Header 1\n\nContent 1.\n\n## Header 2\n\nContent 2.";
    let options = ChunkOptions {
        phase: Some(1),
        ..Default::default()
    };
    let chunks = chunk(text, Some(options));
    assert!(chunks.len() >= 2);
    assert!(chunks
        .iter()
        .any(|c| c.header.as_deref().map(|s| s.as_str()) == Some("Header 1")));
    assert!(chunks
        .iter()
        .any(|c| c.header.as_deref().map(|s| s.as_str()) == Some("Header 2")));
}

#[test]
fn test_breadcrumb_building() {
    let text = "# H1\n\nC1\n\n## H2\n\nC2\n\n### H3\n\nC3";
    let chunks = chunk(text, None);

    let h1_chunk = chunks
        .iter()
        .find(|c| c.header.as_deref().map(|s| s.as_str()) == Some("H1"));
    let h2_chunk = chunks
        .iter()
        .find(|c| c.header.as_deref().map(|s| s.as_str()) == Some("H2"));
    let h3_chunk = chunks
        .iter()
        .find(|c| c.header.as_deref().map(|s| s.as_str()) == Some("H3"));

    if let Some(c) = h1_chunk {
        assert_eq!(c.breadcrumb.as_ref().as_str(), "H1");
    }
    if let Some(c) = h2_chunk {
        assert_eq!(c.breadcrumb.as_ref().as_str(), "H1 > H2");
    }
    if let Some(c) = h3_chunk {
        assert_eq!(c.breadcrumb.as_ref().as_str(), "H1 > H2 > H3");
    }
}

#[test]
fn test_code_block_preservation() {
    let text = "# Header\n\n```rust\nfn main() {}\n```\n\nMore text.";
    let chunks = chunk(text, None);
    let combined: String = chunks.iter().map(|c| c.text.clone()).collect();
    assert!(combined.contains("```rust"));
    assert!(combined.contains("fn main() {}"));
}

#[test]
fn test_phase_1_only() {
    let text = "# H1\n\na\n\n# H1\n\nb";
    let options = ChunkOptions {
        phase: Some(1),
        ..Default::default()
    };
    let chunks = chunk(text, Some(options));
    assert!(chunks.len() >= 2);
}

#[test]
fn test_phase_2_merge_same_breadcrumb() {
    let text = "# H1\n\na\n\n# H1\n\nb";
    let options = ChunkOptions {
        phase: Some(2),
        min_length: Some(128),
        max_length: Some(1000),
        ..Default::default()
    };
    let chunks = chunk(text, Some(options));
    assert!(
        chunks.len()
            < chunk(
                text,
                Some(ChunkOptions {
                    phase: Some(1),
                    ..Default::default()
                })
            )
            .len()
    );
}

#[test]
fn test_title_option() {
    let text = "Content before headers.";
    let options = ChunkOptions {
        title: Some("My Title".to_string()),
        ..Default::default()
    };
    let chunks = chunk(text, Some(options));
    assert_eq!(chunks[0].headers[0], Some("My Title".to_string()));
}

#[test]
fn test_min_max_length() {
    let text = "# H1\n\na\n\n# H2\n\nb";
    let options = ChunkOptions {
        min_length: Some(1),
        max_length: Some(10),
        ..Default::default()
    };
    let chunks = chunk(text, Some(options));
    for chunk in chunks {
        assert!(chunk.length <= 10);
    }
}
