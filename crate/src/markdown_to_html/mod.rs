mod helpers;
mod sanitize_admin;

use comrak::{format_html, parse_document, Arena, Options};
use scraper::Html;

use crate::image_proxy::{
    is_external_http_url, rewrite_image_to_proxy, should_proxy_image,
    DEFAULT_IMAGE_PROXY_URL_PREFIX,
};
use crate::serialize_fragment_body;
use helpers::{collect_urls, set_attr_md, walk_and_sanitize_urls};
use sanitize_admin::sanitize_admin_html;

pub struct MarkdownUrlsResult {
    pub link_urls: Vec<String>,
    pub image_urls: Vec<String>,
}

/// Options for rendering markdown to HTML.
pub struct MarkdownRenderOptions {
    /// false = escape HTML (non-admin), true = allow + sanitize (admin)
    pub allow_html: bool,
    /// true = add nofollow ugc to external links (non-admin)
    pub nofollow_links: bool,
    /// true = rewrite external image URLs through the configured proxy prefix
    pub proxy_images: bool,
    /// URL path prefix prepended to proxied image URLs (e.g. `/proxy/`).
    pub image_proxy_url_prefix: String,
    /// hex-encoded HMAC-SHA256 keys, newest first; empty = dev mode (unsigned)
    pub image_proxy_signing_keys: Vec<String>,
}

impl Default for MarkdownRenderOptions {
    fn default() -> Self {
        Self {
            allow_html: false,
            nofollow_links: true,
            proxy_images: true,
            image_proxy_url_prefix: DEFAULT_IMAGE_PROXY_URL_PREFIX.to_string(),
            image_proxy_signing_keys: Vec::new(),
        }
    }
}

/// Render markdown to HTML with structured options.
#[allow(clippy::missing_panics_doc)]
pub fn render_markdown_to_html_with_options(text: &str, opts: &MarkdownRenderOptions) -> String {
    let mut options = Options::default();
    options.extension.autolink = true;
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.render.r#unsafe = opts.allow_html;
    options.render.escape = !opts.allow_html;
    options.render.hardbreaks = true;

    let arena = Arena::new();
    let root = parse_document(&arena, text, &options);

    // Always sanitize URLs in the AST
    walk_and_sanitize_urls(root);

    let mut html = String::new();
    format_html(root, &options, &mut html).expect("BUG: format_html failed");

    // If admin HTML is allowed, run the permissive sanitizer
    let html = if opts.allow_html {
        sanitize_admin_html(&html)
    } else {
        html
    };

    let result = post_process_html(&html, opts);
    result.trim().to_string()
}

pub fn extract_markdown_urls_sync(text: &str) -> MarkdownUrlsResult {
    let mut options = Options::default();
    options.extension.autolink = true;

    let arena = Arena::new();
    let root = parse_document(&arena, text, &options);

    let mut links = Vec::new();
    let mut images = Vec::new();
    collect_urls(root, &mut links, &mut images);

    MarkdownUrlsResult {
        link_urls: links,
        image_urls: images,
    }
}

fn post_process_html(html: &str, opts: &MarkdownRenderOptions) -> String {
    if html.is_empty() {
        return String::new();
    }
    let mut fragment = Html::parse_fragment(html);
    let node_ids: Vec<_> = fragment.tree.nodes().map(|n| n.id()).collect();
    for id in node_ids {
        let mut node = fragment
            .tree
            .get_mut(id)
            .expect("BUG: node id collected from the same tree should exist");
        let scraper::Node::Element(ref mut element) = *node.value() else {
            continue;
        };
        let tag: &str = element.name.local.as_ref();
        match tag {
            "a" => {
                let href_val = element
                    .attrs
                    .iter()
                    .find(|(n, _)| n.local.as_ref() == "href")
                    .map(|(_, v)| v.as_ref().to_string());
                match href_val.as_deref() {
                    Some("") | None => {
                        element.attrs.retain(|(n, _)| n.local.as_ref() != "href");
                    }
                    Some(url) if is_external_http_url(url) => {
                        if opts.nofollow_links {
                            set_attr_md(&mut element.attrs, "rel", "nofollow ugc noopener");
                        } else {
                            set_attr_md(&mut element.attrs, "rel", "noopener");
                        }
                        set_attr_md(&mut element.attrs, "target", "_blank");
                    }
                    _ => {}
                }
            }
            "img" => {
                let src_val = element
                    .attrs
                    .iter()
                    .find(|(n, _)| n.local.as_ref() == "src")
                    .map(|(_, v)| v.as_ref().to_string());
                match src_val.as_deref() {
                    Some("") => {
                        element.attrs.retain(|(n, _)| n.local.as_ref() != "src");
                    }
                    Some(url)
                        if opts.proxy_images
                            && should_proxy_image(url, &opts.image_proxy_url_prefix) =>
                    {
                        let proxied = rewrite_image_to_proxy(
                            url,
                            &opts.image_proxy_url_prefix,
                            &opts.image_proxy_signing_keys,
                        );
                        set_attr_md(&mut element.attrs, "src", &proxied);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    serialize_fragment_body(&fragment)
}
