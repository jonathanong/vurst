use super::*;

#[test]
fn formats_prediction_errors() {
    assert_eq!(
        slop_prediction_error_message(anyhow::anyhow!("boom")),
        "slop detector prediction failed: boom"
    );
}

#[test]
fn converts_upstream_string_panics_to_errors() {
    let mut predict = || panic!("missing model");
    let error = run_slop_detector(&mut predict)
        .err()
        .expect("upstream panics should become errors");

    assert_eq!(
        error,
        "slop detector unavailable: upstream detector panicked while loading the model or predicting: missing model"
    );
}

#[test]
fn converts_upstream_owned_string_panics_to_errors() {
    let mut predict = || std::panic::panic_any(String::from("corrupt model"));
    let error = run_slop_detector(&mut predict)
        .err()
        .expect("upstream panics should become errors");

    assert_eq!(
        error,
        "slop detector unavailable: upstream detector panicked while loading the model or predicting: corrupt model"
    );
}

#[test]
fn converts_upstream_non_string_panics_to_errors() {
    let mut predict = || std::panic::panic_any(42_u8);
    let error = run_slop_detector(&mut predict)
        .err()
        .expect("upstream panics should become errors");

    assert_eq!(
        error,
        "slop detector unavailable: upstream detector panicked while loading the model or predicting: unknown panic"
    );
}

#[test]
fn preserves_non_panic_prediction_errors() {
    let mut predict = || Err(anyhow::anyhow!("boom"));
    let error = run_slop_detector(&mut predict)
        .err()
        .expect("upstream errors should stay prediction errors");

    assert_eq!(error, "slop detector prediction failed: boom");
}

#[test]
fn detect_ai_generated_text_returns_prediction_errors() {
    let mut predict = || Err(anyhow::anyhow!("model unavailable"));
    let prediction = run_slop_detector(&mut predict);
    let error = build_slop_detection_result(0.95, prediction)
        .expect_err("upstream prediction errors should be returned");

    assert_eq!(error, "slop detector prediction failed: model unavailable");
}

#[test]
fn converts_upstream_ai_classification() {
    assert_eq!(
        SlopClassification::from(Classification::AI),
        SlopClassification::Ai
    );
}

#[test]
fn slop_detector_panic_error_message_handles_str() {
    let payload = catch_unwind(|| panic!("string slice error")).unwrap_err();
    let message = slop_detector_panic_error_message(payload);
    assert_eq!(
        message,
        "slop detector unavailable: upstream detector panicked while loading the model or predicting: string slice error"
    );
}

#[test]
fn slop_detector_panic_error_message_handles_string() {
    let payload = catch_unwind(|| std::panic::panic_any(String::from("owned string error"))).unwrap_err();
    let message = slop_detector_panic_error_message(payload);
    assert_eq!(
        message,
        "slop detector unavailable: upstream detector panicked while loading the model or predicting: owned string error"
    );
}

#[test]
fn slop_detector_panic_error_message_handles_unknown_type() {
    let payload = catch_unwind(|| std::panic::panic_any(123_i32)).unwrap_err();
    let message = slop_detector_panic_error_message(payload);
    assert_eq!(
        message,
        "slop detector unavailable: upstream detector panicked while loading the model or predicting: unknown panic"
    );
}
