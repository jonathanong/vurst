use super::helpers::{is_safe_image_url, is_safe_link_url};
use crate::image_proxy::{
    is_external_http_url, rewrite_image_to_proxy, should_proxy_image,
    DEFAULT_IMAGE_PROXY_URL_PREFIX,
};
use ego_tree::NodeRef;
use scraper::{node::Node, Html};
use std::borrow::Cow;

/// Tags allowed in admin HTML content (permissive allowlist).
const ALLOWED_ADMIN_TAGS: &[&str] = &[
    "p",
    "br",
    "a",
    "strong",
    "em",
    "b",
    "i",
    "u",
    "s",
    "del",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "ul",
    "ol",
    "li",
    "blockquote",
    "pre",
    "code",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
    "img",
    "hr",
    "figure",
    "figcaption",
    "details",
    "summary",
    "sup",
    "sub",
    "span",
    "div",
    "dl",
    "dt",
    "dd",
];

/// Tags removed entirely including all descendants.
const DANGEROUS_ADMIN_TAGS: &[&str] = &[
    "script", "style", "iframe", "object", "embed", "form", "input", "button", "select",
    "textarea", "meta", "base", "svg",
];

/// Void tags rendered without closing tags.
const VOID_ADMIN_TAGS: &[&str] = &["br", "hr", "img"];

/// Attributes allowed on `<a>` tags.
const ALLOWED_A_ATTRS: &[&str] = &["href", "title"];

/// Attributes allowed on `<img>` tags.
const ALLOWED_IMG_ATTRS: &[&str] = &["src", "alt", "title", "width", "height"];

/// Attributes allowed on any element.
const ALLOWED_GLOBAL_ATTRS: &[&str] = &["class", "id"];

pub struct AdminHtmlOptions<'a> {
    pub nofollow_links: bool,
    pub proxy_images: bool,
    pub image_proxy_url_prefix: &'a str,
    pub image_proxy_signing_keys: &'a [String],
}

impl Default for AdminHtmlOptions<'_> {
    fn default() -> Self {
        Self {
            nofollow_links: false,
            proxy_images: false,
            image_proxy_url_prefix: DEFAULT_IMAGE_PROXY_URL_PREFIX,
            image_proxy_signing_keys: &[],
        }
    }
}

/// Sanitize admin-authored HTML and apply render-time link/image policy in the
/// same parsed fragment traversal.
pub fn sanitize_admin_html_with_options(html: &str, opts: &AdminHtmlOptions<'_>) -> String {
    if html.is_empty() {
        return String::new();
    }
    let fragment = Html::parse_fragment(html);
    render_children(fragment.tree.root(), opts)
}

fn escape_text(s: &str) -> Cow<'_, str> {
    // ⚡ Bolt: Pass u8 to avoid UTF-8 decoding overhead when searching for ASCII chars
    escape_text_chars(s, |b| match b {
        b'&' => Some("&amp;"),
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        _ => None,
    })
}

fn escape_attr_val(s: &str) -> Cow<'_, str> {
    // ⚡ Bolt: Pass u8 to avoid UTF-8 decoding overhead when searching for ASCII chars
    escape_text_chars(s, |b| match b {
        b'&' => Some("&amp;"),
        b'"' => Some("&quot;"),
        b'<' => Some("&lt;"),
        _ => None,
    })
}

fn escape_text_chars(
    s: &str,
    find_replacement: impl Fn(u8) -> Option<&'static str>,
) -> Cow<'_, str> {
    let mut last_idx = 0;
    let mut out: Option<String> = None;

    // ⚡ Bolt: Iterate over raw bytes to avoid decoding overhead.
    // ASCII chars are valid UTF-8 and don't overlap with multibyte sequences.
    for (i, &b) in s.as_bytes().iter().enumerate() {
        let Some(escaped) = find_replacement(b) else {
            continue;
        };

        if out.is_none() {
            out = Some(String::with_capacity(s.len().saturating_add(16)));
        }
        let out_str = out.as_mut().unwrap();

        out_str.push_str(&s[last_idx..i]);
        out_str.push_str(escaped);
        last_idx = i + 1;
    }

    if let Some(mut out_str) = out {
        out_str.push_str(&s[last_idx..]);
        Cow::Owned(out_str)
    } else {
        Cow::Borrowed(s)
    }
}

fn render_children(node: NodeRef<'_, Node>, opts: &AdminHtmlOptions<'_>) -> String {
    let mut out = String::new();
    for child in node.children() {
        match render_node(child, opts) {
            Cow::Borrowed(text) => out.push_str(text),
            Cow::Owned(text) => out.push_str(&text),
        }
    }
    out
}

fn is_allowed_attr(tag: &str, attr_name: &str) -> bool {
    if attr_name == "style" || attr_name.starts_with("on") {
        return false;
    }
    if ALLOWED_GLOBAL_ATTRS.contains(&attr_name) {
        return true;
    }
    match tag {
        "a" => ALLOWED_A_ATTRS.contains(&attr_name),
        "img" => ALLOWED_IMG_ATTRS.contains(&attr_name),
        _ => false,
    }
}

fn admin_img_src<'a>(src: &'a str, opts: &AdminHtmlOptions<'_>) -> Cow<'a, str> {
    if opts.proxy_images && should_proxy_image(src, opts.image_proxy_url_prefix) {
        Cow::Owned(rewrite_image_to_proxy(
            src,
            opts.image_proxy_url_prefix,
            opts.image_proxy_signing_keys,
        ))
    } else {
        Cow::Borrowed(src)
    }
}

fn render_element_attrs(
    tag: &str,
    elem: &scraper::node::Element,
    children: &str,
    opts: &AdminHtmlOptions<'_>,
) -> String {
    let is_void = VOID_ADMIN_TAGS.contains(&tag);

    let mut open = format!("<{tag}");
    let mut has_external_href = false;
    for (name, val) in &elem.attrs {
        let attr_name: &str = name.local.as_ref();
        if !is_allowed_attr(tag, attr_name) {
            continue;
        }
        if attr_name == "href" && !is_safe_link_url(val) {
            continue;
        }
        if attr_name == "src" && !is_safe_image_url(val) {
            continue;
        }
        if tag == "a" && attr_name == "href" {
            has_external_href = is_external_http_url(val);
        }
        let attr_value = if tag == "img" && attr_name == "src" {
            admin_img_src(val, opts)
        } else {
            Cow::Borrowed(val.as_ref())
        };

        write_attr_escape(&mut open, attr_name, escape_attr_val(&attr_value));
    }

    if has_external_href {
        let rel = if opts.nofollow_links {
            "nofollow ugc noopener"
        } else {
            "noopener"
        };
        write_attr_escape(&mut open, "rel", escape_attr_val(rel));
        open.push_str(" target=\"_blank\"");
    }
    open.push('>');

    if is_void {
        open
    } else {
        format!("{open}{children}</{tag}>")
    }
}

fn write_attr_escape(open: &mut String, attr_name: &str, escaped_value: Cow<'_, str>) {
    // ⚡ Bolt: Replace the fmt::Write macro with direct push/push_str calls for speed
    open.push(' ');
    open.push_str(attr_name);
    open.push_str("=\"");
    open.push_str(escaped_value.as_ref());
    open.push('"');
}

fn render_node<'a>(node: NodeRef<'a, Node>, opts: &'a AdminHtmlOptions<'a>) -> Cow<'a, str> {
    match node.value() {
        Node::Text(text) => escape_text(text),
        Node::Element(elem) => {
            let tag: &str = elem.name.local.as_ref();

            if DANGEROUS_ADMIN_TAGS.contains(&tag) {
                return Cow::Borrowed("");
            }

            let children = render_children(node, opts);

            if !ALLOWED_ADMIN_TAGS.contains(&tag) {
                return Cow::Owned(children);
            }

            Cow::Owned(render_element_attrs(tag, elem, &children, opts))
        }
        Node::Document | Node::Fragment => Cow::Owned(render_children(node, opts)),
        _ => Cow::Borrowed(""),
    }
}

#[cfg(test)]
mod tests;
