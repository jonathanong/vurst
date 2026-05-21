use super::*;

#[test]
fn sanitizer_private_helpers_cover_empty_and_attr_paths() {
    assert_eq!(sanitize_admin_html(""), "");
    assert!(is_allowed_attr("p", "class"));
    assert!(!is_allowed_attr("p", "onclick"));
    assert!(!is_allowed_attr("p", "style"));
    assert!(!is_allowed_attr("p", "unknown"));
    assert_eq!(
        sanitize_admin_html(
            "<script>bad()</script><img src=\"javascript:bad\" alt=\"x\"><a href=\"javascript:bad\">x</a><br><p>ok</p><!-- comment -->"
        ),
        "<img alt=\"x\"><a>x</a><br><p>ok</p>"
    );
    assert_eq!(sanitize_admin_html("<unknown>x</unknown>"), "x");
    let fragment = Html::parse_fragment("<p>ok</p>");
    assert_eq!(render_node(fragment.tree.root()), "<p>ok</p>");
}
