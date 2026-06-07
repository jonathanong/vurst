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
#[allow(unsafe_code)]
fn env_usize_falls_back_to_default_on_garbage() {
    // nextest runs each test in a separate process, so set_var is safe here
    unsafe { std::env::set_var("VURST_TEST_ENV_GARBAGE", "not-a-number") };
    assert_eq!(super::env_usize("VURST_TEST_ENV_GARBAGE", 4), 4);
}

#[test]
fn parses_positive_integer() {
    assert_eq!(super::parse_positive_usize("16"), Some(16));
}

#[test]
fn trims_whitespace() {
    assert_eq!(super::parse_positive_usize("  8  "), Some(8));
}

#[test]
fn returns_none_on_zero() {
    assert_eq!(super::parse_positive_usize("0"), None);
}

#[test]
fn returns_none_on_garbage() {
    assert_eq!(super::parse_positive_usize("not-a-number"), None);
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
