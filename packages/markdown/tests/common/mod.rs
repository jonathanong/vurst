use vurst_markdown_node::markdown_to_html::MarkdownRenderOptions;

#[allow(dead_code)]
pub fn default_opts() -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        proxy_images: false,
        ..MarkdownRenderOptions::default()
    }
}

pub fn admin_opts() -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        allow_html: true,
        nofollow_links: false,
        proxy_images: false,
        ..MarkdownRenderOptions::default()
    }
}
