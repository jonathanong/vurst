## 2024-12-07 - Fix Stored XSS via HTML Entities in URL Validation

**Vulnerability:** The `is_safe_url` function was bypassing malicious schemes like `javascript:` if they were obfuscated with HTML entities (e.g., `javascript&#58;alert(1)`).
**Learning:** Browsers decode HTML entities when parsing `href` or `src` attributes. Simple string match checks on the raw markdown URL can easily be bypassed if the URL is not decoded first.
**Prevention:** Always decode HTML entities in URLs before validating their schemes to ensure the security check analyzes the exact same string the browser will eventually execute.
