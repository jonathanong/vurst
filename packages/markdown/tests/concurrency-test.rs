use std::sync::Arc;
use std::thread;
/// Concurrency tests to verify thread safety
/// These tests ensure the code is safe when called from multiple threads simultaneously
use vurst_markdown_node::{chunk, default_length_counter, ChunkOptions};

// ============================================================================
// CONCURRENT TOKEN COUNTING
// ============================================================================

#[test]
fn test_concurrent_token_counting_basic() {
    // Spawn 10 threads that all count tokens simultaneously
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                let text = format!("Test string number {}", i);
                default_length_counter(&text)
            })
        })
        .collect();

    // Wait for all threads and verify results
    for handle in handles {
        let count = handle.join().expect("Thread panicked");
        assert!(count > 0, "Token count should be > 0");
    }
}

#[test]
fn test_concurrent_token_counting_shared_data() {
    // Multiple threads counting the same text
    let text = Arc::new("Hello world! This is a test string.".to_string());

    let handles: Vec<_> = (0..50)
        .map(|_| {
            let text_clone = Arc::clone(&text);
            thread::spawn(move || default_length_counter(&text_clone))
        })
        .collect();

    // All threads should get the same result
    let results: Vec<usize> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect();

    // Verify all results are identical
    let first = results[0];
    for &result in &results {
        assert_eq!(
            result, first,
            "Token counts should be consistent across threads"
        );
    }
}

#[test]
fn test_concurrent_token_counting_different_inputs() {
    // Many threads with different inputs
    let inputs = vec![
        "Short text",
        "A much longer text with many more words to count tokens for",
        "Unicode: 你好世界 🚀",
        "Special chars: !@#$%^&*()",
        "   Whitespace   test   ",
    ];

    let handles: Vec<_> = inputs
        .into_iter()
        .map(|text| {
            let text = text.to_string();
            thread::spawn(move || (text.clone(), default_length_counter(&text)))
        })
        .collect();

    // All threads should complete successfully
    for handle in handles {
        let (text, count) = handle.join().expect("Thread panicked");
        assert!(count > 0, "Token count should be > 0 for '{}'", text);
    }
}

// ============================================================================
// CONCURRENT CHUNKING
// ============================================================================

#[test]
fn test_concurrent_chunking_basic() {
    let text = Arc::new("# Header\n\nContent here\n\n## Subheader\n\nMore content".to_string());

    let handles: Vec<_> = (0..20)
        .map(|_| {
            let text_clone = Arc::clone(&text);
            thread::spawn(move || chunk(&text_clone, None))
        })
        .collect();

    // All threads should produce the same number of chunks
    let results: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("Thread panicked"))
        .collect();

    let first_len = results[0].len();
    for chunks in &results {
        assert_eq!(
            chunks.len(),
            first_len,
            "All threads should produce same number of chunks"
        );
    }
}

#[test]
fn test_concurrent_chunking_different_inputs() {
    let texts = vec![
        "# H1\n\nC1",
        "## H2\n\nC2\n\n### H3\n\nC3",
        "No headers at all",
        "# Header\n\n```code block```\n\nText",
    ];

    let handles: Vec<_> = texts
        .into_iter()
        .map(|text| {
            let text = text.to_string();
            thread::spawn(move || chunk(&text, None))
        })
        .collect();

    // All threads should complete without panicking
    for handle in handles {
        let chunks = handle.join().expect("Thread panicked");
        assert!(!chunks.is_empty(), "Should produce at least one chunk");
    }
}

#[test]
fn test_concurrent_chunking_with_options() {
    let text = Arc::new("# H1\n\n".to_string() + &"word ".repeat(100));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let text_clone = Arc::clone(&text);
            thread::spawn(move || {
                chunk(
                    &text_clone,
                    Some(ChunkOptions {
                        min_length: Some(50),
                        max_length: Some(200),
                        ..Default::default()
                    }),
                )
            })
        })
        .collect();

    // All threads should complete successfully
    for handle in handles {
        let chunks = handle.join().expect("Thread panicked");
        assert!(!chunks.is_empty());
    }
}

// ============================================================================
// STRESS TESTS (Many Threads)
// ============================================================================

#[test]
fn test_stress_many_threads_mixed_operations() {
    // Spawn 100 threads doing various operations
    let handles: Vec<_> = (0..100)
        .map(|i| {
            thread::spawn(move || {
                match i % 2 {
                    0 => {
                        // Character-length counting
                        default_length_counter(&format!("text number {}", i));
                    }
                    _ => {
                        // Chunking
                        let text = format!("# Header {}\n\nContent {}", i, i);
                        let _ = chunk(&text, None);
                    }
                }
            })
        })
        .collect();

    // All threads should complete without panics
    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
}

#[test]
fn test_stress_rapid_creation_destruction() {
    // Rapidly create and destroy threads (stress LazyLock initialization)
    for _ in 0..10 {
        let handles: Vec<_> = (0..20)
            .map(|_| thread::spawn(|| default_length_counter("test")))
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }
    }
}

// ============================================================================
// REGRESSION: Thread Safety of Singletons
// ============================================================================

#[test]
fn test_singleton_initialization_race() {
    // Try to trigger race conditions during LazyLock initialization
    // by spawning threads immediately
    let handles: Vec<_> = (0..100)
        .map(|_| {
            thread::spawn(|| {
                // First access to WHITESPACE_REGEX LazyLock might have race condition
                default_length_counter("initialize")
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

// ============================================================================
// DETERMINISM: Same Input = Same Output (Across Threads)
// ============================================================================

#[test]
fn test_deterministic_across_threads() {
    let test_cases: Vec<(&str, &str, Option<()>)> = vec![
        ("token", "hello world", None),
        ("token", "🚀 emoji test", None),
        ("chunk", "# H1\n\nContent", None),
        ("chunk", "## H2\n\n### H3\n\nNested", None),
    ];

    for (op, input, _) in test_cases {
        // Run the same operation in 20 threads
        let handles: Vec<_> = (0..20)
            .map(|_| {
                let op = op.to_string();
                let input = input.to_string();
                thread::spawn(move || match op.as_str() {
                    "token" => (default_length_counter(&input), vec![]),
                    "chunk" => {
                        let chunks = chunk(&input, None);
                        (chunks.len(), chunks)
                    }
                    _ => (0, vec![]),
                })
            })
            .collect();

        // Collect all results
        let results: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().expect("Thread panicked"))
            .collect();

        // Verify all threads got the same result
        let first = &results[0];
        for result in &results[1..] {
            assert_eq!(
                result.0, first.0,
                "Determinism violation: different threads got different results for same input"
            );
        }
    }
}
