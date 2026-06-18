use super::entities::decode_html_entities;
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

const HTML_BOUNDARY: char = '\u{E000}';
const HTML_ROLE_BOUNDARY: char = '\u{E001}';
const HTML_BOUNDARY_REPLACEMENT: &str = " \u{E000} ";
const HTML_ROLE_BOUNDARY_REPLACEMENT: &str = " \u{E001} ";
const MAX_ROLE_TAG_LEN: usize = 10;

fn is_role_boundary_tag(tag_name: &[u8]) -> bool {
    if tag_name.is_empty() || tag_name.len() > MAX_ROLE_TAG_LEN {
        return false;
    }

    let len = tag_name.len();
    let mut buf = [0u8; MAX_ROLE_TAG_LEN];
    buf[..len].copy_from_slice(tag_name);
    buf[..len].make_ascii_lowercase();

    matches!(
        &buf[..len],
        b"address"
            | b"article"
            | b"aside"
            | b"blockquote"
            | b"body"
            | b"br"
            | b"caption"
            | b"col"
            | b"colgroup"
            | b"dd"
            | b"details"
            | b"dialog"
            | b"div"
            | b"dl"
            | b"dt"
            | b"fieldset"
            | b"figcaption"
            | b"figure"
            | b"footer"
            | b"form"
            | b"h1"
            | b"h2"
            | b"h3"
            | b"h4"
            | b"h5"
            | b"h6"
            | b"header"
            | b"head"
            | b"hr"
            | b"html"
            | b"legend"
            | b"li"
            | b"main"
            | b"menu"
            | b"nav"
            | b"noscript"
            | b"ol"
            | b"p"
            | b"pre"
            | b"script"
            | b"section"
            | b"style"
            | b"summary"
            | b"table"
            | b"tbody"
            | b"template"
            | b"td"
            | b"tfoot"
            | b"th"
            | b"thead"
            | b"title"
            | b"tr"
            | b"ul"
    )
}

// Unicode format characters (Cf category): zero-width space (U+200B), zero-width
// non-joiner (U+200C), zero-width joiner (U+200D), soft hyphen (U+00AD), BOM (U+FEFF),
// and others. Attackers insert these between keywords to evade INJECTION_RE's \s+:
//   "ignore\u{200B}previous instructions" → \s+ does not match U+200B
// Replacing with a space (not empty string) preserves word boundaries so the phrase
// collapses to "ignore previous instructions" that INJECTION_RE then strips.
static ZERO_WIDTH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\p{Cf}").expect("BUG: invalid ZERO_WIDTH_RE"));

fn html_boundary_separator_pattern() -> String {
    format!(
        r"[\s\x{{{:X}}}\x{{{:X}}}]+",
        HTML_BOUNDARY as u32, HTML_ROLE_BOUNDARY as u32
    )
}

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
// Note: `<system>` tags are removed by the HTML tag stripping pass.
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
// Internal HTML boundary markers are accepted as whitespace so phrases split
// by stripped tags/comments still collapse for the second injection-pattern pass.
// Matches are replaced with a space (not empty string) so that adjacent text around a
// stripped phrase is not concatenated into a new word (e.g. "pretext ignore…suffix" →
// "pretext  suffix" rather than "pretextsuffix").
static INJECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    let sep = html_boundary_separator_pattern();
    let html_tag_remainder = r#"(?:(?:"[^"]*"|'[^']*'|[^>"'])*)"#;

    Regex::new(&format!(
        r"(?i)(?:\bignore(?:{sep}all)?{sep}(?:(?:the|your|my|our){sep})?previous{sep}(?:instructions?|prompts?)|\bignore{sep}all{sep}(?:(?:the|your|my|our){sep})?(?:instructions?|prompts?)|\bforget{sep}(?:(?:the|your|my|our){sep})*(?:(?:all|previous){sep})+(?:(?:the|your|my|our){sep})?(?:instructions?|prompts?|above)|\bforget{sep}everything{sep}above|\bdisregard{sep}(?:all{sep})?(?:(?:the|your|my|our){sep})?previous{sep}(?:instructions?|prompts?)|\bdisregard{sep}all{sep}(?:(?:the|your|my|our){sep})?(?:instructions?|prompts?)|<\|im_start\|>|<\|im_end\|>|<\|begin_of_text\|>|<\|start_header_id\|>|<\|end_header_id\|>|<\|eot_id\|>|\[INST\]|\[/INST\]|<system\b{html_tag_remainder}>|</system\b{html_tag_remainder}>)"
    ))
    .expect("BUG: invalid INJECTION_RE")
});

// Also strips CDATA sections (<![CDATA[...]]>) which RSS feeds use to embed HTML;
// comment/block-style markup is handled separately from tag stripping.
static HTML_COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)(?:<!--.*?-->|<!\[CDATA\[.*?\]\]>)").expect("BUG: invalid HTML_COMMENT_RE")
});

// Only system: and assistant: are removed — both are LLM-specific role labels with no
// common legitimate use at line starts or immediately after structural HTML boundaries.
// user: is intentionally excluded: it appears frequently in email headers ("User: Alice"),
// log entries, and form submissions.
static ROLE_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    let generic_boundary = format!(r"\x{{{:X}}}", HTML_BOUNDARY as u32);
    let role_boundary = format!(r"\x{{{:X}}}", HTML_ROLE_BOUNDARY as u32);

    Regex::new(&format!(
        r"(?im)(^[^\S\n]*(?:{generic_boundary}[^\S\n]*)*|[^\S\n]*{role_boundary}(?:[^\S\n]*{generic_boundary})*[^\S\n]*)(system|assistant):[^\S\n]*"
    ))
        .expect("BUG: invalid ROLE_PREFIX_RE")
});

static HORIZONTAL_WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\S\n]+").expect("BUG: invalid HORIZONTAL_WHITESPACE_RE"));

static EXCESSIVE_NEWLINES_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\n{3,}").expect("BUG: invalid EXCESSIVE_NEWLINES_RE"));

static ALL_WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{White_Space}]+").expect("BUG: invalid ALL_WHITESPACE_RE"));

fn html_tag_replacement(tag: &str) -> &'static str {
    let bytes = tag.as_bytes();

    // Fallback for unexpected empty tags
    if bytes.is_empty() {
        return HTML_BOUNDARY_REPLACEMENT;
    }

    let tag_name_start = if bytes.get(1) == Some(&b'/') { 2 } else { 1 };

    let mut tag_name_end = tag_name_start;
    while tag_name_end < bytes.len() {
        let b = bytes[tag_name_end];
        if b == b'>' || b == b'/' || b.is_ascii_whitespace() {
            break;
        }
        tag_name_end += 1;
    }

    let is_role_boundary = is_role_boundary_tag(&bytes[tag_name_start..tag_name_end]);

    if is_role_boundary {
        HTML_ROLE_BOUNDARY_REPLACEMENT
    } else {
        HTML_BOUNDARY_REPLACEMENT
    }
}

fn strip_zero_width_and_boundaries(content: &str) -> String {
    // Strip Unicode format/zero-width characters (Cf category) — replacing with
    // a space so "ignore\u{200B}previous" becomes "ignore previous" rather than
    // "ignoreprevious", allowing INJECTION_RE's whitespace separator to match.
    let mut sanitized = ZERO_WIDTH_RE.replace_all(content, " ");
    // ⚡ Bolt: Boundary sentinels both start with 0xEE in UTF-8; skip char scans
    // for the common case where no private-use sentinel bytes are present.
    if sanitized.as_bytes().contains(&0xEE) {
        let contains_boundary = sanitized.contains(HTML_BOUNDARY);
        let contains_role_boundary = sanitized.contains(HTML_ROLE_BOUNDARY);

        if contains_boundary && contains_role_boundary {
            sanitized =
                Cow::Owned(sanitized.replace(&[HTML_BOUNDARY, HTML_ROLE_BOUNDARY][..], " "));
        } else if contains_boundary {
            sanitized = Cow::Owned(sanitized.replace(HTML_BOUNDARY, " "));
        } else if contains_role_boundary {
            sanitized = Cow::Owned(sanitized.replace(HTML_ROLE_BOUNDARY, " "));
        }
    }
    sanitized.into_owned()
}

fn remove_injection_patterns(content: &str) -> String {
    INJECTION_RE.replace_all(content, " ").into_owned()
}

fn strip_html_tag(content: &str, tag_start: usize) -> Option<(usize, &'static str)> {
    let bytes = content.as_bytes();
    let len = bytes.len();

    let mut cursor = tag_start + 1;
    if cursor >= len {
        return None;
    }

    if bytes[cursor] == b'/' {
        cursor += 1;
        if cursor >= len {
            return None;
        }
    }

    if !bytes[cursor].is_ascii_alphabetic() {
        return None;
    }

    let mut quote: Option<u8> = None;
    while cursor < len {
        let b = bytes[cursor];

        if let Some(quote_char) = quote {
            if b == quote_char {
                quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            quote = Some(b);
        } else if b == b'>' {
            return Some((
                cursor + 1,
                html_tag_replacement(&content[tag_start..=cursor]),
            ));
        }

        cursor += 1;
    }

    None
}

fn strip_html_markup(content: &str) -> String {
    // Remove HTML comments
    let sanitized = HTML_COMMENT_RE
        .replace_all(content, HTML_BOUNDARY_REPLACEMENT)
        .into_owned();

    let bytes = sanitized.as_bytes();
    let mut cursor = 0;
    let mut stripped = String::with_capacity(sanitized.len());

    // ⚡ Bolt: Use memchr (via `position`) to fast-forward to the next '<' character
    // instead of scanning byte-by-byte and character-by-character.
    while let Some(relative_pos) = bytes[cursor..].iter().position(|&b| b == b'<') {
        let tag_start = cursor + relative_pos;
        if let Some((next_cursor, replacement)) = strip_html_tag(&sanitized, tag_start) {
            stripped.push_str(&sanitized[cursor..tag_start]);
            stripped.push_str(replacement);
            cursor = next_cursor;
        } else {
            // The '<' wasn't a valid tag we strip, so just copy it and move past it
            let next = tag_start + 1;
            stripped.push_str(&sanitized[cursor..next]);
            cursor = next;
        }
    }

    if cursor < sanitized.len() {
        stripped.push_str(&sanitized[cursor..]);
    }

    stripped
}

fn remove_role_prefixes(content: &str) -> String {
    // Remove role prefixes at line starts or after structural HTML boundaries
    let mut sanitized = ROLE_PREFIX_RE.replace_all(content, "$1");
    // ⚡ Bolt: Boundary sentinels both start with 0xEE in UTF-8; skip char scans
    // for the common case where no private-use sentinel bytes are present.
    if sanitized.as_bytes().contains(&0xEE) {
        let contains_boundary = sanitized.contains(HTML_BOUNDARY);
        let contains_role_boundary = sanitized.contains(HTML_ROLE_BOUNDARY);

        if contains_boundary && contains_role_boundary {
            sanitized =
                Cow::Owned(sanitized.replace(&[HTML_BOUNDARY, HTML_ROLE_BOUNDARY][..], " "));
        } else if contains_boundary {
            sanitized = Cow::Owned(sanitized.replace(HTML_BOUNDARY, " "));
        } else if contains_role_boundary {
            sanitized = Cow::Owned(sanitized.replace(HTML_ROLE_BOUNDARY, " "));
        }
    }
    sanitized.into_owned()
}

fn normalize_whitespace(content: &str, is_title: bool) -> String {
    let mut sanitized;
    if is_title {
        sanitized = ALL_WHITESPACE_RE.replace_all(content, " ").into_owned();
    } else {
        sanitized = HORIZONTAL_WHITESPACE_RE
            .replace_all(content, " ")
            .into_owned();
        sanitized = EXCESSIVE_NEWLINES_RE
            .replace_all(&sanitized, "\n\n")
            .into_owned();
    }
    sanitized
}

/// Sanitize content to prevent prompt injection attacks.
///
/// Applies a 9-step pipeline:
/// 1. Decode HTML entities
/// 2. Strip Unicode format/zero-width characters (Cf category) and internal HTML boundary markers
/// 3. Remove injection patterns (first pass — catches plain-text and entity-encoded)
/// 4. Remove HTML comments
/// 5. Strip HTML/XML tags while preserving internal markup boundaries
/// 6. Remove injection patterns (second pass — catches phrases split across comments/tags)
/// 7. Remove role prefixes at line starts or after stripped HTML boundaries
/// 8. Normalize whitespace
/// 9. Trim
///
/// Two injection passes are needed because comments and tags act as whitespace-free
/// splitters: `ignore<!-- -->previous` or `ignore<span></span>previous` would bypass a
/// single pre-stripping pass, but after step 4/5 the phrase resolves to
/// `ignore  previous` which `INJECTION_RE` matches in step 6.
///
/// Role-prefix removal uses a structural HTML boundary marker so block tags can expose
/// `system:` or `assistant:` without broadening matches to normal prose such as
/// `the system: design notes`.
pub fn sanitize_prompt_injection_sync(content: &str, is_title: bool) -> String {
    // Step 1: Decode HTML entities
    let mut sanitized = decode_html_entities(content);

    // Step 2: Strip Unicode format/zero-width characters and boundary markers
    sanitized = strip_zero_width_and_boundaries(&sanitized);

    // Step 3: Remove injection patterns (first pass)
    sanitized = remove_injection_patterns(&sanitized);

    // Step 4 & 5: Remove HTML comments and tags
    sanitized = strip_html_markup(&sanitized);

    // Step 6: Remove injection patterns (second pass)
    sanitized = remove_injection_patterns(&sanitized);

    // Step 7: Remove role prefixes
    sanitized = remove_role_prefixes(&sanitized);

    // Step 8: Normalize whitespace
    sanitized = normalize_whitespace(&sanitized, is_title);

    // Step 9: Trim
    sanitized.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_tag_replacement_for_empty_tag_is_boundary_replacement() {
        assert_eq!(
            html_tag_replacement(""),
            HTML_BOUNDARY_REPLACEMENT,
            "empty HTML tag should map to boundary replacement"
        );
    }

    #[test]
    fn html_tag_replacement_matches_exact_role_boundaries() {
        assert_eq!(
            html_tag_replacement("</SECTION>"),
            HTML_ROLE_BOUNDARY_REPLACEMENT
        );
        assert_eq!(
            html_tag_replacement("<section/>"),
            HTML_ROLE_BOUNDARY_REPLACEMENT
        );
        assert_eq!(
            html_tag_replacement("<section class=\"x\">"),
            HTML_ROLE_BOUNDARY_REPLACEMENT
        );
        assert_eq!(html_tag_replacement("<custom>"), HTML_BOUNDARY_REPLACEMENT);
        assert_eq!(
            html_tag_replacement("<averylongtagname>"),
            HTML_BOUNDARY_REPLACEMENT
        );
        assert_eq!(
            html_tag_replacement("<section-custom>"),
            HTML_BOUNDARY_REPLACEMENT
        );
    }
}
