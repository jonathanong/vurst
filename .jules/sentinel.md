## 2024-05-25 - Prevent SSRF/Open Redirects via Protocol-Relative Backslash Bypasses
**Vulnerability:** A URL filter validating schemes and rejecting protocol-relative URLs missed variations using backslashes (`\\`, `/\`, `\/`).
**Learning:** Browsers and URL parsers often normalize backslashes to forward slashes. Filtering out `//` is insufficient as an attacker can provide `\\attacker.com` to achieve an open redirect or SSRF, which the browser executes as `//attacker.com`.
**Prevention:** Always check for `\`, `/`, `/\`, `\/`, `\\` variations when attempting to block or identify protocol-relative URLs.

## 2024-05-24 - Protocol-Relative URL Bypass via Backslashes
**Vulnerability:** URL safety checks (like `is_relative_url`) intended to reject protocol-relative URLs (`//example.com`) only checked for the standard forward slash pattern (`url.starts_with("//")`). This allowed an attacker to bypass the check by using backslash variations (`\\`, `/\`, or `\/`), which parsers or browsers normalize to `//` and treat as protocol-relative URLs.
**Learning:** Checking for protocol-relative URLs by only looking for `//` is insufficient because browsers and URL parsers normalize backslashes (`\`) to forward slashes (`/`). A comprehensive check must account for all combinations of forward and backward slashes at the start of a URL.
**Prevention:** When checking for protocol-relative URLs without incurring full string decoding overhead, use a byte slice pattern match on the first two bytes to check for all slash variations: `url.len() >= 2 && matches!(url.as_bytes()[..2], [b'/' | b'\\', b'/' | b'\\'])`.
