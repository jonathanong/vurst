use std::future::Future;

use napi::bindgen_prelude::Buffer;
use vurst_node::{
    extract_markdown_urls, html_to_embedding_text, render_markdown_to_html,
    render_markdown_to_html_batch, NapiMarkdownRenderOptions,
};

fn block_on<T>(future: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should build")
        .block_on(future)
}

fn buffer(input: impl Into<Vec<u8>>) -> Buffer {
    Buffer::from(input.into())
}

fn oversized_buffer() -> Buffer {
    Buffer::from(vec![b'x'; 10 * 1024 * 1024 + 1])
}

fn assert_error_contains<T>(result: napi::Result<T>, expected: &str) {
    match result {
        Ok(_) => panic!("expected error containing {expected:?}"),
        Err(error) => assert!(
            error.to_string().contains(expected),
            "expected {expected:?} in {error}"
        ),
    }
}

#[test]
fn markdown_bindings_cover_render_urls_embedding_and_errors() {
    let urls = block_on(extract_markdown_urls(buffer(
        "[site](https://example.com) ![img](https://example.com/a.png) [bad](javascript:bad)",
    )))
    .expect("URL extraction should succeed");
    assert_eq!(urls.link_urls, vec!["https://example.com"]);
    assert_eq!(urls.image_urls, vec!["https://example.com/a.png"]);

    let rendered = block_on(render_markdown_to_html(
        buffer("<b>safe</b> [site](https://example.com)"),
        Some(NapiMarkdownRenderOptions {
            allow_html: Some(true),
            nofollow_links: Some(false),
            proxy_images: Some(false),
            image_proxy_signing_keys: Some(vec![]),
            ..NapiMarkdownRenderOptions::default()
        }),
    ))
    .expect("markdown should render");
    let rendered = std::str::from_utf8(&rendered).unwrap();
    assert!(rendered.contains("<b>safe</b>"));
    assert!(rendered.contains("rel=\"noopener\""));

    let rendered_batch = block_on(render_markdown_to_html_batch(
        vec![buffer("**One**"), buffer("![x](https://example.com/x.png)")],
        None,
    ))
    .expect("markdown batch should render");
    assert_eq!(rendered_batch.len(), 2);

    let embedding = block_on(html_to_embedding_text(buffer(
        "<article>Hello <b>world</b></article>",
    )))
    .expect("embedding text should render");
    assert!(embedding.contains("Hello"));

    assert_error_contains(
        block_on(extract_markdown_urls(oversized_buffer())),
        "Input too large",
    );
    assert_error_contains(
        block_on(extract_markdown_urls(buffer(vec![0xff]))),
        "Invalid UTF-8 in text",
    );
    assert_error_contains(
        block_on(render_markdown_to_html(buffer(vec![0xff]), None)),
        "Invalid UTF-8 in text",
    );
    assert_error_contains(
        block_on(render_markdown_to_html(oversized_buffer(), None)),
        "Input too large",
    );
    assert_error_contains(
        block_on(render_markdown_to_html_batch(
            vec![buffer(vec![0xff])],
            None,
        )),
        "Invalid UTF-8 in inputs[0]",
    );
    assert_error_contains(
        block_on(render_markdown_to_html_batch(
            vec![oversized_buffer()],
            None,
        )),
        "Input too large",
    );
    assert_error_contains(
        block_on(html_to_embedding_text(buffer(vec![0xff]))),
        "Invalid UTF-8",
    );
    assert_error_contains(
        block_on(html_to_embedding_text(oversized_buffer())),
        "Input too large",
    );
}
