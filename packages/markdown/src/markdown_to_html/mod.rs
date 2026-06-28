mod helpers;
mod sanitize_admin;

use comrak::html::{render_sourcepos, ChildRendering};
use comrak::nodes::NodeValue;
use comrak::{create_formatter, parse_document, Arena, Options};
use std::borrow::Cow;
use std::fmt::Write as _;

use helpers::{collect_plaintext_links, collect_urls, walk_and_sanitize_urls};
use sanitize_admin::{sanitize_admin_html_with_options, AdminHtmlOptions};
use vurst_shared::image_proxy::{
    is_external_http_url, rewrite_image_to_proxy, should_proxy_image,
    DEFAULT_IMAGE_PROXY_URL_PREFIX,
};

#[derive(Debug, PartialEq)]
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

#[derive(Default)]
struct MarkdownHtmlFormatOptions<'a> {
    nofollow_links: bool,
    proxy_images: bool,
    image_proxy_url_prefix: &'a str,
    image_proxy_signing_keys: &'a [String],
}

impl<'a> From<&'a MarkdownRenderOptions> for MarkdownHtmlFormatOptions<'a> {
    fn from(opts: &'a MarkdownRenderOptions) -> Self {
        Self {
            nofollow_links: opts.nofollow_links,
            proxy_images: opts.proxy_images,
            image_proxy_url_prefix: &opts.image_proxy_url_prefix,
            image_proxy_signing_keys: &opts.image_proxy_signing_keys,
        }
    }
}

fn rendered_image_src<'a>(url: &'a str, opts: &MarkdownHtmlFormatOptions<'_>) -> Cow<'a, str> {
    if opts.proxy_images && should_proxy_image(url, opts.image_proxy_url_prefix) {
        Cow::Owned(rewrite_image_to_proxy(
            url,
            opts.image_proxy_url_prefix,
            opts.image_proxy_signing_keys,
        ))
    } else {
        Cow::Borrowed(url)
    }
}

fn should_render_nested_link(node: &comrak::nodes::AstNode<'_>, opts: &Options) -> bool {
    if !opts.parse.relaxed_autolinks {
        return true;
    }

    match node.parent() {
        Some(parent) => !matches!(parent.data().value, NodeValue::Link(_)),
        None => true,
    }
}

create_formatter!(MarkdownHtmlFormatter<MarkdownHtmlFormatOptions<'a>>, {
    NodeValue::Link(ref link) => |context, node, entering| {
        if !should_render_nested_link(node, context.options) {
            return Ok(ChildRendering::HTML);
        }

        if entering {
            context.write_str("<a")?;
            render_sourcepos(context, node)?;
            if !link.url.is_empty() {
                context.write_str(" href=\"")?;
                context.escape_href(&link.url)?;
                context.write_str("\"")?;
            }
            if !link.title.is_empty() {
                context.write_str(" title=\"")?;
                context.escape(&link.title)?;
                context.write_str("\"")?;
            }
            if is_external_http_url(&link.url) {
                let rel = if context.user.nofollow_links {
                    "nofollow ugc noopener"
                } else {
                    "noopener"
                };
                context.write_str(" rel=\"")?;
                context.write_str(rel)?;
                context.write_str("\" target=\"_blank\"")?;
            }
            context.write_str(">")?;
        } else {
            context.write_str("</a>")?;
        }
    },
    NodeValue::Image(ref image) => |context, node, entering| {
        if entering {
            if context.options.render.figure_with_caption {
                context.write_str("<figure>")?;
            }
            context.write_str("<img")?;
            render_sourcepos(context, node)?;
            if !image.url.is_empty() {
                let src = rendered_image_src(&image.url, &context.user);
                context.write_str(" src=\"")?;
                context.escape_href(src.as_ref())?;
                context.write_str("\"")?;
            }
            context.write_str(" alt=\"")?;
            return Ok(ChildRendering::Plain);
        }

        if !image.title.is_empty() {
            context.write_str("\" title=\"")?;
            context.escape(&image.title)?;
        }
        context.write_str("\" />")?;
        if context.options.render.figure_with_caption {
            if !image.title.is_empty() {
                context.write_str("<figcaption>")?;
                context.escape(&image.title)?;
                context.write_str("</figcaption>")?;
            }
            context.write_str("</figure>")?;
        }
    },
});

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
    MarkdownHtmlFormatter::format_document(root, &options, &mut html, opts.into())
        .expect("BUG: format_html failed");

    let html = if opts.allow_html {
        sanitize_admin_html_with_options(
            &html,
            &AdminHtmlOptions {
                nofollow_links: opts.nofollow_links,
                proxy_images: opts.proxy_images,
                image_proxy_url_prefix: &opts.image_proxy_url_prefix,
                image_proxy_signing_keys: &opts.image_proxy_signing_keys,
            },
        )
    } else {
        html
    };

    html.trim().to_string()
}

pub fn extract_markdown_urls_sync(text: &str) -> MarkdownUrlsResult {
    let mut options = Options::default();
    options.extension.autolink = true;

    let arena = Arena::new();
    let root = parse_document(&arena, text, &options);

    let mut links = Vec::new();
    let mut images = Vec::new();
    collect_urls(root, &mut links, &mut images);
    collect_plaintext_links(root, &mut links);

    // Dedup preserving first-seen order (bare-domain scan may repeat for
    // duplicate mentions; also guards against any overlap with autolinks).
    let mut seen = std::collections::HashSet::new();
    let keep: Vec<bool> = links.iter().map(|url| seen.insert(url.as_str())).collect();
    let mut keep_iter = keep.into_iter();
    links.retain(|_| keep_iter.next().unwrap());

    MarkdownUrlsResult {
        link_urls: links,
        image_urls: images,
    }
}

#[cfg(test)]
mod tests;
