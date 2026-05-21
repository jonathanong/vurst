//! AI-generated text detection backed by the `is-it-slop` crate.
//!
//! This module wraps the upstream detector in a small repo-local API so the
//! N-API layer and TypeScript moderation code can consume a stable result
//! shape without depending on the crate directly.

use std::sync::LazyLock;

use is_it_slop::{Classification, Predictor, MODEL_VERSION};

/// Singleton predictor — `Predictor::new()` loads an ONNX model so we amortize
/// that cost across all calls. `Predictor::new` returns `Predictor` directly
/// (not `Result`), so `.expect("BUG: …")` does not apply; any model-load
/// failure propagates as a panic inside the crate.
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
/// Returns an error when the threshold is outside `0.0..=1.0` or the upstream
/// detector fails to score the text.
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

    let prediction = PREDICTOR
        .predict(text)
        .map_err(slop_prediction_error_message)?;
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

fn slop_prediction_error_message(err: impl std::fmt::Display) -> String {
    format!("slop detector prediction failed: {err}")
}

#[cfg(test)]
mod tests;
