use vurst::markdown_to_html::{render_markdown_to_html_with_options, MarkdownRenderOptions};

fn admin_opts() -> MarkdownRenderOptions {
    MarkdownRenderOptions {
        allow_html: true,
        nofollow_links: false,
        proxy_images: false,
        ..MarkdownRenderOptions::default()
    }
}

#[test]
fn allows_safe_formatting_tags() {
    let result = render_markdown_to_html_with_options(
        "<p><strong>bold</strong> and <em>italic</em></p>",
        &admin_opts(),
    );
    assert!(result.contains("<p><strong>bold</strong> and <em>italic</em></p>"));
}

#[test]
fn strips_script_tags() {
    let result = render_markdown_to_html_with_options(
        "<p>safe</p><script>alert('xss')</script>",
        &admin_opts(),
    );
    assert!(result.contains("<p>safe</p>"));
    assert!(!result.contains("<script>"));
    assert!(!result.contains("alert"));
}

#[test]
fn strips_iframe_tags() {
    let result = render_markdown_to_html_with_options(
        "<p>text</p><iframe src=\"https://evil.com\"></iframe>",
        &admin_opts(),
    );
    assert!(result.contains("<p>text</p>"));
    assert!(!result.contains("<iframe"));
}

#[test]
fn strips_style_tags() {
    let result = render_markdown_to_html_with_options(
        "<p>text</p><style>body{display:none}</style>",
        &admin_opts(),
    );
    assert!(result.contains("<p>text</p>"));
    assert!(!result.contains("<style>"));
    assert!(!result.contains("display:none"));
}

#[test]
fn strips_event_handlers() {
    let result =
        render_markdown_to_html_with_options("<p onclick=\"alert('xss')\">text</p>", &admin_opts());
    assert!(result.contains("<p>text</p>"));
    assert!(!result.contains("onclick"));
}

#[test]
fn strips_style_attribute() {
    let result =
        render_markdown_to_html_with_options("<p style=\"color:red\">text</p>", &admin_opts());
    assert!(result.contains("<p>text</p>"));
    assert!(!result.contains("style="));
}

#[test]
fn strips_javascript_url_in_href() {
    let result = render_markdown_to_html_with_options(
        "<a href=\"javascript:alert('xss')\">click</a>",
        &admin_opts(),
    );
    assert!(result.contains("click"));
    assert!(!result.contains("javascript:"));
}

#[test]
fn strips_data_url_in_img() {
    let result = render_markdown_to_html_with_options(
        "<img src=\"data:image/png;base64,abc\">",
        &admin_opts(),
    );
    assert!(!result.contains("data:"));
}

#[test]
fn allows_class_and_id_attrs() {
    let result = render_markdown_to_html_with_options(
        "<div class=\"container\" id=\"main\">text</div>",
        &admin_opts(),
    );
    assert!(result.contains("class=\"container\""));
    assert!(result.contains("id=\"main\""));
}

#[test]
fn allows_table_structure() {
    let result = render_markdown_to_html_with_options(
        "<table><thead><tr><th>H</th></tr></thead><tbody><tr><td>D</td></tr></tbody></table>",
        &admin_opts(),
    );
    assert!(result.contains("<table>"));
    assert!(result.contains("<th>H</th>"));
    assert!(result.contains("<td>D</td>"));
}

#[test]
fn allows_details_summary() {
    let result = render_markdown_to_html_with_options(
        "<details><summary>Click</summary>Content</details>",
        &admin_opts(),
    );
    assert!(result.contains("<details>"));
    assert!(result.contains("<summary>Click</summary>"));
}

#[test]
fn strips_form_elements() {
    let result = render_markdown_to_html_with_options(
        "<form action=\"/\"><input type=\"text\"><button>Submit</button></form>",
        &admin_opts(),
    );
    assert!(!result.contains("<form"));
    assert!(!result.contains("<input"));
    assert!(!result.contains("<button"));
}

#[test]
fn unwraps_unknown_tags() {
    let result =
        render_markdown_to_html_with_options("<custom>preserved text</custom>", &admin_opts());
    assert!(result.contains("preserved text"));
    assert!(!result.contains("<custom>"));
}

#[test]
fn allows_anchor_fragment_links() {
    let result =
        render_markdown_to_html_with_options("<a href=\"#section\">jump</a>", &admin_opts());
    assert!(result.contains("href=\"#section\""));
}

#[test]
fn allows_relative_path_links() {
    let result =
        render_markdown_to_html_with_options("<a href=\"../other\">relative</a>", &admin_opts());
    assert!(result.contains("href=\"../other\""));

    let result2 =
        render_markdown_to_html_with_options("<a href=\"./page\">current</a>", &admin_opts());
    assert!(result2.contains("href=\"./page\""));
}

#[test]
fn allows_query_string_links() {
    let result =
        render_markdown_to_html_with_options("<a href=\"?sort=new\">sort</a>", &admin_opts());
    assert!(result.contains("href=\"?sort=new\""));
}

#[test]
fn allows_tel_links() {
    let result =
        render_markdown_to_html_with_options("<a href=\"tel:+123456789\">call</a>", &admin_opts());
    assert!(result.contains("href=\"tel:+123456789\""));
}

#[test]
fn rejects_protocol_relative_urls_in_admin_html() {
    let result =
        render_markdown_to_html_with_options("<a href=\"//attacker.com\">bad</a>", &admin_opts());
    assert!(!result.contains("//attacker.com"));
}
