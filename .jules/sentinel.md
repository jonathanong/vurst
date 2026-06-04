## 2024-05-25 - Prevent SSRF/Open Redirects via Protocol-Relative Backslash Bypasses
**Vulnerability:** URL safety checks (like `is_relative_url`) intended to reject protocol-relative URLs (`//example.com`) only checked for the standard forward slash pattern. This allowed an attacker to bypass the check by using backslash variations (`\\`, `/\`, or `\/`), which parsers or browsers normalize to `//` and treat as protocol-relative URLs.
**Learning:** Browsers and URL parsers often normalize backslashes to forward slashes. Filtering out `//` is insufficient as an attacker can provide `\\attacker.com` to achieve an open redirect or SSRF, which the browser executes as `//attacker.com`. A comprehensive check must account for all combinations of forward and backward slashes at the start of a URL.
**Prevention:** When checking for protocol-relative URLs without incurring full string decoding overhead, use a byte slice pattern match on the first two bytes to check for all slash variations: `matches!(url.as_bytes(), [b'/' | b'\\', b'/' | b'\\', ..])`.
## 2025-02-28 - [Protocol-relative SSRF Bypass via Single Backslash]
**Vulnerability:** [is_relative_url missed single backslash urls like \attacker.com]
**Learning:** [Browsers and parsers can normalize single backslash to //, leading to SSRF bypasses if only double backslashes are checked]
**Prevention:** [Always check for single backslash prefixes in addition to double backslashes when validating protocol-relative URLs]
