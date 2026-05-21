use comrak::nodes::NodeValue;

pub const LINK_SCHEMES: &[&str] = &["http", "https", "mailto", "tel"];
pub const IMAGE_SCHEMES: &[&str] = &["http", "https"];

pub fn extract_scheme(url: &str) -> Option<String> {
    let colon_idx = url.find(':')?;
    let scheme = &url[..colon_idx];
    // RFC 3986: scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    // The first character must be a letter; digits are only valid after position 0.
    let mut chars = scheme.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    if chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
        Some(scheme.to_lowercase())
    } else {
        None
    }
}

pub fn is_safe_link_url(url: &str) -> bool {
    let url = url.trim();
    // Reject empty or protocol-relative URLs (//host).
    // Protocol-relative URLs bypass the scheme allowlist and point to external hosts.
    !url.is_empty()
        && !url.starts_with("//")
        && if url.contains(':') {
            // URL contains ':', so a valid scheme is required.
            // If extract_scheme returns None the scheme is malformed (e.g. "1abc:").
            extract_scheme(url).is_some_and(|scheme| LINK_SCHEMES.contains(&scheme.as_str()))
        } else {
            // No ':', so it is a relative URL (e.g. /path, ../path) — safe.
            true
        }
}

pub fn is_safe_image_url(url: &str) -> bool {
    let url = url.trim();
    // Reject empty or protocol-relative URLs (//host).
    !url.is_empty()
        && !url.starts_with("//")
        && if url.contains(':') {
            // URL contains ':', so a valid scheme is required.
            extract_scheme(url).is_some_and(|scheme| IMAGE_SCHEMES.contains(&scheme.as_str()))
        } else {
            // No ':', so it is a relative URL — safe.
            true
        }
}

pub fn set_attr_md(
    attrs: &mut Vec<(markup5ever::QualName, markup5ever::tendril::StrTendril)>,
    name: &str,
    value: &str,
) {
    use markup5ever::{ns, LocalName, QualName};
    attrs.retain(|(n, _)| n.local.as_ref() != name);
    attrs.push((
        QualName::new(None, ns!(), LocalName::from(name)),
        value.into(),
    ));
}

pub fn walk_and_sanitize_urls<'a>(node: &'a comrak::nodes::AstNode<'a>) {
    {
        let mut data = node.data.borrow_mut();
        match &mut data.value {
            NodeValue::Link(ref mut link) if !is_safe_link_url(&link.url) => {
                link.url.clear();
            }
            NodeValue::Image(ref mut link) if !is_safe_image_url(&link.url) => {
                link.url.clear();
            }
            _ => {}
        }
    }
    for child in node.children() {
        walk_and_sanitize_urls(child);
    }
}

pub fn collect_urls<'a>(
    node: &'a comrak::nodes::AstNode<'a>,
    links: &mut Vec<String>,
    images: &mut Vec<String>,
) {
    {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Link(link) if is_safe_link_url(&link.url) && !link.url.is_empty() => {
                links.push(link.url.clone());
            }
            NodeValue::Image(link) if is_safe_image_url(&link.url) && !link.url.is_empty() => {
                images.push(link.url.clone());
            }
            _ => {}
        }
    }
    for child in node.children() {
        collect_urls(child, links, images);
    }
}

#[cfg(test)]
mod tests;
