## 2024-05-22 - Optimize Sequential String Replacements in Rust
**Learning:** In Rust, multiple sequential `String::replace` calls on single characters can be significantly slower than a single `String::replace` call using a char array slice (e.g., `s.replace(&['a', 'b'][..], " ")`) because the latter leverages Rust's optimized Pattern matching over multiple characters in a single allocation pass.
**Action:** When seeing successive `.replace(char)` calls on the same String, combine them into a single `.replace(&[char1, char2][..])` to cut allocations and iteration overhead by roughly half.
