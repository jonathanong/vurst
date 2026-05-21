use super::env_usize;
use std::sync::Mutex;

// Serialize env mutation across tests in this module.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().expect("env lock should not be poisoned")
}

fn assert_env_panics(var: &'static str, val: &str, expected_msg: &str) {
    let _g = lock();
    unsafe { std::env::set_var(var, val) };
    let result = std::panic::catch_unwind(|| env_usize(var, 4));
    unsafe { std::env::remove_var(var) };
    let err = result.expect_err("expected a panic");
    let msg = err.downcast_ref::<String>().map_or("", String::as_str);
    assert!(msg.contains(expected_msg), "unexpected panic: {msg}");
}

#[test]
fn returns_default_when_unset() {
    let _g = lock();
    unsafe { std::env::remove_var("VURST_TEST_RUNTIME_DEFAULT") };
    assert_eq!(env_usize("VURST_TEST_RUNTIME_DEFAULT", 4), 4);
}

#[test]
fn parses_positive_integer() {
    let _g = lock();
    unsafe { std::env::set_var("VURST_TEST_RUNTIME_PARSE", "16") };
    assert_eq!(env_usize("VURST_TEST_RUNTIME_PARSE", 4), 16);
    unsafe { std::env::remove_var("VURST_TEST_RUNTIME_PARSE") };
}

#[test]
fn panics_on_zero() {
    assert_env_panics("VURST_TEST_RUNTIME_ZERO", "0", "must be a positive integer");
}

#[test]
fn panics_on_garbage() {
    assert_env_panics(
        "VURST_TEST_RUNTIME_GARBAGE",
        "not-a-number",
        "must be a positive integer",
    );
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
