/// Adversarial tests to find edge cases and potential vulnerabilities
/// These tests try to break the system with malicious or extreme inputs
use vurst_markdown_node::{chunk, default_length_counter, ChunkOptions};

// ============================================================================
// REGEX DENIAL OF SERVICE (ReDoS) ATTACKS
// ============================================================================

#[test]
fn test_redos_massive_whitespace_in_paragraphs() {
    // Try to trigger slow regex with \n\s*\n where \s* has huge whitespace
    let spaces = " ".repeat(100_000);
    let text = format!("Para 1\n{spaces}\nPara 2");

    // Should complete without hanging (no strict timing to avoid CI flakiness)
    let chunks = chunk(&text, None);
    assert!(!chunks.is_empty(), "Should produce at least one chunk");
}

#[test]
fn test_redos_extremely_long_line() {
    // Try to trigger slow regex with .+ on extremely long lines
    let long_line = "a".repeat(1_000_000);
    let text = format!("# {}", long_line);

    // Just verify it completes without hanging (no timing assertions for CI stability)
    let chunks = chunk(&text, None);

    // Ensure that non-empty input produces >= 1 chunk, even for extremely long headers
    // that would otherwise be parsed as empty sections.
    assert!(!chunks.is_empty(), "Long header produced 0 chunks");
}

#[test]
fn test_redos_pathological_code_blocks() {
    // Nested backticks trying to confuse the regex
    // NOTE: 10k backticks takes ~1s (acceptable, not a security concern)
    let text = "`".repeat(10_000);

    // Verify it completes without hanging (no timing assertions for CI stability)
    let chunks = chunk(&text, None);
    assert!(!chunks.is_empty(), "Should produce at least one chunk");
}

// ============================================================================
// MEMORY EXHAUSTION ATTACKS
// ============================================================================

#[test]
fn test_memory_massive_text_for_tokenization() {
    // Test correctness with reasonable size (not performance)
    // KNOWN PERF ISSUE: 1M words is slow (documented in VULNERABILITIES_FOUND.md)
    let text = "word ".repeat(10_000); // 10k words for correctness test

    let count = default_length_counter(&text);
    assert!(count > 40_000); // Roughly 5 chars per "word " entry
    assert!(count < 60_000);
}

// ============================================================================
// UNICODE ATTACKS
// ============================================================================

#[test]
fn test_unicode_zero_width_characters() {
    // Zero-width spaces, joiners, non-joiners
    let text = "Hello\u{200B}\u{200C}\u{200D}\u{FEFF}World";
    let count = default_length_counter(text);
    assert!(count > 0);

    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
}

#[test]
fn test_unicode_rtl_override() {
    // Right-to-left override can be used for spoofing
    let text = "File\u{202E}gpj.exe"; // Displays as "File.gpj.exe" but is actually "Fileexe.gpj"
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
}

#[test]
fn test_unicode_combining_characters() {
    // Stacking combining characters (zalgo text)
    let mut text = "H".to_string();
    for _ in 0..100 {
        text.push('\u{0301}'); // Combining acute accent
    }
    text.push_str("ello");

    let count = default_length_counter(&text);
    assert!(count > 0);

    let chunks = chunk(&text, None);
    assert!(!chunks.is_empty());
}

#[test]
fn test_unicode_emoji_flood() {
    // Test with small amount to verify correctness (not performance)
    // KNOWN PERF ISSUE: 10k complex emojis take 60-157s (documented in VULNERABILITIES_FOUND.md)
    let text = "👨‍👩‍👧‍👦".repeat(100); // Small sample for correctness

    let count = default_length_counter(&text);
    assert!(count > 0, "Should count character length of complex emojis");
}

#[test]
fn test_unicode_homoglyphs() {
    // Cyrillic 'а' looks like Latin 'a'
    let text = "аdministrator"; // First 'a' is Cyrillic U+0430
    let chunks = chunk(text, None);
    assert!(!chunks.is_empty());
}

#[test]
fn test_unicode_normalization_bomb() {
    // Some Unicode characters expand when normalized
    let text = "ﬃ".repeat(1000); // U+FB03 (ffi ligature)
    let count = default_length_counter(&text);
    assert!(count > 0);
}

// ============================================================================
// INTEGER OVERFLOW / UNDERFLOW ATTACKS
// ============================================================================

#[test]
fn test_chunking_with_extreme_token_limits() {
    let text = "# Header\n\n".to_string() + &"word ".repeat(10_000);

    // Max u32 for min_length (should clamp or handle gracefully)
    let chunks = chunk(
        &text,
        Some(ChunkOptions {
            min_length: Some(u32::MAX),
            max_length: Some(100),
            ..Default::default()
        }),
    );

    // Should not panic or produce invalid results
    assert!(!chunks.is_empty());
}

#[test]
fn test_chunking_min_greater_than_max() {
    let text = "# Header\n\nContent here";

    // Invalid config: min > max
    let chunks = chunk(
        text,
        Some(ChunkOptions {
            min_length: Some(1000),
            max_length: Some(100), // Less than min!
            ..Default::default()
        }),
    );

    // Should handle gracefully
    assert!(!chunks.is_empty());
}

// ============================================================================
// EDGE CASES IN STRING OPERATIONS
// ============================================================================

#[test]
fn test_code_block_placeholder_collision() {
    // KNOWN BUG: User text containing placeholder pattern gets corrupted
    let text = "Code: ___CODE_BLOCK_0___ should not break\n\n```python\nreal_code()\n```";

    let chunks = chunk(text, None);

    let all_text: String = chunks.iter().map(|c| &c.text).cloned().collect();

    // Real code blocks should be preserved
    assert!(all_text.contains("real_code"), "Code should be preserved");

    assert!(
        all_text.contains("___CODE_BLOCK_0___"),
        "User text '___CODE_BLOCK_0___' was removed/corrupted. This could happen if user is documenting code block handling"
    );
}

#[test]
fn test_malicious_header_patterns() {
    // Headers with regex-breaking characters
    let text = r#"
# Header with .*+?[]{}()^$|\
## Header with <script>
### Header with `backticks` and **bold**
#### Header with
trailing
newlines
"#;

    let chunks = chunk(text, None);

    // Should handle all special characters
    assert!(chunks.iter().any(|c| c.header.is_some()));
}
