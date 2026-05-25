## 2024-05-25 - Prevent SSRF/Open Redirects via Protocol-Relative Backslash Bypasses
**Vulnerability:** A URL filter validating schemes and rejecting protocol-relative URLs missed variations using backslashes (`\\`, `/\`, `\/`).
**Learning:** Browsers and URL parsers often normalize backslashes to forward slashes. Filtering out `//` is insufficient as an attacker can provide `\\attacker.com` to achieve an open redirect or SSRF, which the browser executes as `//attacker.com`.
**Prevention:** Always check for `\`, `/`, `/\`, `\/`, `\\` variations when attempting to block or identify protocol-relative URLs.
