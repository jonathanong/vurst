#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(unused_doc_comments)]

// === N-API BRIDGE LAYER ===
//
// Thin translation layer between JavaScript and Rust. Mirrors each internal
// Rust type with a `#[napi]` type that can cross the JS/Rust boundary,
// converts between them via `From` impls, and runs CPU-intensive work on a
// bounded blocking pool (see `runtime`) so the Node.js event loop stays
// responsive.

use napi::bindgen_prelude::*;
use napi_derive::napi;

mod runtime;

pub mod slop_detection;

pub use slop_detection::{detect_ai_generated_text, SlopClassification, SlopDetectionResult};

/// Maximum input size for all functions (10 MiB).
const SANITIZE_MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;

// ============================================================================
// detect_ai_generated_text
// ============================================================================

#[napi(object)]
#[derive(Clone, Debug)]
pub struct NapiSlopDetectionResult {
    pub flagged: bool,
    pub confidence_score: f64,
    pub confidence_threshold: f64,
    pub classification: String,
    pub detector: String,
    pub detector_model_version: String,
}

impl From<slop_detection::SlopDetectionResult> for NapiSlopDetectionResult {
    fn from(result: slop_detection::SlopDetectionResult) -> Self {
        Self {
            flagged: result.flagged,
            confidence_score: f64::from(result.confidence_score),
            confidence_threshold: f64::from(result.confidence_threshold),
            classification: match result.classification {
                slop_detection::SlopClassification::Ai => "ai".to_string(),
                slop_detection::SlopClassification::Human => "human".to_string(),
            },
            detector: result.detector.to_string(),
            detector_model_version: result.detector_model_version.to_string(),
        }
    }
}

#[napi(js_name = "detectAiGeneratedText")]
pub async fn detect_ai_generated_text_napi(
    text: Buffer,
    confidence_threshold: Option<f64>,
) -> Result<NapiSlopDetectionResult> {
    if text.len() > SANITIZE_MAX_INPUT_BYTES {
        return Err(Error::from_reason(format!(
            "Input too large: {} bytes (max {SANITIZE_MAX_INPUT_BYTES} bytes)",
            text.len()
        )));
    }

    let decoded = String::from_utf8(text.into())
        .map_err(|e| Error::from_reason(format!("Invalid UTF-8 in text: {e}")))?;
    #[allow(clippy::cast_possible_truncation)] // 0.0..=1.0 fits losslessly in f32
    let threshold = confidence_threshold.unwrap_or(0.95) as f32;

    runtime::await_blocking_result(runtime::spawn_blocking(move || {
        slop_detection::detect_ai_generated_text(&decoded, threshold)
            .map(std::convert::Into::into)
            .map_err(Error::from_reason)
    }))
    .await
}
