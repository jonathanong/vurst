fn assert_parse_panics(val: &str, expected_msg: &str) {
    let result = std::panic::catch_unwind(|| super::parse_positive_usize("VAR", val));
    let err = result.expect_err("expected a panic");
    let msg = err.downcast_ref::<String>().map_or("", String::as_str);
    assert!(msg.contains(expected_msg), "unexpected panic: {msg}");
}

#[test]
fn returns_default_when_unset() {
    assert_eq!(super::env_usize("VURST_TEST_ENV_GUARANTEED_UNSET", 4), 4);
}

#[test]
fn parses_positive_integer() {
    assert_eq!(super::parse_positive_usize("VAR", "16"), 16);
}

#[test]
fn trims_whitespace() {
    assert_eq!(super::parse_positive_usize("VAR", "  8  "), 8);
}

#[test]
fn panics_on_zero() {
    assert_parse_panics("0", "must be a positive integer");
}

#[test]
fn panics_on_garbage() {
    assert_parse_panics("not-a-number", "must be a positive integer");
}

#[test]
fn spawn_blocking_runs_tasks() {
    let mut handles = Vec::with_capacity(50);
    for i in 0..50 {
        handles.push(super::spawn_blocking(move || i * 2));
    }
    let sum: usize = super::RUNTIME.block_on(async {
        let mut total = 0_usize;
        for h in handles {
            total += h.await.expect("task should join");
        }
        total
    });
    assert_eq!(sum, (0..50).map(|i| i * 2).sum::<usize>());
}

#[test]
fn await_blocking_maps_join_errors() {
    let result = super::RUNTIME.block_on(super::await_blocking(super::spawn_blocking(|| {
        panic!("boom");
    })));
    let error = result.expect_err("panic should become napi error");
    assert!(error.to_string().contains("Task failed"));
}
