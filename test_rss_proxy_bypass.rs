use vurst_html_node::{sanitize_rss_html_sync, SanitizeRssHtmlOptions};

fn main() {
    let opts = SanitizeRssHtmlOptions {
        proxy_images: true,
        image_proxy_url_prefix: "/proxy/".to_string(),
        image_proxy_signing_keys: vec![],
    };

    let html = r#"<img src="//attacker.com/image.png">"#;
    let result = sanitize_rss_html_sync(html, &opts);
    println!("Protocol relative //: {}", result.html);

    let html2 = r#"<img src="\\attacker.com/image.png">"#;
    let result2 = sanitize_rss_html_sync(html2, &opts);
    println!("Backslash \\\\: {}", result2.html);

    let html3 = r#"<img src="http://attacker.com/image.png">"#;
    let result3 = sanitize_rss_html_sync(html3, &opts);
    println!("Normal http: {}", result3.html);
}
