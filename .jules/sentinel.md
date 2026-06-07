## 2024-05-24 - HTML Entity Decode Bypass in Sanitizer
**Vulnerability:** The HTML sanitizer's custom link scheme checker (`has_dangerous_url_scheme`) evaluated URL strings directly without decoding HTML entities first. This allowed attackers to use entities like `&#58;` for a colon (e.g., `javascript&#58;alert(1)`), evading detection while still being executed as XSS by the browser.
**Learning:** Browsers process URL attributes by first unescaping HTML entities before parsing the scheme. Custom sanitization filters that inspect raw attributes must emulate this decoding phase, otherwise they suffer from defense-in-depth bypasses.
**Prevention:** Always parse and decode HTML entities on attribute values *before* evaluating their contents for safety rules.
