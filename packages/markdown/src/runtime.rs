// Owned tokio runtime for the N-API blocking thread pool.
//
// napi's `tokio_rt` feature ships its own runtime to drive the JS Promise/N-API
// async bridge. That runtime defaults to `max_blocking_threads = 512`, which is
// far too large for a typical Node worker process that wants a bounded thread
// budget.
//
// We keep napi's runtime for polling async N-API futures, but route every
// `spawn_blocking` call through this owned multi-thread runtime so we control
// the blocking-thread cap. Tokio's `JoinHandle` is runtime-agnostic at await
// time, so awaiting it from napi's runtime is safe.
//
// Tunable via env vars (positive integers):
//   - `RUST_TOKIO_WORKER_THREADS`        (default 2)
//   - `RUST_TOKIO_MAX_BLOCKING_THREADS`  (default 8)

use std::sync::LazyLock;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    let worker_threads = env_usize("RUST_TOKIO_WORKER_THREADS", 2).max(1);
    let max_blocking_threads = env_usize("RUST_TOKIO_MAX_BLOCKING_THREADS", 8).max(1);
    Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(max_blocking_threads)
        .thread_name("vurst-node")
        .enable_all()
        .build()
        .expect("BUG: failed to build vurst-node tokio runtime")
});

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    RUNTIME.spawn_blocking(f)
}

pub async fn await_blocking<R>(handle: JoinHandle<R>) -> napi::Result<R> {
    handle
        .await
        .map_err(|e| napi::Error::from_reason(format!("Task failed: {e}")))
}

pub(crate) fn parse_positive_usize(name: &str, raw: &str) -> usize {
    let trimmed = raw.trim();
    trimmed
        .parse::<usize>()
        .ok()
        .filter(|n| *n > 0)
        .unwrap_or_else(|| panic!("{name} must be a positive integer, got \"{raw}\""))
}

fn env_usize(name: &str, default: usize) -> usize {
    let Ok(raw) = std::env::var(name) else {
        return default;
    };
    parse_positive_usize(name, &raw)
}

#[cfg(test)]
mod tests;
