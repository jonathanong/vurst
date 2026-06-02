## 2024-05-25 - Prevent SSRF/Open Redirects via Protocol-Relative Backslash Bypasses
**Vulnerability:** A URL filter validating schemes and rejecting protocol-relative URLs missed variations using backslashes (`\\`, `/\`, `\/`).
**Learning:** Browsers and URL parsers often normalize backslashes to forward slashes. Filtering out `//` is insufficient as an attacker can provide `\\attacker.com` to achieve an open redirect or SSRF, which the browser executes as `//attacker.com`.
**Prevention:** Always check for `\`, `/`, `/\`, `\/`, `\\` variations when attempting to block or identify protocol-relative URLs.

## 2024-06-02 - SSRF Bypass via Normalization of Backslashes in URLs
**Vulnerability:** The HTML/Markdown sanitization and image proxy validation logic failed to account for browser normalization of backslashes. Inputs like `\/attacker.com`, `/\attacker.com`, and `\\attacker.com` bypassed the `starts_with("//")` protocol-relative checks but were still treated as external hosts by browsers, leading to potential Server-Side Request Forgery (SSRF) via the image proxy and open redirects.
**Learning:** Checking for `//` is insufficient when validating URLs that will be rendered in a browser. Browsers normalize both forward and backward slashes interchangeably when parsing URLs, meaning that any combination of two slashes/backslashes creates a protocol-relative link.
**Prevention:** Always check the first two characters for any combination of `\` and `/`. In Rust, a highly efficient pattern match `matches!(url.as_bytes()[..2], [b'/' | b'\\', b'/' | b'\\'])` should be used instead of sequential string decoding checks. Ensure that `starts_with('\\')` checks are also preserved if necessary to handle single-backslash bypasses.
