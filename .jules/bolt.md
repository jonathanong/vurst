## 2023-10-27 - Optimize HTML string escaping with Cow<'a, str>
**Learning:** Chained `.replace()` calls for HTML escaping allocate and copy repeatedly on each replacement. A single-pass scan is faster and avoids unnecessary allocation for texts that require no escaping.
**Action:** Replace multiple `.replace` calls with a manual string scan using `char_indices` that builds an output buffer only when an escape character is encountered, returns `Cow<'_, str>`, and appends borrowed text directly when no replacements are needed.
