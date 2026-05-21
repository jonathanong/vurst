//! AI-generated text detection backed by the `is-it-slop` crate.
//!
//! This module wraps the upstream detector in a small repo-local API so the
//! N-API layer and TypeScript moderation code can consume a stable result
//! shape without depending on the crate directly.

use std::{
    any::{Any, TypeId},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::LazyLock,
};

use is_it_slop::{Classification, Predictor, UnifiedPrediction, MODEL_VERSION};

/// Singleton predictor for the upstream detector configuration.
static PREDICTOR: LazyLock<Predictor> = LazyLock::new(Predictor::new);

/// Result returned by [`detect_ai_generated_text`].
#[derive(Clone, Debug, PartialEq)]
pub struct SlopDetectionResult {
    /// True when the AI probability meets or exceeds the supplied threshold.
    pub flagged: bool,
    /// Detector-reported AI probability in the range 0.0..=1.0.
    pub confidence_score: f32,
    /// Threshold used to turn the probability into a boolean decision.
    pub confidence_threshold: f32,
    /// Final detector classification.
    pub classification: SlopClassification,
    /// Stable detector identifier stored in moderation JSON.
    pub detector: &'static str,
    /// Upstream model version reported by `is-it-slop`.
    pub detector_model_version: &'static str,
}

/// Final text classification produced by the detector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlopClassification {
    /// The detector classified the text as AI-generated.
    Ai,
    /// The detector classified the text as human-written.
    Human,
}

impl From<Classification> for SlopClassification {
    fn from(value: Classification) -> Self {
        match value {
            Classification::AI => Self::Ai,
            Classification::Human => Self::Human,
        }
    }
}

/// Detect whether the supplied text is likely AI-generated.
///
/// # Errors
///
/// Returns an error when the threshold is outside `0.0..=1.0`, the upstream
/// detector fails to score the text, or the upstream detector panics while
/// loading the model or predicting.
///
pub fn detect_ai_generated_text(
    text: &str,
    confidence_threshold: f32,
) -> Result<SlopDetectionResult, String> {
    if !(0.0..=1.0).contains(&confidence_threshold) {
        return Err(format!(
            "confidence_threshold must be between 0.0 and 1.0, got {confidence_threshold}"
        ));
    }

    let mut predict = || PREDICTOR.predict(text);

    build_slop_detection_result(confidence_threshold, run_slop_detector(&mut predict))
}

fn build_slop_detection_result(
    confidence_threshold: f32,
    prediction: Result<UnifiedPrediction, String>,
) -> Result<SlopDetectionResult, String> {
    let prediction = prediction?;
    let classification = prediction.prediction.classification(confidence_threshold);

    Ok(SlopDetectionResult {
        flagged: matches!(classification, Classification::AI),
        confidence_score: prediction.prediction.ai_probability(),
        confidence_threshold,
        classification: classification.into(),
        detector: "is-it-slop",
        detector_model_version: MODEL_VERSION,
    })
}

#[allow(clippy::needless_pass_by_value)]
fn slop_prediction_error_message(err: anyhow::Error) -> String {
    format!("slop detector prediction failed: {err}")
}

fn run_slop_detector(
    operation: &mut dyn FnMut() -> anyhow::Result<UnifiedPrediction>,
) -> Result<UnifiedPrediction, String> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(result) => result.map_err(slop_prediction_error_message),
        Err(payload) => Err(slop_detector_panic_error_message(payload)),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn slop_detector_panic_error_message(payload: Box<dyn Any + Send>) -> String {
    let payload_type = payload.as_ref().type_id();

    if payload_type == TypeId::of::<&str>() {
        let message = payload
            .downcast_ref::<&str>()
            .expect("panic payload type checked before downcast");
        return format!("slop detector unavailable: upstream detector panicked while loading the model or predicting: {message}");
    }

    if payload_type == TypeId::of::<String>() {
        let message = payload
            .downcast_ref::<String>()
            .expect("panic payload type checked before downcast");
        return format!("slop detector unavailable: upstream detector panicked while loading the model or predicting: {message}");
    }

    "slop detector unavailable: upstream detector panicked while loading the model or predicting: unknown panic".to_string()
}

#[cfg(test)]
mod tests;
