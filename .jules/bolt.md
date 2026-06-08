## 2024-05-19 - Optimize ammonia string and vec cloning in HTML sanitizer
**Learning:** In Rust, when passing configuration objects containing strings and collections (like `String` and `Vec<String>`) into closures that require `'static` lifetimes (such as Ammonia's `attribute_filter`), deep cloning these structures on every invocation introduces significant `O(N)` allocation overhead. Wrapping them in `Arc<str>` and `Arc<[String]>` in the configuration struct allows cheap reference counting (`O(1)` cloning) instead.
**Action:** When designing N-API wrapper options that will be passed into `'static` closures, use `Arc<str>` instead of `String` and `Arc<[T]>` instead of `Vec<T>` to avoid deep cloning overhead, especially when the options are reused across many calls.

## 2024-05-18 - Avoid Unnecessary Allocations with str::replace
**Learning:** `str::replace()` always allocates a new `String` in Rust, even if the substring is not found in the target string. Chaining `.replace()` calls or calling it unconditionally on strings that rarely contain the target pattern leads to excessive memory allocations and degraded performance.
**Action:** Use `std::borrow::Cow` combined with `.contains()` checks to implement zero-allocation fast paths. Return `Cow::Borrowed` when no changes are needed, and only allocate `Cow::Owned` when a replacement actually occurs.

## 2023-10-27 - Optimize HTML string escaping with Cow<'a, str>
**Learning:** Chained `.replace()` calls for HTML escaping allocate and copy repeatedly on each replacement. A single-pass scan is faster and avoids unnecessary allocation for texts that require no escaping.
**Action:** Replace multiple `.replace` calls with a manual string scan using `char_indices` that builds an output buffer only when an escape character is encountered, returns `Cow<'_, str>`, and appends borrowed text directly when no replacements are needed.

## 2024-05-22 - Optimize string searching and building
**Learning:** In tight loops over text, when searching for ASCII-only targets (like '<', '>', '&', '"'), using `char_indices` incurs unnecessary UTF-8 decoding overhead. A byte-slice scan (`position` + offset jumps) can skip ahead to likely matches efficiently while preserving overall O(n) behavior, since ASCII bytes never overlap with multibyte UTF-8 sequences in valid input. Additionally, the `write!` macro introduces measurable overhead compared to direct `push`/`push_str` calls for simple string building.
**Action:** Use byte-slice scans with `as_bytes()` for ASCII boundary detection before falling back to scalar decoding where needed. Use `push()` and `push_str()` directly on `String` buffers instead of the `write!` macro for better performance.

## 2024-05-26 - Optimize HTML tag stripping
**Learning:** `strip_html_markup` was manually scanning byte-by-byte and character-by-character to find the next HTML tag opening angle bracket (`<`).
**Action:** Use byte-slice scans with `position` (`memchr`) to fast-forward to the next `<` character before processing tags, which is significantly faster for content with few tags.

## 2024-05-14 - Allocation-Free Fast Paths in URL Sanitization
**Learning:** URL sanitization loops in hot paths (like markdown processing) often perform unnecessary allocations by filtering safe URLs (e.g., stripping whitespace). We discovered that by first scanning the raw bytes for the presence of invalid characters (`b.is_ascii_whitespace()` or `b.is_ascii_control()`), we can avoid expensive UTF-8 parsing and `String` allocations entirely for the vast majority of valid URLs.
**Action:** When writing filters or sanitizers that remove invalid characters, always introduce a lightweight `.bytes().any(...)` check as a fast path to return a borrowed string or perform logic immediately, reserving allocations and filtering for the slow path.

## 2025-05-31 - Fast-Path Empty HTML Node Checking
**Learning:** Checking for empty containers using `.chars().next().expect(...).is_whitespace()` in a tight loop is exceptionally slow (~640ms/100k chars) because it requires UTF-8 character decoding on every step, even when parsing contiguous ASCII spaces (like those found in deeply nested, pretty-printed HTML).
**Action:** When searching for the end of whitespace sequences, prefer byte-slice operations like `.as_bytes().iter().position(...)` with a fallback for multi-byte Unicode. `iter().position` utilizes highly optimized internal implementations (often SIMD vectorization via `memchr`), providing a 10x+ speedup on large whitespace sequences.

## 2024-06-05 - Avoid `.bytes().filter().collect()` for string filtering
**Learning:** While iterating over `.bytes()` rather than `.chars()` avoids UTF-8 decoding overhead, using `.bytes().filter(...).collect()` to build a new `Vec<u8>` is actually *slower* than using `.chars()` in Rust due to iterator and allocation overhead. The fastest way to filter a string byte-by-byte is to pre-allocate a vector with the maximum capacity (`Vec::with_capacity(url.len())`) and then populate it using `.extend()` with a filtered iterator. This avoids both the initial `memcpy` of the entire string and the shifting overhead of `.retain()`.
**Action:** When optimizing string filtering for ASCII-only characters, always use `Vec::with_capacity(len)` combined with `.extend()` rather than chaining iterators with `.collect()` or using `.to_vec()` with `.retain()`.

## 2024-06-07 - Defer into_owned() to Avoid Unnecessary String Allocations
**Learning:** In Rust, chained string replacements after `regex::replace_all` should not eagerly call `.into_owned()`. `str::replace` always allocates a new `String` even if the pattern isn't found.
**Action:** Keep strings as `Cow<str>` and use `.contains()` before calling `.replace()` for boundary characters or rare patterns to avoid costly, unnecessary allocations in hot paths. Delay `.into_owned()` until all replacements are final.

## 2024-05-18 - Early Exit Matching for Dangerous URL Schemes
**Learning:** Checking URL schemes iteratively using an array of strings creates excessive looping overhead for the "happy path" (safe URLs), especially when iterators need to be created and advanced multiple times.
**Action:** When filtering or matching strings against a small, known set of prefixes, match on the first byte immediately to provide an `O(1)` fast-path reject for the vast majority of non-matching strings.
## $(date +%Y-%m-%d) - Safe Zero-Allocation String Prefix Matching
**Learning:** In Rust, replacing heap-allocating `.to_ascii_lowercase()` with string slicing (`url[..7].eq_ignore_ascii_case("http://")`) introduces a critical DoS vulnerability because slicing panics on invalid UTF-8 multi-byte character boundaries.
**Action:** When performing zero-allocation string checks on arbitrary user input against ASCII constants, always use byte slices (`url.as_bytes()[..7].eq_ignore_ascii_case(b"http://")`) to safely avoid panics.
