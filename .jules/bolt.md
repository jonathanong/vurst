## 2024-05-19 - Optimize ammonia string and vec cloning in HTML sanitizer
**Learning:** In Rust, when passing configuration objects containing strings and collections (like `String` and `Vec<String>`) into closures that require `'static` lifetimes (such as Ammonia's `attribute_filter`), deep cloning these structures on every invocation introduces significant `O(N)` allocation overhead. Wrapping them in `Arc<str>` and `Arc<[String]>` in the configuration struct allows cheap reference counting (`O(1)` cloning) instead.
**Action:** When designing N-API wrapper options that will be passed into `'static` closures, use `Arc<str>` instead of `String` and `Arc<[T]>` instead of `Vec<T>` to avoid deep cloning overhead, especially when the options are reused across many calls.

## 2024-05-18 - Avoid Unnecessary Allocations with str::replace
**Learning:** `str::replace()` always allocates a new `String` in Rust, even if the substring is not found in the target string. Chaining `.replace()` calls or calling it unconditionally on strings that rarely contain the target pattern leads to excessive memory allocations and degraded performance.
**Action:** Use `std::borrow::Cow` combined with `.contains()` checks to implement zero-allocation fast paths. Return `Cow::Borrowed` when no changes are needed, and only allocate `Cow::Owned` when a replacement actually occurs.
