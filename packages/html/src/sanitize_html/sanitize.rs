use crate::serialize_fragment_body;
use ammonia::{Builder, UrlRelative};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use vurst_shared::html::char_refs::decode_numeric_char_ref;
use vurst_shared::image_proxy::{
    rewrite_image_to_proxy, should_proxy_image, DEFAULT_IMAGE_PROXY_URL_PREFIX,
};

use scraper::Html;

/// Attributes to strip from `<img>` elements specifically.
const IMG_ATTRS_TO_STRIP: &[&str] = &["srcset", "sizes", "width", "height"];

/// Tags whose content should be removed with the element.
///
/// Keep this list disjoint from Ammonia's allowed tag list. `script` and `style`
/// are already clean-content tags in Ammonia's default policy.
const CLEAN_CONTENT_TAGS: &[&str] = &[
    "iframe", "form", "input", "button", "select", "textarea", "object", "embed", "base", "svg",
    "math", "meta",
];

/// Fast-path matching for container tags eligible for empty-element cleanup.
///
/// Keep this list in sync with container-tag handling across HTML sanitization.
/// Any newly added container tag must be added here or it will be skipped.
fn is_container_tag(tag: &str) -> bool {
    let bytes = tag.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    match bytes[0].to_ascii_lowercase() {
        b'a' => match bytes.len() {
            7 => tag.eq_ignore_ascii_case("article"),
            5 => tag.eq_ignore_ascii_case("aside"),
            _ => false,
        },
        b'd' => match bytes.len() {
            3 => tag.eq_ignore_ascii_case("div"),
            7 => tag.eq_ignore_ascii_case("details"),
            _ => false,
        },
        b'f' => match bytes.len() {
            6 => tag.eq_ignore_ascii_case("footer") || tag.eq_ignore_ascii_case("figure"),
            10 => tag.eq_ignore_ascii_case("figcaption"),
            _ => false,
        },
        b'h' => bytes.len() == 6 && tag.eq_ignore_ascii_case("header"),
        b'm' => bytes.len() == 4 && tag.eq_ignore_ascii_case("main"),
        b'n' => bytes.len() == 3 && tag.eq_ignore_ascii_case("nav"),
        b'p' => match bytes.len() {
            1 => tag.eq_ignore_ascii_case("p"),
            _ => false,
        },
        b's' => match bytes.len() {
            4 => tag.eq_ignore_ascii_case("span"),
            7 => tag.eq_ignore_ascii_case("section") || tag.eq_ignore_ascii_case("summary"),
            _ => false,
        },
        _ => false,
    }
}

/// Options for [`sanitize_rss_html_sync`].
pub struct SanitizeRssHtmlOptions {
    /// When `true`, rewrite external `<img src>` URLs through the configured
    /// image-proxy prefix.
    pub proxy_images: bool,
    /// URL path prefix prepended to proxied image URLs (e.g. `/proxy/`).
    pub image_proxy_url_prefix: Arc<str>,
    /// Hex-encoded HMAC-SHA256 signing keys (newest first). Empty = dev mode (no sig).
    pub image_proxy_signing_keys: Arc<[String]>,
}

impl Default for SanitizeRssHtmlOptions {
    fn default() -> Self {
        Self {
            proxy_images: false,
            image_proxy_url_prefix: Arc::from(DEFAULT_IMAGE_PROXY_URL_PREFIX),
            image_proxy_signing_keys: Arc::default(),
        }
    }
}

/// Result returned by [`sanitize_rss_html_sync`].
pub struct SanitizeRssHtmlResult {
    /// Sanitized HTML safe for rendering.
    pub html: String,
    /// The raw `src` of the first external `<img>` found in the original HTML,
    /// before any proxying. `None` when no external image was present.
    pub first_image_src: Option<String>,
}

/// Sanitize raw RSS feed HTML into clean, safe content.
///
/// Applies fixed sanitization rules:
/// - Removes dangerous elements (script, style, iframe, form controls, etc.)
/// - Strips unsafe attributes (style, class, id, data-*, event handlers, etc.)
/// - Strips img-specific attrs (srcset, sizes, width, height)
/// - Adds safe defaults to links (rel="nofollow noopener", target="_blank")
/// - Adds performance attributes to images (loading="lazy", etc.)
/// - Optionally rewrites external `<img src>` URLs to `/proxy/` proxy paths
/// - Removes empty container elements left after sanitization
///
/// Returns [`SanitizeRssHtmlResult`] containing the sanitized HTML and the raw
/// `src` of the first external image found (for thumbnail extraction).
///
/// Returns a result with an empty string when `html` is empty; cannot fail on
/// malformed HTML because `scraper::Html::parse_fragment` is infallible.
pub fn sanitize_rss_html_sync(html: &str, opts: &SanitizeRssHtmlOptions) -> SanitizeRssHtmlResult {
    if html.is_empty() {
        return SanitizeRssHtmlResult {
            html: String::new(),
            first_image_src: None,
        };
    }

    // Pass 1: Ammonia owns the allowlist-based sanitization policy and fixed
    // link/image attributes. The closure also preserves our `/proxy/` image
    // proxy and captures the first original external image URL.
    let (sanitized, first_image_src) = sanitize_with_ammonia(html, opts);

    // Pass 2 only when needed: most RSS items do not contain containers that
    // became empty after sanitization, so avoid reparsing those fragments.
    let html = if may_have_empty_container(&sanitized) {
        remove_empty_containers_from_html(&sanitized)
    } else {
        sanitized
    };

    SanitizeRssHtmlResult {
        html,
        first_image_src,
    }
}

fn sanitize_with_ammonia(html: &str, opts: &SanitizeRssHtmlOptions) -> (String, Option<String>) {
    let proxy_images = opts.proxy_images;
    let signing_keys = Arc::clone(&opts.image_proxy_signing_keys);
    let url_prefix = Arc::clone(&opts.image_proxy_url_prefix);
    let first_image_src = Arc::new(Mutex::new(None::<String>));
    let first_image_src_filter = Arc::clone(&first_image_src);
    let mut builder = Builder::default();

    builder
        .link_rel(Some("nofollow noopener"))
        .url_relative(UrlRelative::PassThrough)
        .set_tag_attribute_value("a", "target", "_blank")
        .set_tag_attribute_value("img", "loading", "lazy")
        .set_tag_attribute_value("img", "fetchpriority", "low")
        .set_tag_attribute_value("img", "decoding", "async")
        .rm_tag_attributes("img", IMG_ATTRS_TO_STRIP)
        .add_clean_content_tags(CLEAN_CONTENT_TAGS)
        .attribute_filter(move |tag, attr, value| {
            if (attr == "href" || attr == "src") && has_dangerous_url_scheme(value) {
                return None;
            }
            if tag == "img" && attr == "src" && should_proxy_image(value, &url_prefix) {
                let mut captured = first_image_src_filter
                    .lock()
                    .expect("BUG: first image capture mutex poisoned");
                captured.get_or_insert_with(|| value.to_string());

                if proxy_images {
                    return Some(Cow::Owned(rewrite_image_to_proxy(
                        value,
                        &url_prefix,
                        &signing_keys,
                    )));
                }
            }
            Some(Cow::Borrowed(value))
        });

    let sanitized = builder.clean(html).to_string();
    let first_image_src = first_image_src
        .lock()
        .expect("BUG: first image capture mutex poisoned")
        .clone();
    (sanitized, first_image_src)
}

pub(super) fn has_dangerous_url_scheme(url: &str) -> bool {
    // Browsers strip ASCII TAB/LF/CR and C0 control characters from URL schemes
    // during parsing (WHATWG URL Standard), and form feed has historically been
    // a defensive test case for this sanitizer. Ammonia covers the standard cases;
    // this preserves strict ASCII-whitespace and ASCII-control normalization and
    // ASCII case-insensitive scheme checks before rewritten attrs are inspected.
    // Decode HTML entities first to prevent bypasses like javascript&#58; and
    // semicolonless numeric references such as javascript&#58alert(1).
    let url_decoded = decode_url_html_entities(url);
    let mut bytes = url_decoded
        .as_ref()
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && !b.is_ascii_control());

    let expected: &[u8] = match bytes.next() {
        Some(b'j' | b'J') => b"avascript:",
        Some(b'd' | b'D') => b"ata:",
        Some(b'v' | b'V') => b"bscript:",
        _ => return false,
    };

    for &e in expected {
        if bytes.next().map(|b| b.to_ascii_lowercase()) != Some(e) {
            return false;
        }
    }

    true
}

fn decode_url_html_entities(url: &str) -> Cow<'_, str> {
    if !url.contains('&') {
        return Cow::Borrowed(url);
    }
    let decoded = html_escape::decode_html_entities(url);
    let decoded_ref = decoded.as_ref();
    if !decoded_ref.contains("&#") {
        return decoded;
    }

    let mut output = String::with_capacity(decoded_ref.len());
    let mut remaining = decoded_ref;
    let mut changed = false;

    while let Some(idx) = remaining.find("&#") {
        output.push_str(&remaining[..idx]);
        let entity = &remaining[idx..];
        if let Some((ch, consumed)) = decode_numeric_char_ref(entity) {
            output.push(ch);
            remaining = &entity[consumed..];
            changed = true;
        } else {
            output.push_str("&#");
            remaining = &entity[2..];
        }
    }

    if !changed {
        return decoded;
    }

    output.push_str(remaining);
    Cow::Owned(output)
}

fn remove_empty_containers_from_html(html: &str) -> String {
    let mut fragment = Html::parse_fragment(html);
    remove_empty_containers(&mut fragment);
    serialize_fragment_body(&fragment)
}

fn html_whitespace_entity_len(rest: &str) -> Option<usize> {
    if rest.starts_with("&nbsp;") {
        return Some("&nbsp;".len());
    }

    let digits = rest.strip_prefix("&#")?;
    let (digits, radix) = digits
        .strip_prefix(['x', 'X'])
        .map_or((digits, 10), |hex_digits| (hex_digits, 16));
    let semicolon = digits.find(';')?;
    if semicolon == 0 {
        return None;
    }

    let codepoint = u32::from_str_radix(&digits[..semicolon], radix).ok()?;
    char::from_u32(codepoint)
        .is_some_and(char::is_whitespace)
        .then_some(rest.len() - digits.len() + semicolon + 1)
}

fn empty_text_candidate_end(html: &str, mut i: usize) -> usize {
    let bytes = html.as_bytes();
    while i < bytes.len() {
        // Fast-path: chunked ASCII whitespace check. We use iter().position
        // for SIMD acceleration over long contiguous whitespaces.
        let rest_bytes = &bytes[i..];
        let non_ws_idx = rest_bytes
            .iter()
            .position(|&b| !(b.is_ascii_whitespace() || b == 0x0B));

        match non_ws_idx {
            Some(0) => {} // first byte isn't ascii whitespace
            Some(offset) => {
                i += offset;
                continue;
            }
            None => {
                i = bytes.len();
                break;
            }
        }

        let b = bytes[i];
        let rest = &html[i..];
        if b == b'&' {
            if let Some(entity_len) = html_whitespace_entity_len(rest) {
                i += entity_len;
                continue;
            }
        }

        if b.is_ascii() {
            break;
        }

        let ch = rest
            .chars()
            .next()
            .expect("BUG: loop condition guarantees a non-empty remainder");
        // Keep Unicode-aware whitespace here (including vertical-tab) so empty-container
        // detection behavior does not narrow from prior releases.
        if !ch.is_whitespace() {
            break;
        }
        i += ch.len_utf8();
    }

    i
}

fn opening_tag_end(bytes: &[u8], mut i: usize) -> Option<usize> {
    let mut quote = None;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' | b'"' if quote == Some(bytes[i]) => quote = None,
            b'\'' | b'"' if quote.is_none() => quote = Some(bytes[i]),
            b'>' if quote.is_none() => return Some(i + 1),
            _ => {}
        }
        i += 1;
    }

    None
}

pub(super) fn may_have_empty_container(html: &str) -> bool {
    let bytes = html.as_bytes();
    let mut i = 0;
    let mut found = false;

    while let Some(open_offset) = bytes[i..].iter().position(|b| *b == b'<') {
        let open = i + open_offset;
        let tag_start = open + 1;
        if tag_start >= bytes.len()
            || matches!(
                bytes[tag_start],
                b'/' | b'!' | b'?' | b'0'..=b'9' | b'-' | b'.'
            )
        {
            i = tag_start;
            continue;
        }

        let tag_end = bytes[tag_start..]
            .iter()
            .position(|b| !b.is_ascii_alphanumeric())
            .map_or(bytes.len(), |offset| tag_start + offset);
        let tag = &html[tag_start..tag_end];
        if !is_container_tag(tag) {
            i = tag_end;
            continue;
        }

        let Some(open_end) = opening_tag_end(bytes, tag_end) else {
            return false;
        };

        let content_end = empty_text_candidate_end(html, open_end);
        if html[content_end..].starts_with("</") {
            let close_tag_start = content_end + 2;
            let close_tag_end = close_tag_start + tag.len();
            let has_matching_close = close_tag_end <= bytes.len()
                && html[close_tag_start..close_tag_end].eq_ignore_ascii_case(tag)
                && bytes[close_tag_end..]
                    .iter()
                    .position(|b| !b.is_ascii_whitespace())
                    .is_some_and(|offset| bytes[close_tag_end + offset] == b'>');
            found = has_matching_close;
        }

        i = if found { bytes.len() } else { open_end };
    }

    found
}

/// Remove empty container elements via single-pass bottom-up traversal.
/// Reversing the node list gives us children-before-parents order, so nested
/// empty containers collapse in one pass without re-parsing.
fn remove_empty_containers(fragment: &mut Html) {
    let mut ids: Vec<_> = fragment.tree.nodes().map(|n| n.id()).collect();
    ids.reverse(); // bottom-up: children before parents

    for id in ids {
        let is_empty_container = {
            let node = fragment
                .tree
                .get(id)
                .expect("BUG: node id collected from the same tree should exist");
            let Some(element) = node.value().as_element() else {
                continue;
            };
            let tag: &str = element.name.local.as_ref();
            is_container_tag(tag)
                && node.children().all(
                    |child| matches!(child.value(), scraper::Node::Text(t) if t.trim().is_empty()),
                )
        };

        if is_empty_container {
            let mut node = fragment
                .tree
                .get_mut(id)
                .expect("BUG: node id collected from the same tree should exist");
            node.detach();
        }
    }
}

#[cfg(test)]
mod entity_decode_tests {
    use super::*;

    #[test]
    fn decode_url_html_entities_covers_borrowed_invalid_and_multiple_refs() {
        assert_eq!(
            decode_url_html_entities("https://example.com").as_ref(),
            "https://example.com"
        );
        assert_eq!(decode_url_html_entities("a&amp;b").as_ref(), "a&b");
        assert_eq!(decode_url_html_entities("a&#oops").as_ref(), "a&#oops");
        assert_eq!(
            decode_url_html_entities("java&#115cript&#58alert(1)").as_ref(),
            "javascript:alert(1)"
        );
    }
}

#[cfg(test)]
mod is_container_tag_tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        assert!(!is_container_tag(""));
    }

    #[test]
    fn test_valid_container_tags() {
        let valid_tags = [
            "article",
            "aside",
            "div",
            "details",
            "footer",
            "figure",
            "figcaption",
            "header",
            "main",
            "nav",
            "p",
            "span",
            "section",
            "summary",
        ];

        for tag in valid_tags {
            assert!(is_container_tag(tag), "Failed for tag: {}", tag);
            let upper_tag = tag.to_uppercase();
            assert!(
                is_container_tag(&upper_tag),
                "Failed for tag: {}",
                upper_tag
            );
        }
    }

    #[test]
    fn test_invalid_container_tags() {
        let invalid_tags = [
            "a", "img", "script", "style", "div2", "span2", "b", "i", "strong", "em", "unknown",
        ];

        for tag in invalid_tags {
            assert!(!is_container_tag(tag), "Failed for invalid tag: {}", tag);
        }
    }
}
