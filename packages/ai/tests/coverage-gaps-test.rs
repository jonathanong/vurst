use vurst_ai_node::detect_ai_generated_text;

#[test]
fn covers_slop_edge_paths() {
    let slop = detect_ai_generated_text("generic marketing paragraph", 0.0)
        .expect("threshold zero should classify as AI");
    assert!(slop.flagged);
}
