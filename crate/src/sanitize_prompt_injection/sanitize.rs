use regex::Regex;
use std::sync::LazyLock;

// Named HTML entity map — includes bracket/pipe chars used in control tokens
// ([INST], [/INST], <|im_start|>) so that named-entity encoded forms are decoded
// before the injection-pattern check runs.
static HTML_ENTITIES: &[(&str, &str)] = &[
    ("&amp;", "&"),
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&quot;", "\""),
    ("&#39;", "'"),
    ("&apos;", "'"),
    ("&nbsp;", " "),
    ("&lsqb;", "["),
    ("&rsqb;", "]"),
    ("&lbrack;", "["),
    ("&rbrack;", "]"),
    ("&vert;", "|"),
    ("&verbar;", "|"),
    ("&sol;", "/"),
];

static HTML_ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)&(?:[a-z]+|#\d+|#x[0-9a-f]+);").expect("BUG: invalid HTML_ENTITY_RE")
});

// Unicode format characters (Cf category): zero-width space (U+200B), zero-width
// non-joiner (U+200C), zero-width joiner (U+200D), soft hyphen (U+00AD), BOM (U+FEFF),
// and others. Attackers insert these between keywords to evade INJECTION_RE's \s+:
//   "ignore\u{200B}previous instructions" → \s+ does not match U+200B
// Replacing with a space (not empty string) preserves word boundaries so the phrase
// collapses to "ignore previous instructions" that INJECTION_RE then strips.
static ZERO_WIDTH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{Cf}").expect("BUG: invalid ZERO_WIDTH_RE"));

// All injection patterns combined into a single alternation for one-pass replacement.
// Each branch targets a specific adversarial vector:
//   - ignore [all] [the|your|my|our] previous instructions/prompts
//   - ignore all [the|your|my|our] instructions/prompts
//   - forget [the|your|my|our] (all|previous)+ [the|your|my|our] instructions/prompts/above
//   - forget everything above (specific unambiguous phrase)
//   - disregard [all] [the|your|my|our] previous instructions/prompts
//   - disregard all [the|your|my|our] instructions/prompts
//   - LLM control tokens: ChatML (<|im_start|>, <|im_end|>), Llama 2 ([INST], [/INST]),
//     Llama 3 (<|begin_of_text|>, <|start_header_id|>, <|end_header_id|>, <|eot_id|>)
//   - <system> tags (bare or with attributes)
// Optional article/pronoun (the|your|my|our) between the verb and qualifier prevents
// bypasses like "forget the previous instructions" or "ignore your previous prompts".
// Intentionally excluded:
//   - "new instructions" — too common in legitimate content
//     (e.g. "Follow these new instructions to configure your environment.")
//   - "ignore/forget/disregard/reset instructions" without any qualifier — too broad;
//     e.g. "disregard instructions you cannot follow", "factory reset instructions for your router",
//     or "reset prompts to default" would be incorrectly sanitized.
//     The qualifier (all/previous) disambiguates adversarial intent.
// Word-boundary anchors (\b) prevent partial-word matches such as "helperignore previous
// instructions" from being stripped, and avoid confusion with zero-width Unicode characters
// adjacent to trigger words.
// Matches are replaced with a space (not empty string) so that adjacent text around a
// stripped phrase is not concatenated into a new word (e.g. "pretext ignore…suffix" →
// "pretext  suffix" rather than "pretextsuffix").
static INJECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:\bignore(?:\s+all)?\s+(?:(?:the|your|my|our)\s+)?previous\s+(?:instructions?|prompts?)|\bignore\s+all\s+(?:(?:the|your|my|our)\s+)?(?:instructions?|prompts?)|\bforget\s+(?:(?:the|your|my|our)\s+)*(?:(?:all|previous)\s+)+(?:(?:the|your|my|our)\s+)?(?:instructions?|prompts?|above)|\bforget\s+everything\s+above|\bdisregard\s+(?:all\s+)?(?:(?:the|your|my|our)\s+)?previous\s+(?:instructions?|prompts?)|\bdisregard\s+all\s+(?:(?:the|your|my|our)\s+)?(?:instructions?|prompts?)|<\|im_start\|>|<\|im_end\|>|<\|begin_of_text\|>|<\|start_header_id\|>|<\|end_header_id\|>|<\|eot_id\|>|\[INST\]|\[/INST\]|<system\b[^>]*>|</system\b[^>]*>)",
    )
    .expect("BUG: invalid INJECTION_RE")
});

// Also strips CDATA sections (<![CDATA[...]]>) which RSS feeds use to embed HTML;
// HTML_TAG_RE does not match <! prefixes so CDATA must be removed here.
static HTML_COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)(?:<!--.*?-->|<!\[CDATA\[.*?\]\]>)").expect("BUG: invalid HTML_COMMENT_RE")
});

// Limitation: [^>]* stops at the first '>' so a crafted attribute containing an
// unquoted '>' (e.g. onclick="x>y") would leave a dangling fragment; the fragment
// could produce a role prefix or injection phrase after tag stripping.
// For inputs containing arbitrary HTML attributes, always preprocess with
// sanitize_rss_html (DOM-parser path) before calling sanitize_prompt_injection_sync.
static HTML_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</?[a-z][^>]*>").expect("BUG: invalid HTML_TAG_RE"));

// Only system: and assistant: are removed — both are LLM-specific role labels with no
// common legitimate use at line starts. user: is intentionally excluded: it appears
// frequently in email headers ("User: Alice"), log entries, and form submissions.
static ROLE_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^[^\S\n]*(system|assistant):[^\S\n]*").expect("BUG: invalid ROLE_PREFIX_RE")
});

static HORIZONTAL_WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\S\n]+").expect("BUG: invalid HORIZONTAL_WHITESPACE_RE"));

static EXCESSIVE_NEWLINES_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n{3,}").expect("BUG: invalid EXCESSIVE_NEWLINES_RE"));

static ALL_WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{White_Space}]+").expect("BUG: invalid ALL_WHITESPACE_RE"));

fn decode_html_entities(text: &str) -> String {
    HTML_ENTITY_RE
        .replace_all(text, |caps: &regex::Captures| {
            let m = &caps[0];

            // Check named entities first
            for (entity, replacement) in HTML_ENTITIES {
                if m.eq_ignore_ascii_case(entity) {
                    return (*replacement).to_string();
                }
            }

            // Handle numeric entities
            let inner = &m[1..m.len() - 1]; // strip & and ;
            if let Some(digits) = inner
                .strip_prefix("#x")
                .or_else(|| inner.strip_prefix("#X"))
            {
                return u32::from_str_radix(digits, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map_or_else(|| m.to_string(), |c| c.to_string());
            }

            if let Some(digits) = inner.strip_prefix('#') {
                return digits
                    .parse::<u32>()
                    .ok()
                    .and_then(char::from_u32)
                    .map_or_else(|| m.to_string(), |c| c.to_string());
            }

            m.to_string()
        })
        .into_owned()
}

/// Sanitize content to prevent prompt injection attacks.
///
/// Applies a 9-step pipeline:
/// 1. Decode HTML entities
/// 2. Strip Unicode format/zero-width characters (Cf category)
/// 3. Remove injection patterns (first pass — catches plain-text and entity-encoded)
/// 4. Remove HTML comments
/// 5. Strip HTML/XML tags
/// 6. Remove injection patterns (second pass — catches phrases split across comments/tags)
/// 7. Remove role prefixes at line starts
/// 8. Normalize whitespace
/// 9. Trim
///
/// Two injection passes are needed because comments and tags act as whitespace-free
/// splitters: `ignore<!-- -->previous` or `ignore<span></span>previous` would bypass a
/// single pre-stripping pass, but after step 4/5 the phrase resolves to
/// `ignore  previous` which `\s+` in `INJECTION_RE` matches in step 6.
///
/// # Limitations
/// - Role-prefix removal (step 6) only matches at line starts. HTML block-tag merging
///   in step 4 can place `system:` mid-line (e.g. `<p>text</p><p>system: bad</p>`
///   → `text  system: bad`), which step 6 will not catch. For rich HTML input prefer
///   the DOM-parser path (`sanitize_rss_html`) which preserves newline boundaries.
pub fn sanitize_prompt_injection_sync(content: &str, is_title: bool) -> String {
    // Step 1: Decode HTML entities
    let mut sanitized = decode_html_entities(content);

    // Step 2: Strip Unicode format/zero-width characters (Cf category) — replacing with
    // a space so "ignore\u{200B}previous" becomes "ignore previous" rather than
    // "ignoreprevious", allowing INJECTION_RE's \s+ to match in step 3.
    sanitized = ZERO_WIDTH_RE.replace_all(&sanitized, " ").into_owned();

    // Step 3: Remove injection patterns — first pass catches plain-text and entity-encoded
    sanitized = INJECTION_RE.replace_all(&sanitized, " ").into_owned();

    // Step 4: Remove HTML comments
    sanitized = HTML_COMMENT_RE.replace_all(&sanitized, " ").into_owned();

    // Step 5: Strip HTML/XML tags
    sanitized = HTML_TAG_RE.replace_all(&sanitized, " ").into_owned();

    // Step 6: Remove injection patterns — second pass catches phrases that were split
    // across HTML comments or inline tags (e.g. ignore<!-- -->previous instructions)
    sanitized = INJECTION_RE.replace_all(&sanitized, " ").into_owned();

    // Step 7: Remove role prefixes at line starts
    sanitized = ROLE_PREFIX_RE.replace_all(&sanitized, "").into_owned();

    // Step 8: Normalize whitespace
    if is_title {
        sanitized = ALL_WHITESPACE_RE.replace_all(&sanitized, " ").into_owned();
    } else {
        sanitized = HORIZONTAL_WHITESPACE_RE
            .replace_all(&sanitized, " ")
            .into_owned();
        sanitized = EXCESSIVE_NEWLINES_RE
            .replace_all(&sanitized, "\n\n")
            .into_owned();
    }

    // Step 9: Trim
    sanitized.trim().to_string()
}
