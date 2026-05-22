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
#[allow(unsafe_code)]
fn reads_set_env_var() {
    // nextest runs each test in a separate process, so set_var is safe here
    unsafe { std::env::set_var("VURST_TEST_ENV_SET_TO_16", "16") };
    assert_eq!(super::env_usize("VURST_TEST_ENV_SET_TO_16", 4), 16);
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

#[test]
fn await_blocking_result_flattens_inner_results() {
    let ok = super::RUNTIME.block_on(super::await_blocking_result(super::spawn_blocking(|| {
        napi::Result::Ok(7)
    })));
    assert_eq!(ok.expect("inner ok should pass through"), 7);

    let inner_error =
        super::RUNTIME.block_on(super::await_blocking_result(super::spawn_blocking(|| {
            napi::Result::<usize>::Err(napi::Error::from_reason("inner"))
        })));
    assert!(inner_error
        .expect_err("inner error should pass through")
        .to_string()
        .contains("inner"));

    let join_error = super::RUNTIME.block_on(super::await_blocking_result(super::spawn_blocking(
        || -> napi::Result<usize> {
            panic!("boom");
        },
    )));
    assert!(join_error
        .expect_err("join error should pass through")
        .to_string()
        .contains("Task failed"));
}
