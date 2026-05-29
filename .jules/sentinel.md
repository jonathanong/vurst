## 2024-05-25 - Prevent SSRF/Open Redirects via Protocol-Relative Backslash Bypasses
**Vulnerability:** A URL filter validating schemes and rejecting protocol-relative URLs missed variations using backslashes (`\\`, `/\`, `\/`).
**Learning:** Browsers and URL parsers often normalize backslashes to forward slashes. Filtering out `//` is insufficient as an attacker can provide `\\attacker.com` to achieve an open redirect or SSRF, which the browser executes as `//attacker.com`.
**Prevention:** Always check for `\`, `/`, `/\`, `\/`, `\\` variations when attempting to block or identify protocol-relative URLs.

## 2024-05-27 - Protocol-relative URL check bypass via backslashes
**Vulnerability:** The `is_relative_url` function checked for protocol-relative URLs using only `url.starts_with("//")`. This allowed an attacker to bypass SSRF protections by passing URLs that start with mixed backslashes/forward slashes like `\\`, `/\`, or `\/`, which browsers and standard parsers normalize to `//` leading to external requests.
**Learning:** URL prefix checks based only on strings are fragile against URL normalization edge cases like backslashes in protocol-relative boundaries.
**Prevention:** Always validate URL schemes and boundaries using proper parsers, or safely check byte-slices against all normalized boundary permutations `matches!(url.as_bytes()[..2], [b'/' | b'\\', b'/' | b'\\'])`.
