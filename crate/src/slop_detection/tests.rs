use super::*;

#[test]
fn formats_prediction_errors() {
    assert_eq!(
        slop_prediction_error_message("boom"),
        "slop detector prediction failed: boom"
    );
}
