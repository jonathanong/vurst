use vurst_ai_node::detect_ai_generated_text;

#[test]
fn covers_slop_edge_paths() {
    let result = detect_ai_generated_text("generic marketing paragraph", 0.0);
    match result {
        Ok(slop) => assert!(slop.flagged, "threshold zero should classify as AI"),
        Err(e) => {
            // Depending on the environment, onnxruntime dylib might not be available
            // so we shouldn't fail the test if the error is about loading the dylib.
            if !e.contains("Failed to load ONNX Runtime dylib") {
                panic!("Expected a successful classification or an ONNX loading error, but got: {}", e);
            }
        }
    }
}
