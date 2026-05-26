## 2024-05-25 - Prevent SSRF/Open Redirects via Protocol-Relative Backslash Bypasses
**Vulnerability:** A URL filter validating schemes and rejecting protocol-relative URLs missed variations using backslashes (`\\`, `/\`, `\/`).
**Learning:** Browsers and URL parsers often normalize backslashes to forward slashes. Filtering out `//` is insufficient as an attacker can provide `\\attacker.com` to achieve an open redirect or SSRF, which the browser executes as `//attacker.com`.
**Prevention:** Always check for `\`, `/`, `/\`, `\/`, `\\` variations when attempting to block or identify protocol-relative URLs.
## 2024-05-24 - SSRF / Open Redirect Bypass via Backslash Variations
**Vulnerability:** URL safety checks (`starts_with("//")`) used for rejecting protocol-relative URLs missed variations using backslashes (e.g., `\\`, `/\`, `\/`).
**Learning:** Browsers and URL parsers normalize backslashes to forward slashes. A simple `starts_with("//")` is insufficient to block all forms of protocol-relative URLs.
**Prevention:** Normalize input first, then check `clean_url.as_bytes().get(0..2).is_some_and(|bytes| matches!(bytes, [b'/' | b'\\', b'/' | b'\\'])`.
