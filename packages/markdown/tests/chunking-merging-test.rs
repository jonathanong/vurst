use std::fs;
use vurst_markdown_node::{chunk, ChunkOptions};

fn read_fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{}", name);
    fs::read_to_string(&path).expect(&format!("Failed to read fixture: {}", path))
}

// Phase tests
#[test]
fn test_phase1_returns_early() {
    let text = "\n# H1\n\nParagraph 1\n\n## H2\n\nParagraph 2\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            phase: Some(1),
            ..Default::default()
        }),
    );
    assert!(!chunks.is_empty());
    assert!(chunks.iter().any(|c| c.breadcrumb == "H1"));
}

#[test]
fn test_phase2_after_first_merge() {
    let text = "\n# H1\n\nParagraph 1\n\n## H2\n\nParagraph 2\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            phase: Some(2),
            ..Default::default()
        }),
    );
    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|c| c.length > 0));
}

// Merging tests
#[test]
fn test_paragraphs_merge_under_min_length() {
    let text = "\n# Merge Example\n\nShort one.\n\nShort two.\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            min_length: Some(1000),
            max_length: Some(10000),
            ..Default::default()
        }),
    );

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("Short one.\n\nShort two."));
}

#[test]
fn test_parent_absorbs_child() {
    let text = "\n# Parent Header\n\nParent paragraph.\n\n## Child Header\n\nChild paragraph.\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            min_length: Some(1000),
            max_length: Some(10000),
            ..Default::default()
        }),
    );

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("## Child Header"));
    assert!(chunks[0].text.contains("Child paragraph."));
}

#[test]
fn test_h6_merges_into_parent() {
    let text = "\n# Root\n\nRoot paragraph.\n\n###### Deep Child\n\nDeep child paragraph.\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            min_length: Some(1000),
            max_length: Some(10000),
            ..Default::default()
        }),
    );

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.contains("###### Deep Child"));
    assert!(chunks[0].text.contains("Deep child paragraph"));
}

// Edge cases
#[test]
fn test_empty_paragraphs() {
    let text = "\n# H1\n\n\n\n## H2\n\nContent after empty paragraphs.\n";
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|c| !c.text.is_empty()));
}

#[test]
fn test_very_long_paragraph() {
    let long_text = "This is a very long paragraph. ".repeat(100);
    let text = format!("\n# H1\n\n{}\n", long_text);
    let chunks = chunk(
        &text,
        Some(ChunkOptions {
            max_length: Some(200),
            ..Default::default()
        }),
    );
    assert!(!chunks.is_empty());
    assert!(chunks.iter().any(|c| c.text.len() > 100));
}

#[test]
fn test_headers_without_content() {
    let text = "\n# H1\n\n## H2\n\n### H3\n\n## H2-2\n\nContent here.\n";
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
    let h2_chunk = chunks.iter().find(|c| c.text.contains("Content here"));
    assert!(h2_chunk.is_some());
    assert!(h2_chunk.unwrap().breadcrumb.contains("H1"));
}

#[test]
fn test_deep_nesting_h1_to_h6() {
    let text = "\n# H1\n\n## H2\n\n### H3\n\n#### H4\n\n##### H5\n\n###### H6\n\nContent at deepest level.\n";
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
    let deep_chunk = chunks.iter().find(|c| c.text.contains("deepest level"));
    assert!(deep_chunk.is_some());
    assert!(deep_chunk.unwrap().breadcrumb.contains("H1"));
}

#[test]
fn test_special_characters_in_headers() {
    let text = "\n# Header with \"quotes\" and 'apostrophes'\n\nContent here.\n\n## Sub-header with $dollar$ and %percent%\n\nMore content.\n";
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
    assert!(chunks.iter().any(|c| c.breadcrumb.contains("quotes")));
}

#[test]
fn test_content_before_first_header_separate_chunk() {
    let text = "\nIntro paragraph before any headings.\n\n# First Header\n\nContent under the first header.\n";
    let chunks = chunk(text, None);
    assert!(chunks.len() >= 2);
    let preface = &chunks[0];
    assert_eq!(preface.breadcrumb, "");
    assert!(preface.header.is_none());
    assert!(preface.text.contains("Intro paragraph"));
    assert!(chunks.iter().any(|c| c.breadcrumb == "First Header"));
}

#[test]
fn test_header_regex_valid_only() {
    let text = "\n# Valid H1\n\nThis has a #hashtag in the middle.\n\n##Invalid header without space\n\n###### Valid H6\n\nContent continues.\n\n####### Invalid H7 (too many hashes)\n";
    let chunks = chunk(text, None);

    assert!(chunks
        .iter()
        .any(|c| c.header == Some("Valid H1".to_string())));
    assert!(!chunks
        .iter()
        .any(|c| c.header.as_ref().map_or(false, |h| h.contains("##Invalid"))));
    assert!(chunks.iter().any(|c| c.text.contains("#hashtag")));
}

#[test]
fn test_merging_respects_token_limits() {
    let text = format!(
        "\n# H1\n\n{}\n\n{}\n\n{}\n",
        "Short. ".repeat(20),
        "Short. ".repeat(20),
        "Short. ".repeat(20)
    );

    let chunks_small = chunk(
        &text,
        Some(ChunkOptions {
            max_length: Some(150),
            ..Default::default()
        }),
    );
    let chunks_large = chunk(
        &text,
        Some(ChunkOptions {
            max_length: Some(2000),
            ..Default::default()
        }),
    );

    assert!(chunks_small.len() >= chunks_large.len());
    assert!(chunks_small.iter().all(|c| c.length <= 150));
    assert!(chunks_large.iter().all(|c| c.length <= 2000));
}

#[test]
fn test_hierarchical_merging_preserves_structure() {
    let text = "\n# H1\n\nContent 1\n\n## H2\n\nContent 2\n\n### H3\n\nContent 3\n";
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            min_length: Some(40),
            max_length: Some(4000),
            ..Default::default()
        }),
    );

    assert!(chunks.len() <= 2);
    if chunks.len() == 1 {
        let main_chunk = &chunks[0];
        assert!(main_chunk.text.contains("## H2") || main_chunk.text.contains("Content 2"));
    }
}

#[test]
fn test_frequent_flyer_phase1_unmerged() {
    let text = read_fixture("frequent-flyer.md");
    let phase1_chunks = chunk(
        &text,
        Some(ChunkOptions {
            phase: Some(1),
            ..Default::default()
        }),
    );
    let full_chunks = chunk(&text, None);

    assert!(
        phase1_chunks.len() >= full_chunks.len(),
        "Phase 1 should have equal or more chunks"
    );
}
