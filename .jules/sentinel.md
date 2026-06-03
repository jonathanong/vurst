## 2024-05-25 - Prevent SSRF/Open Redirects via Protocol-Relative Backslash Bypasses
**Vulnerability:** A URL filter validating schemes and rejecting protocol-relative URLs missed variations using backslashes (`\\`, `/\`, `\/`).
**Learning:** Browsers and URL parsers often normalize backslashes to forward slashes. Filtering out `//` is insufficient as an attacker can provide `\\attacker.com` to achieve an open redirect or SSRF, which the browser executes as `//attacker.com`.
**Prevention:** Always check for `\`, `/`, `/\`, `\/`, `\\` variations when attempting to block or identify protocol-relative URLs.

## 2025-02-20 - Protocol-relative URL bypasses using backslashes
**Vulnerability:** The `is_relative_url` check in `image_proxy` only validated `url.starts_with("//")`. An attacker could bypass this check and trick the system into classifying a protocol-relative URL (like `\\attacker.com` or `/\attacker.com`) as a normal relative URL, leading to potential SSRF or open redirect vulnerabilities.
**Learning:** Browsers and URL parsers typically normalize `\\`, `\/`, and `/\` to `//`. An exact string match for `//` is insufficient.
**Prevention:** Utilize byte slice pattern matching on the first two bytes to catch all slash/backslash combinations (`matches!(url.as_bytes()[..2], [b'/' | b'\\', b'/' | b'\\'])`), and explicitly handle single backslashes `starts_with('\\')`.
