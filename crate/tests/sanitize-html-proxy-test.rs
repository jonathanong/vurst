use vurst::{sanitize_rss_html_sync, SanitizeRssHtmlOptions};

fn sanitize(html: &str) -> String {
    sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default()).html
}

// === Image proxying ===

#[test]
fn proxy_off_preserves_external_img_src() {
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default());
    assert!(result.html.contains("https://example.com/photo.jpg"));
    assert!(!result.html.contains("/proxy/"));
}

#[test]
fn proxy_on_rewrites_external_img_src() {
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize_rss_html_sync(
        html,
        &SanitizeRssHtmlOptions {
            proxy_images: true,
            ..SanitizeRssHtmlOptions::default()
        },
    );
    assert!(!result.html.contains("https://example.com"));
    assert!(result.html.contains("/proxy/"));
}

#[test]
fn proxy_on_with_signing_key_adds_sig() {
    let key = "deadbeef".repeat(8); // 64 hex chars = 32 bytes
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize_rss_html_sync(
        html,
        &SanitizeRssHtmlOptions {
            proxy_images: true,
            image_proxy_signing_keys: vec![key],
            ..SanitizeRssHtmlOptions::default()
        },
    );
    assert!(result.html.contains("sig="));
}

#[test]
fn proxy_skips_relative_img_src() {
    let html = r#"<img src="/local/image.jpg">"#;
    let result = sanitize_rss_html_sync(
        html,
        &SanitizeRssHtmlOptions {
            proxy_images: true,
            ..SanitizeRssHtmlOptions::default()
        },
    );
    assert!(result.html.contains("/local/image.jpg"));
    assert!(!result.html.contains("/proxy/"));
}

#[test]
fn proxy_skips_already_proxied_src() {
    let html = r#"<img src="/proxy/abc123">"#;
    let result = sanitize_rss_html_sync(
        html,
        &SanitizeRssHtmlOptions {
            proxy_images: true,
            ..SanitizeRssHtmlOptions::default()
        },
    );
    assert!(!result.html.contains("/proxy//proxy/"));
    assert_eq!(result.html.matches("/proxy/").count(), 1);
}

// === First image extraction ===

#[test]
fn first_image_src_captured_for_external_url() {
    let html = r#"<p>Text</p><img src="https://example.com/photo.jpg" alt="photo">"#;
    let result = sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default());
    assert_eq!(
        result.first_image_src.as_deref(),
        Some("https://example.com/photo.jpg")
    );
}

#[test]
fn first_image_src_skips_relative_finds_external() {
    let html = r#"<img src="/relative.jpg"><img src="https://example.com/photo.jpg">"#;
    let result = sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default());
    assert_eq!(
        result.first_image_src.as_deref(),
        Some("https://example.com/photo.jpg")
    );
}

#[test]
fn first_image_src_none_when_no_external_img() {
    let html = r#"<p>No images</p>"#;
    let result = sanitize_rss_html_sync(html, &SanitizeRssHtmlOptions::default());
    assert!(result.first_image_src.is_none());
}

#[test]
fn first_image_src_is_original_not_proxied() {
    let html = r#"<img src="https://example.com/photo.jpg">"#;
    let result = sanitize_rss_html_sync(
        html,
        &SanitizeRssHtmlOptions {
            proxy_images: true,
            ..SanitizeRssHtmlOptions::default()
        },
    );
    // first_image_src is the raw URL before proxying
    assert_eq!(
        result.first_image_src.as_deref(),
        Some("https://example.com/photo.jpg")
    );
    // but the rendered HTML has the proxied src
    assert!(result.html.contains("/proxy/"));
}

// === Realistic content ===

#[test]
fn sanitizes_wordpress_content() {
    let html = r#"
<div class="entry-content" id="post-content">
    <p style="font-size: 18px; line-height: 1.8;" class="has-large-font-size">
        Welcome to this <strong>amazing</strong> article about travel rewards.
    </p>
    <figure class="wp-block-image size-large" data-widget="gallery">
        <img src="https://example.com/photo.jpg"
             srcset="https://example.com/photo-300.jpg 300w, https://example.com/photo-768.jpg 768w"
             sizes="(max-width: 768px) 100vw, 768px"
             width="768" height="512"
             class="wp-image-12345"
             alt="Travel photo">
    </figure>
    <p class="wp-block-paragraph">Here is some <a href="https://example.com" class="external-link" onclick="trackClick()">useful link</a>.</p>
    <div class="sharedaddy sd-sharing-enabled" data-sharing="true">
        <div class="sd-content">
            <ul>
                <li class="share-facebook"><a href="https://facebook.com/share">Share on Facebook</a></li>
                <li class="share-twitter"><a href="https://twitter.com/share">Share on Twitter</a></li>
            </ul>
        </div>
    </div>
    <script>
        window.trackPageView('article-123');
    </script>
    <style>
        .entry-content { max-width: 800px; }
    </style>
</div>
    "#;

    let result = sanitize(html);

    // Content preserved
    assert!(result.contains("amazing"));
    assert!(result.contains("article about travel rewards"));
    assert!(result.contains("useful link"));
    assert!(result.contains("Share on Facebook"));

    // Dangerous elements removed
    assert!(!result.contains("<script"));
    assert!(!result.contains("trackPageView"));
    assert!(!result.contains("<style"));

    // Attributes stripped
    assert!(!result.contains("class="));
    assert!(!result.contains("style="));
    assert!(!result.contains("id="));
    assert!(!result.contains("data-"));
    assert!(!result.contains("onclick"));
    assert!(!result.contains("srcset"));
    assert!(!result.contains("sizes="));
    assert!(!result.contains("width="));
    assert!(!result.contains("height="));

    // Safe attributes added
    assert!(result.contains(r#"rel="nofollow noopener""#));
    assert!(result.contains(r#"target="_blank""#));
    assert!(result.contains(r#"loading="lazy""#));
    assert!(result.contains(r#"fetchpriority="low""#));
    assert!(result.contains(r#"decoding="async""#));

    // Image src and alt preserved
    assert!(result.contains(r#"src="https://example.com/photo.jpg""#));
    assert!(result.contains(r#"alt="Travel photo""#));
}

#[test]
fn sanitizes_content_with_emoji_images() {
    let html = r#"<p>Great post <img src="https://example.com/emoji/thumbsup.png" class="wp-smiley" style="height: 1em;" width="20" height="20" alt="👍"> thanks!</p>"#;
    let result = sanitize(html);
    assert!(result.contains("Great post"));
    assert!(result.contains("thanks!"));
    assert!(result.contains(r#"src="https://example.com/emoji/thumbsup.png""#));
    assert!(!result.contains("class="));
    assert!(!result.contains("style="));
    assert!(!result.contains("width="));
    assert!(!result.contains("height="));
    assert!(result.contains(r#"loading="lazy""#));
}

#[test]
fn handles_multiple_links_with_varied_attrs() {
    let html = r#"<p><a href="/internal">Internal</a> and <a href="https://external.com" rel="sponsored" target="_top" class="link">External</a></p>"#;
    let result = sanitize(html);

    // Both links should have our safe attributes
    assert!(result.contains(r#"href="/internal""#));
    assert!(result.contains(r#"href="https://external.com""#));
    assert!(!result.contains("class="));
    assert!(!result.contains("sponsored"));
    assert!(!result.contains("_top"));
}

#[test]
fn removes_empty_containers_after_stripping_attrs() {
    let html = r#"<p>Content</p><div class="wp-block-group" id="group-1" style="margin: 10px"><span class="inner"></span></div>"#;
    let result = sanitize(html);
    assert!(result.contains("Content"));
    // The div's class/id/style are stripped, and the span is empty, so both should be removed
    assert!(!result.contains("<div"));
    assert!(!result.contains("<span"));
}
