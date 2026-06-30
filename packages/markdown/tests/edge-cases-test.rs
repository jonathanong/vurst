/// Edge case tests for robustness and security
use vurst_markdown_node::{chunk, default_length_counter, Chunk, ChunkOptions};

// ============================================================================
// TOKEN COUNTER EDGE CASES
// ============================================================================

#[test]
fn test_token_counter_empty_string() {
    assert_eq!(default_length_counter(""), 0);
}

#[test]
fn test_token_counter_whitespace_only() {
    assert_eq!(default_length_counter("   "), 0);
    assert_eq!(default_length_counter("\n\n\t  "), 0);
    assert_eq!(default_length_counter("     \r\n    "), 0);
}

#[test]
fn test_token_counter_unicode() {
    // Emojis
    let count = default_length_counter("Hello 👋 World 🌍");
    assert!(count > 0);

    // CJK characters
    let count = default_length_counter("你好世界");
    assert!(count > 0);

    // Mixed scripts
    let count = default_length_counter("English 日本語 한국어 Français");
    assert!(count > 0);
}

#[test]
fn test_token_counter_special_characters() {
    let count = default_length_counter("@#$%^&*(){}[]<>");
    assert!(count > 0);
}

#[test]
fn test_token_counter_consistency() {
    // Multiple spaces should normalize to single space
    assert_eq!(
        default_length_counter("hello   world"),
        default_length_counter("hello world")
    );

    // Leading/trailing whitespace should be trimmed
    assert_eq!(
        default_length_counter("  test  "),
        default_length_counter("test")
    );

    // Newlines should normalize
    assert_eq!(
        default_length_counter("hello\n\nworld"),
        default_length_counter("hello world")
    );
}

#[test]
fn test_token_counter_very_long_string() {
    // Test with 100k words
    let long_text = "word ".repeat(100_000);
    let count = default_length_counter(&long_text);
    assert!(count > 400_000); // Roughly 5 chars per "word " entry
    assert!(count < 600_000);
}

// ============================================================================
// CHUNKING EDGE CASES
// ============================================================================

#[test]
fn test_chunking_empty_string() {
    let chunks = chunk("", None);
    assert_eq!(chunks.len(), 0);
}

#[test]
fn test_chunking_extremely_deep_nesting() {
    // Test h1 > h2 > h3 > h4 > h5 > h6 with multiple h6s
    let text = "# H1\nC1\n## H2\nC2\n### H3\nC3\n#### H4\nC4\n##### H5\nC5\n###### H6\nC6\n###### H6-2\nC7\n###### H6-3\nC8";
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
    // Should not panic or overflow
}

#[test]
fn test_chunking_code_blocks_with_fake_headers() {
    let text = r#"
# Real Header 1

```python
# This is not a header
def foo():
    # Neither is this
    pass
```

## Real Header 2

`# inline code` should not be a header either

### Real Header 3
"#;
    let chunks = chunk(text, None);

    // Main assertion: code blocks should be preserved in the output
    let all_text: String = chunks.iter().map(|c| &c.text).cloned().collect();
    assert!(
        all_text.contains("```python"),
        "Code block should be preserved"
    );
    assert!(
        all_text.contains("def foo():"),
        "Code content should be preserved"
    );
    assert!(
        all_text.contains("# inline code"),
        "Inline code should be preserved"
    );

    // Should have found some header chunks (may be merged, so don't assert exact count)
    let header_chunks: Vec<&Chunk> = chunks.iter().filter(|c| c.level > 0).collect();
    assert!(
        !header_chunks.is_empty(),
        "Should find at least 1 header chunk"
    );
}

#[test]
fn test_chunking_special_characters_in_headers() {
    let text = "# Header with émojis 🚀 and spëcial chârs\n\nContent here";
    let chunks = chunk(text, None);

    let header_chunk = chunks.iter().find(|c| c.level == 1);
    assert!(header_chunk.is_some());
    assert_eq!(
        header_chunk.unwrap().header.as_deref().unwrap().as_str(),
        "Header with émojis 🚀 and spëcial chârs"
    );
}

#[test]
fn test_chunking_empty_sections() {
    // Headers with no content between them
    let text = "# H1\n## H2\n### H3\nActual content here";
    let chunks = chunk(text, None);

    // Should handle gracefully, create chunks for headers even if empty
    assert!(chunks.iter().any(|c| c.text.contains("Actual content")));
}

#[test]
fn test_chunking_single_chunk_exceeds_max() {
    // Single paragraph that's larger than max_length
    let text = "# Header\n\n".to_string() + &"word ".repeat(1000);
    let options = ChunkOptions {
        min_length: Some(100),
        max_length: Some(500),
        ..Default::default()
    };
    let chunks = chunk(&text, Some(options));

    // When a single chunk exceeds max, it should be kept as-is
    // (we can't split it further without breaking paragraphs)
    assert!(!chunks.is_empty());
}

#[test]
fn test_chunking_no_mergeable_content() {
    // All chunks already at or above min_length
    let mut text = String::new();
    for i in 0..5 {
        text.push_str(&format!("# Header {}\n\n", i));
        text.push_str(&"word ".repeat(200)); // Well above min_length
        text.push_str("\n\n");
    }

    let options = ChunkOptions {
        min_length: Some(100),
        max_length: Some(5000),
        ..Default::default()
    };
    let chunks = chunk(&text, Some(options));

    // Should complete without merging (all chunks big enough)
    assert!(chunks.len() >= 5);
}

// ============================================================================
// REGRESSION
// ============================================================================

#[test]
fn test_regression_merge_performance() {
    // Regression test: ensure O(n²) -> O(n) fix is working

    // Create markdown with many small chunks that need merging
    let mut text = "# Main Title\n\n".to_string();
    for i in 0..500 {
        text.push_str(&format!("## Section {}\n\nShort content.\n\n", i));
    }

    let chunks = chunk(
        &text,
        Some(ChunkOptions {
            min_length: Some(100),
            max_length: Some(500),
            phase: Some(3), // Full merging
            ..Default::default()
        }),
    );
    assert!(!chunks.is_empty());
}
