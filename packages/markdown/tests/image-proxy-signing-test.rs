use data_encoding::BASE64URL_NOPAD;
use vurst_markdown_node::markdown_to_html::{render_markdown_to_html_with_options, MarkdownRenderOptions};

const TEST_KEY: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
const OTHER_KEY: &str = "cafebabecafebabecafebabecafebabecafebabecafebabecafebabecafebabe";

fn opts_with_keys(keys: Vec<String>) -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        proxy_images: true,
        image_proxy_signing_keys: keys,
        ..MarkdownRenderOptions::default()
    }
}

fn opts_no_keys() -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        proxy_images: true,
        image_proxy_signing_keys: vec![],
        ..MarkdownRenderOptions::default()
    }
}

/// Extract the `src` attribute value from the rendered HTML.
fn extract_src(html: &str) -> Option<String> {
    let start = html.find("src=\"")?;
    let rest = &html[start + 5..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// HTML serializers encode `&` as `&amp;` in attribute values.
/// Decode for easier assertion matching.
fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
}

#[test]
fn with_keys_produces_sig_param() {
    let result = render_markdown_to_html_with_options(
        "![img](https://example.com/photo.jpg)",
        &opts_with_keys(vec![TEST_KEY.to_string()]),
    );
    let src = decode_html_entities(&extract_src(&result).expect("should have src attribute"));
    assert!(
        src.contains("?sig="),
        "expected ?sig= in image proxy URL: {src}"
    );
}

#[test]
fn without_keys_no_sig_param() {
    let result = render_markdown_to_html_with_options(
        "![img](https://example.com/photo.jpg)",
        &opts_no_keys(),
    );
    let src = decode_html_entities(&extract_src(&result).expect("should have src attribute"));
    assert!(!src.contains("sig="), "unexpected sig= without keys: {src}");
    assert!(!src.contains('?'), "should have no query string: {src}");
}

#[test]
fn sig_is_64_hex_chars() {
    let result = render_markdown_to_html_with_options(
        "![img](https://example.com/photo.jpg)",
        &opts_with_keys(vec![TEST_KEY.to_string()]),
    );
    let src = decode_html_entities(&extract_src(&result).expect("should have src attribute"));
    let sig = src
        .split("?sig=")
        .nth(1)
        .expect("expected ?sig= in image proxy URL");
    assert_eq!(
        sig.len(),
        64,
        "HMAC-SHA256 hex must be 64 chars, got: {sig}"
    );
    assert!(
        sig.chars().all(|c| c.is_ascii_hexdigit()),
        "sig must be hex: {sig}"
    );
}

#[test]
fn sig_is_deterministic() {
    let md = "![img](https://example.com/photo.jpg)";
    let opts = opts_with_keys(vec![TEST_KEY.to_string()]);
    let r1 = render_markdown_to_html_with_options(md, &opts);
    let r2 = render_markdown_to_html_with_options(md, &opts);
    assert_eq!(r1, r2, "same input+key must produce same image proxy URL");
}

#[test]
fn different_urls_produce_different_sigs() {
    let opts = opts_with_keys(vec![TEST_KEY.to_string()]);
    let r1 = render_markdown_to_html_with_options("![img](https://example.com/photo.jpg)", &opts);
    let r2 = render_markdown_to_html_with_options("![img](https://example.com/other.jpg)", &opts);
    let src1 = decode_html_entities(&extract_src(&r1).expect("should have src attribute"));
    let src2 = decode_html_entities(&extract_src(&r2).expect("should have src attribute"));
    let sig1 = src1.split("?sig=").nth(1).expect("expected ?sig=");
    let sig2 = src2.split("?sig=").nth(1).expect("expected ?sig=");
    assert_ne!(
        sig1, sig2,
        "different URLs must produce different signatures"
    );
}

#[test]
fn different_keys_produce_different_sigs() {
    let r1 = render_markdown_to_html_with_options(
        "![img](https://example.com/photo.jpg)",
        &opts_with_keys(vec![TEST_KEY.to_string()]),
    );
    let r2 = render_markdown_to_html_with_options(
        "![img](https://example.com/photo.jpg)",
        &opts_with_keys(vec![OTHER_KEY.to_string()]),
    );
    let src1 = decode_html_entities(&extract_src(&r1).expect("should have src attribute"));
    let src2 = decode_html_entities(&extract_src(&r2).expect("should have src attribute"));
    let sig1 = src1.split("?sig=").nth(1).expect("expected ?sig=");
    let sig2 = src2.split("?sig=").nth(1).expect("expected ?sig=");
    assert_ne!(
        sig1, sig2,
        "different keys must produce different signatures"
    );
}

#[test]
fn sig_matches_expected_base64url_path() {
    // Verify the sig is computed against the /proxy/{base64url} path.
    let url = "https://example.com/photo.jpg";
    let encoded = BASE64URL_NOPAD.encode(url.as_bytes());
    let path = format!("/proxy/{encoded}");

    let result = render_markdown_to_html_with_options(
        &format!("![img]({url})"),
        &opts_with_keys(vec![TEST_KEY.to_string()]),
    );
    let src = decode_html_entities(&extract_src(&result).expect("should have src attribute"));

    // The src path (before ?) should be the image proxy path we expect.
    let src_path = src
        .split('?')
        .next()
        .expect("expected ? in image proxy URL");
    assert_eq!(src_path, path, "image proxy path mismatch");
}

#[test]
fn sig_matches_expected_hmac_sha256() {
    let url = "https://example.com/photo.jpg";
    let expected_src = concat!(
        "/proxy/aHR0cHM6Ly9leGFtcGxlLmNvbS9waG90by5qcGc",
        "?sig=1126c3dba789ae72e3674a819b59c8219c4f12453d58c89ac2b7ea0f55cfc789",
    );

    let result = render_markdown_to_html_with_options(
        &format!("![img]({url})"),
        &opts_with_keys(vec![TEST_KEY.to_string()]),
    );
    let src = decode_html_entities(&extract_src(&result).expect("should have src attribute"));

    assert_eq!(src, expected_src, "signed image proxy URL mismatch");
}
