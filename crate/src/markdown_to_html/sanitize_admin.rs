use super::helpers::{is_safe_image_url, is_safe_link_url};
use ego_tree::NodeRef;
use scraper::{node::Node, Html};

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

/// Attributes allowed on `<a>` tags.
const ALLOWED_A_ATTRS: &[&str] = &["href", "title"];

/// Attributes allowed on `<img>` tags.
const ALLOWED_IMG_ATTRS: &[&str] = &["src", "alt", "title", "width", "height"];

/// Attributes allowed on any element.
const ALLOWED_GLOBAL_ATTRS: &[&str] = &["class", "id"];

/// Sanitize admin-authored HTML with a permissive allowlist.
///
/// Allows rich formatting tags while stripping dangerous elements
/// (`<script>`, `<iframe>`, event handlers, `style` attributes, etc.).
pub fn sanitize_admin_html(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }
    let fragment = Html::parse_fragment(html);
    render_children(fragment.tree.root())
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

fn render_children(node: NodeRef<'_, Node>) -> String {
    node.children().map(render_node).collect()
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

fn render_element_attrs(tag: &str, elem: &scraper::node::Element, children: &str) -> String {
    let is_void = matches!(tag, "br" | "hr" | "img");

    let mut open = format!("<{tag}");
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
        open.push_str(&format!(" {attr_name}=\"{}\"", escape_attr_val(val)));
    }
    open.push('>');

    if is_void {
        open
    } else {
        format!("{open}{children}</{tag}>")
    }
}

fn render_node(node: NodeRef<'_, Node>) -> String {
    match node.value() {
        Node::Text(text) => escape_text(text),
        Node::Element(elem) => {
            let tag: &str = elem.name.local.as_ref();

            if DANGEROUS_ADMIN_TAGS.contains(&tag) {
                return String::new();
            }

            let children = render_children(node);

            if !ALLOWED_ADMIN_TAGS.contains(&tag) {
                return children;
            }

            render_element_attrs(tag, elem, &children)
        }
        Node::Document | Node::Fragment => render_children(node),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests;
