use vurst_ai_node::detect_ai_generated_text;

#[test]
fn detect_ai_generated_text_returns_bounded_scores_and_metadata() {
    let result = detect_ai_generated_text(
        "This is a short paragraph about earning airline miles from travel spending.",
        0.95,
    );

    match result {
        Ok(result) => {
            assert_eq!(result.detector, "is-it-slop");
            assert!(!result.detector_model_version.is_empty());
            assert!((0.0..=1.0).contains(&result.confidence_score));
            assert_eq!(result.confidence_threshold, 0.95);
            assert_eq!(
                result.flagged,
                result.confidence_score >= result.confidence_threshold
            );
        }
        Err(e) => {
            if e.contains("Failed to load ONNX Runtime dylib") {
                eprintln!("Skipping AI behavior test: ONNX Runtime dylib unavailable");
                return;
            }

            panic!("detector should succeed, but failed with: {}", e);
        }
    }
}

#[test]
fn detect_ai_generated_text_rejects_invalid_thresholds() {
    let error = detect_ai_generated_text("threshold validation", 1.1)
        .expect_err("thresholds above 1.0 should be rejected");

    assert!(error.contains("confidence_threshold must be between 0.0 and 1.0"));
}
