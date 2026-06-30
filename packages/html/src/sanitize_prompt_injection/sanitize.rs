use super::entities::decode_html_entities;
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

const HTML_BOUNDARY: char = '\u{E000}';
const HTML_ROLE_BOUNDARY: char = '\u{E001}';
const HTML_BOUNDARY_REPLACEMENT: &str = " \u{E000} ";
const HTML_ROLE_BOUNDARY_REPLACEMENT: &str = " \u{E001} ";
const MAX_ROLE_TAG_LEN: usize = 10;

fn is_heading_tag(tag_name: &[u8]) -> bool {
    matches!(
        tag_name,
        b"h1" | b"h2" | b"h3" | b"h4" | b"h5" | b"h6" | b"header" | b"head"
    )
}

fn is_table_tag(tag_name: &[u8]) -> bool {
    matches!(
        tag_name,
        b"caption"
            | b"col"
            | b"colgroup"
            | b"table"
            | b"tbody"
            | b"td"
            | b"tfoot"
            | b"th"
            | b"thead"
            | b"tr"
    )
}

fn is_list_tag(tag_name: &[u8]) -> bool {
    matches!(
        tag_name,
        b"dd" | b"dl" | b"dt" | b"li" | b"ol" | b"ul" | b"menu"
    )
}

fn is_document_tag(tag_name: &[u8]) -> bool {
    matches!(
        tag_name,
        b"body" | b"html" | b"main" | b"script" | b"style" | b"title" | b"template" | b"noscript"
    )
}

fn is_block_tag(tag_name: &[u8]) -> bool {
    matches!(
        tag_name,
        b"address"
            | b"article"
            | b"aside"
            | b"blockquote"
            | b"br"
            | b"details"
            | b"dialog"
            | b"div"
            | b"fieldset"
            | b"figcaption"
            | b"figure"
            | b"footer"
            | b"form"
            | b"hr"
            | b"legend"
            | b"nav"
            | b"p"
            | b"pre"
            | b"section"
            | b"summary"
    )
}

fn is_role_boundary_tag(tag_name: &[u8]) -> bool {
    if tag_name.is_empty() || tag_name.len() > MAX_ROLE_TAG_LEN {
        return false;
    }

    let len = tag_name.len();
    let mut buf = [0u8; MAX_ROLE_TAG_LEN];
    buf[..len].copy_from_slice(tag_name);
    buf[..len].make_ascii_lowercase();

    let tag = &buf[..len];
    is_heading_tag(tag)
        || is_table_tag(tag)
        || is_list_tag(tag)
        || is_document_tag(tag)
        || is_block_tag(tag)
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

static ALL_WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{White_Space}]+").expect("BUG: invalid ALL_WHITESPACE_RE"));

fn borrowed_if_unchanged<'a>(content: &'a str, replaced: Cow<'a, str>) -> Cow<'a, str> {
    match replaced {
        Cow::Borrowed(_) => Cow::Borrowed(content),
        Cow::Owned(s) => Cow::Owned(s),
    }
}

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

fn strip_zero_width_and_boundaries(content: &str) -> Cow<'_, str> {
    // Strip Unicode format/zero-width characters (Cf category) — replacing with
    // a space so "ignore\u{200B}previous" becomes "ignore previous" rather than
    // "ignoreprevious", allowing INJECTION_RE's whitespace separator to match.
    let mut sanitized = borrowed_if_unchanged(content, ZERO_WIDTH_RE.replace_all(content, " "));
    // ⚡ Bolt: Boundary sentinels both start with 0xEE in UTF-8; skip char scans
    // for the common case where no private-use sentinel bytes are present.
    if sanitized.as_bytes().contains(&0xEE) {
        for boundary in [HTML_BOUNDARY, HTML_ROLE_BOUNDARY] {
            if sanitized.contains(boundary) {
                let mut i = 0;
                let s = sanitized.to_mut();
                while let Some(pos) = s[i..].find(boundary) {
                    i += pos;
                    s.replace_range(i..i + boundary.len_utf8(), " ");
                    i += 1; // " " is 1 byte
                }
            }
        }
    }
    sanitized
}

fn remove_injection_patterns(content: &str) -> Cow<'_, str> {
    borrowed_if_unchanged(content, INJECTION_RE.replace_all(content, " "))
}

fn find_tag_name_start(bytes: &[u8], tag_start: usize) -> Option<usize> {
    let mut cursor = tag_start + 1;
    if cursor >= bytes.len() {
        return None;
    }

    if bytes[cursor] == b'/' {
        cursor += 1;
        if cursor >= bytes.len() {
            return None;
        }
    }

    if !bytes[cursor].is_ascii_alphabetic() {
        return None;
    }

    Some(cursor)
}

fn find_tag_end(bytes: &[u8], start_cursor: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;
    let mut cursor = start_cursor;

    while cursor < bytes.len() {
        let b = bytes[cursor];

        if let Some(quote_char) = quote {
            if b == quote_char {
                quote = None;
            }
        } else if b == b'"' || b == b'\'' {
            quote = Some(b);
        } else if b == b'>' {
            return Some(cursor);
        }

        cursor += 1;
    }

    None
}

fn strip_html_tag(content: &str, tag_start: usize) -> Option<(usize, &'static str)> {
    let bytes = content.as_bytes();

    let name_start = find_tag_name_start(bytes, tag_start)?;
    let tag_end = find_tag_end(bytes, name_start)?;

    Some((
        tag_end + 1,
        html_tag_replacement(&content[tag_start..=tag_end]),
    ))
}

fn strip_html_markup(content: &str) -> Cow<'_, str> {
    // Fast path: if there are no HTML comments or tags, return early
    if !content.contains('<') {
        return Cow::Borrowed(content);
    }
    // Remove HTML comments
    let sanitized = HTML_COMMENT_RE.replace_all(content, HTML_BOUNDARY_REPLACEMENT);

    let bytes = sanitized.as_bytes();
    let mut cursor = 0;

    // Check if there are any tags to strip
    if !bytes.contains(&b'<') {
        return Cow::Owned(sanitized.into_owned());
    }

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

    Cow::Owned(stripped)
}

fn remove_role_prefixes(content: &str) -> Cow<'_, str> {
    // Remove role prefixes at line starts or after structural HTML boundaries
    let mut sanitized = borrowed_if_unchanged(content, ROLE_PREFIX_RE.replace_all(content, "$1"));
    // ⚡ Bolt: Boundary sentinels both start with 0xEE in UTF-8; skip char scans
    // for the common case where no private-use sentinel bytes are present.
    if sanitized.as_bytes().contains(&0xEE) {
        for boundary in [HTML_BOUNDARY, HTML_ROLE_BOUNDARY] {
            if sanitized.contains(boundary) {
                let mut i = 0;
                let s = sanitized.to_mut();
                while let Some(pos) = s[i..].find(boundary) {
                    i += pos;
                    s.replace_range(i..i + boundary.len_utf8(), " ");
                    i += 1; // " " is 1 byte
                }
            }
        }
    }
    sanitized
}

fn normalize_whitespace(content: &str, is_title: bool) -> Cow<'_, str> {
    if is_title {
        return borrowed_if_unchanged(content, ALL_WHITESPACE_RE.replace_all(content, " "));
    }

    let mut needs_modification = false;
    let mut in_horizontal_ws = false;
    let mut consecutive_newlines = 0;

    for c in content.chars() {
        if c == '\n' {
            if in_horizontal_ws {
                needs_modification = true;
                break;
            }
            consecutive_newlines += 1;
            if consecutive_newlines >= 3 {
                needs_modification = true;
                break;
            }
        } else if c.is_whitespace() {
            if in_horizontal_ws {
                needs_modification = true;
                break;
            }
            if c != ' ' {
                needs_modification = true;
                break;
            }
            in_horizontal_ws = true;
            consecutive_newlines = 0;
        } else {
            in_horizontal_ws = false;
            consecutive_newlines = 0;
        }
    }

    if !needs_modification && !in_horizontal_ws {
        return Cow::Borrowed(content);
    }

    let mut out = String::with_capacity(content.len());
    in_horizontal_ws = false;
    consecutive_newlines = 0;

    for c in content.chars() {
        if c == '\n' {
            if in_horizontal_ws {
                out.push(' ');
                in_horizontal_ws = false;
            }
            consecutive_newlines += 1;
            if consecutive_newlines > 2 {
                continue;
            }
            out.push('\n');
        } else if c.is_whitespace() {
            in_horizontal_ws = true;
            consecutive_newlines = 0;
        } else {
            if in_horizontal_ws {
                out.push(' ');
                in_horizontal_ws = false;
            }
            out.push(c);
            consecutive_newlines = 0;
        }
    }

    if !in_horizontal_ws {
        return Cow::Owned(out);
    }
    out.push(' ');

    Cow::Owned(out)
}

fn apply_injection_passes(mut sanitized: String) -> String {
    // Step 3: Remove injection patterns (first pass)
    sanitized = remove_injection_patterns(&sanitized).into_owned();

    // Step 4 & 5: Remove HTML comments and tags
    sanitized = strip_html_markup(&sanitized).into_owned();

    // Step 6: Remove injection patterns (second pass)
    sanitized = remove_injection_patterns(&sanitized).into_owned();

    sanitized
}

fn apply_final_formatting(mut sanitized: String, is_title: bool) -> String {
    // Step 7: Remove role prefixes
    sanitized = remove_role_prefixes(&sanitized).into_owned();

    // Step 8: Normalize whitespace
    sanitized = normalize_whitespace(&sanitized, is_title).into_owned();

    // Step 9: Trim
    sanitized.trim().to_string()
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
    let decoded_entities = decode_html_entities(content);

    // Step 2: Strip Unicode format/zero-width characters and boundary markers
    let mut sanitized = strip_zero_width_and_boundaries(&decoded_entities).into_owned();

    // Steps 3-6: Handle injection patterns and HTML markup
    sanitized = apply_injection_passes(sanitized);

    // Steps 7-9: Final formatting and cleanup
    apply_final_formatting(sanitized, is_title)
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
    fn strip_html_markup_removes_comment_with_no_tags() {
        assert_eq!(
            strip_html_markup("  <!-- injected -->  ").as_ref(),
            format!("  {}  ", HTML_BOUNDARY_REPLACEMENT).as_str()
        );
    }

    #[test]
    fn strip_html_markup_strips_tags_for_borrowed_and_owned_inputs() {
        let borrowed = strip_html_markup("<span>Hello</span>");
        assert!(borrowed.contains(HTML_BOUNDARY));
        assert!(borrowed.contains("Hello"));

        let owned_input = String::from("<span>World</span>");
        let owned = strip_html_markup(&owned_input);
        assert!(owned.contains(HTML_BOUNDARY));
        assert!(owned.contains("World"));
    }

    #[test]
    fn strip_html_tag_returns_boundary_replacement_for_valid_tags() {
        assert_eq!(
            strip_html_tag("<span>Hello</span>", 0),
            Some((6, HTML_BOUNDARY_REPLACEMENT))
        );
    }

    #[test]
    fn strip_html_tag_ignores_angle_brackets_inside_quoted_attributes() {
        let tag = "<span data='>' class=\"x\">";
        assert_eq!(
            strip_html_tag(tag, 0),
            Some((tag.len(), HTML_BOUNDARY_REPLACEMENT))
        );
    }

    #[test]
    fn strip_zero_width_and_boundaries_replaces_both_internal_markers() {
        let input = format!("a{HTML_BOUNDARY}{HTML_ROLE_BOUNDARY}{HTML_BOUNDARY}b");
        assert_eq!(strip_zero_width_and_boundaries(&input), "a   b");
    }

    #[test]
    fn strip_zero_width_and_boundaries_replaces_each_single_boundary_kind() {
        let generic_only = format!("a{HTML_BOUNDARY}b");
        assert_eq!(strip_zero_width_and_boundaries(&generic_only), "a b");

        let role_only = format!("a{HTML_ROLE_BOUNDARY}b");
        assert_eq!(strip_zero_width_and_boundaries(&role_only), "a b");
    }

    #[test]
    fn remove_role_prefixes_cleans_role_and_generic_boundaries() {
        let input = format!("summary{HTML_ROLE_BOUNDARY}{HTML_BOUNDARY}system: override");
        assert_eq!(remove_role_prefixes(&input), "summary  override");
    }

    #[test]
    fn remove_role_prefixes_cleans_each_single_boundary_kind() {
        let generic_only = format!("{HTML_BOUNDARY}assistant: override");
        assert_eq!(remove_role_prefixes(&generic_only), " override");

        let role_only = format!("{HTML_ROLE_BOUNDARY}system: override");
        assert_eq!(remove_role_prefixes(&role_only), " override");
    }

    #[test]
    fn sanitize_prompt_injection_trims_surrounding_whitespace() {
        assert_eq!(
            sanitize_prompt_injection_sync("  safe input  ", false),
            "safe input"
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

    #[test]
    fn html_tag_replacement_covers_each_refactored_role_boundary_category() {
        for tag in ["<h1>", "<table>", "<ul>", "<html>", "<div>"] {
            assert_eq!(html_tag_replacement(tag), HTML_ROLE_BOUNDARY_REPLACEMENT);
        }
    }

    #[test]
    fn normalize_whitespace_returns_unchanged_content_when_already_normalized() {
        assert_eq!(
            normalize_whitespace("alpha beta\n\ngamma", false),
            "alpha beta\n\ngamma"
        );
    }

    #[test]
    fn normalize_whitespace_replaces_horizontal_whitespace_before_newlines() {
        assert_eq!(
            normalize_whitespace("alpha \n\tbeta", false),
            "alpha \n beta"
        );
    }

    #[test]
    fn normalize_whitespace_collapses_extra_blank_lines() {
        assert_eq!(
            normalize_whitespace("alpha\n\n\nbeta", false),
            "alpha\n\nbeta"
        );
    }

    #[test]
    fn normalize_whitespace_keeps_single_trailing_space_after_collapse() {
        assert_eq!(normalize_whitespace("alpha\t\t", false), "alpha ");
    }

    #[test]
    fn normalize_whitespace_collapses_all_title_whitespace() {
        assert_eq!(
            normalize_whitespace("alpha\tbeta\n\ngamma", true),
            "alpha beta gamma"
        );
    }

    #[test]
    fn normalize_whitespace_preserves_already_normalized_titles() {
        assert_eq!(normalize_whitespace("alpha beta", true), "alpha beta");
    }

    #[test]
    fn normalize_whitespace_collapses_repeated_spaces() {
        assert_eq!(normalize_whitespace("alpha  beta", false), "alpha beta");
    }

    #[test]
    fn normalize_whitespace_collapses_tabs_without_newlines() {
        assert_eq!(normalize_whitespace("alpha\tbeta", false), "alpha beta");
    }

    #[test]
    fn normalize_whitespace_preserves_single_trailing_space() {
        assert_eq!(normalize_whitespace("alpha ", false), "alpha ");
    }
}
