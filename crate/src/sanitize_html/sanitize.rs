use crate::image_proxy::{
    rewrite_image_to_proxy, should_proxy_image, DEFAULT_IMAGE_PROXY_URL_PREFIX,
};
use crate::serialize_fragment_body;
use ammonia::{Builder, UrlRelative};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

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

/// Container tag names eligible for empty-element cleanup.
const CONTAINER_TAGS: &[&str] = &[
    "div",
    "span",
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "nav",
    "main",
    "figure",
    "figcaption",
    "details",
    "summary",
    "p",
];

/// Options for [`sanitize_rss_html_sync`].
pub struct SanitizeRssHtmlOptions {
    /// When `true`, rewrite external `<img src>` URLs through the configured
    /// image-proxy prefix.
    pub proxy_images: bool,
    /// URL path prefix prepended to proxied image URLs (e.g. `/proxy/`).
    pub image_proxy_url_prefix: String,
    /// Hex-encoded HMAC-SHA256 signing keys (newest first). Empty = dev mode (no sig).
    pub image_proxy_signing_keys: Vec<String>,
}

impl Default for SanitizeRssHtmlOptions {
    fn default() -> Self {
        Self {
            proxy_images: false,
            image_proxy_url_prefix: DEFAULT_IMAGE_PROXY_URL_PREFIX.to_string(),
            image_proxy_signing_keys: Vec::new(),
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
/// - Optionally rewrites external `<img src>` URLs through the configured image-proxy prefix
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
    // link/image attributes. The closure also preserves our image proxy
    // rewriting and captures the first original external image URL.
    let (sanitized, first_image_src) = sanitize_with_ammonia(html, opts);

    // Pass 2: Keep the historical cleanup of empty RSS layout containers left
    // behind after attribute/tag stripping.
    let html = remove_empty_containers_from_html(&sanitized);

    SanitizeRssHtmlResult {
        html,
        first_image_src,
    }
}

fn sanitize_with_ammonia(html: &str, opts: &SanitizeRssHtmlOptions) -> (String, Option<String>) {
    let proxy_images = opts.proxy_images;
    let signing_keys = opts.image_proxy_signing_keys.clone();
    let url_prefix = opts.image_proxy_url_prefix.clone();
    let url_prefix_for_filter = url_prefix.clone();
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
            if tag == "img" && attr == "src" && should_proxy_image(value, &url_prefix_for_filter) {
                let mut captured = first_image_src_filter
                    .lock()
                    .expect("BUG: first image capture mutex poisoned");
                captured.get_or_insert_with(|| value.to_string());

                if proxy_images {
                    return Some(Cow::Owned(rewrite_image_to_proxy(
                        value,
                        &url_prefix_for_filter,
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

fn has_dangerous_url_scheme(url: &str) -> bool {
    // Browsers strip ASCII TAB/LF/CR from URL schemes during parsing (WHATWG URL
    // Standard), and form feed has historically been a defensive test case for
    // this sanitizer. Ammonia covers the standard cases; this preserves our
    // stricter ASCII-whitespace normalization before it sees rewritten attrs.
    const MAX_SCHEME_LEN: usize = 11; // "javascript:".len()
    const DANGEROUS_URL_SCHEMES: &[&str] = &["javascript:", "data:", "vbscript:"];
    let normalized: String = url
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .take(MAX_SCHEME_LEN)
        .collect();
    DANGEROUS_URL_SCHEMES
        .iter()
        .any(|scheme| normalized.starts_with(scheme))
}

fn remove_empty_containers_from_html(html: &str) -> String {
    let mut fragment = Html::parse_fragment(html);
    remove_empty_containers(&mut fragment);
    serialize_fragment_body(&fragment)
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
            CONTAINER_TAGS.contains(&tag)
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
