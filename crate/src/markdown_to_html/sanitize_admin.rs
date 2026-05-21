use super::helpers::{is_safe_image_url, is_safe_link_url};
use crate::image_proxy::{
    is_external_http_url, rewrite_image_to_proxy, should_proxy_image,
    DEFAULT_IMAGE_PROXY_URL_PREFIX,
};
use ego_tree::NodeRef;
use scraper::{node::Node, Html};
use std::borrow::Cow;
use std::fmt::Write as _;

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

pub struct AdminHtmlOptions {
    pub nofollow_links: bool,
    pub proxy_images: bool,
    pub image_proxy_url_prefix: String,
    pub image_proxy_signing_keys: Vec<String>,
}

impl Default for AdminHtmlOptions {
    fn default() -> Self {
        Self {
            nofollow_links: false,
            proxy_images: false,
            image_proxy_url_prefix: DEFAULT_IMAGE_PROXY_URL_PREFIX.to_string(),
            image_proxy_signing_keys: Vec::new(),
        }
    }
}

/// Sanitize admin-authored HTML and apply render-time link/image policy in the
/// same parsed fragment traversal.
pub fn sanitize_admin_html_with_options(html: &str, opts: &AdminHtmlOptions) -> String {
    if html.is_empty() {
        return String::new();
    }
    let fragment = Html::parse_fragment(html);
    render_children(fragment.tree.root(), opts)
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr_val(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}

fn render_children(node: NodeRef<'_, Node>, opts: &AdminHtmlOptions) -> String {
    node.children()
        .map(|child| render_node(child, opts))
        .collect()
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

fn admin_img_src<'a>(src: &'a str, opts: &AdminHtmlOptions) -> Cow<'a, str> {
    if opts.proxy_images && should_proxy_image(src, &opts.image_proxy_url_prefix) {
        Cow::Owned(rewrite_image_to_proxy(
            src,
            &opts.image_proxy_url_prefix,
            &opts.image_proxy_signing_keys,
        ))
    } else {
        Cow::Borrowed(src)
    }
}

fn render_element_attrs(
    tag: &str,
    elem: &scraper::node::Element,
    children: &str,
    opts: &AdminHtmlOptions,
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
        let _ = write!(open, " {attr_name}=\"{}\"", escape_attr_val(&attr_value));
    }

    if has_external_href {
        let rel = if opts.nofollow_links {
            "nofollow ugc noopener"
        } else {
            "noopener"
        };
        let _ = write!(open, " rel=\"{}\" target=\"_blank\"", escape_attr_val(rel));
    }
    open.push('>');

    if is_void {
        open
    } else {
        format!("{open}{children}</{tag}>")
    }
}

fn render_node(node: NodeRef<'_, Node>, opts: &AdminHtmlOptions) -> String {
    match node.value() {
        Node::Text(text) => escape_text(text),
        Node::Element(elem) => {
            let tag: &str = elem.name.local.as_ref();

            if DANGEROUS_ADMIN_TAGS.contains(&tag) {
                return String::new();
            }

            let children = render_children(node, opts);

            if !ALLOWED_ADMIN_TAGS.contains(&tag) {
                return children;
            }

            render_element_attrs(tag, elem, &children, opts)
        }
        Node::Document | Node::Fragment => render_children(node, opts),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests;
