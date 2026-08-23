use vurst_html_node::decode_html::decode_html_bytes;

#[test]
fn bom_overrides_the_http_charset() {
    let bytes = [
        0xef, 0xbb, 0xbf, b'R', 0xc3, 0xa9, b's', b'u', b'm', 0xc3, 0xa9,
    ];
    assert_eq!(
        decode_html_bytes(&bytes, Some("text/html; charset=windows-1252")),
        "Résumé"
    );
}

#[test]
fn http_charset_overrides_an_html_meta_charset() {
    let bytes = b"<meta charset=windows-1252><p>R\xc3\xa9sum\xc3\xa9";
    assert_eq!(
        decode_html_bytes(bytes, Some("text/html; charset=utf-8")),
        "<meta charset=windows-1252><p>Résumé"
    );
}

#[test]
fn prescan_detects_a_meta_charset_in_the_first_1024_bytes() {
    let mut bytes = b"<meta charset=windows-1252><p>\"".to_vec();
    bytes.push(0x93);
    assert!(decode_html_bytes(&bytes, None).contains('“'));
}

#[test]
fn prescan_ignores_meta_text_in_comments_and_raw_text_elements() {
    let mut bytes =
        b"<!-- <meta charset=iso-8859-7> --><script><meta charset=iso-8859-7></script><p>\""
            .to_vec();
    bytes.push(0x93);
    assert!(decode_html_bytes(&bytes, None).contains('“'));
}

#[test]
fn prescan_stops_at_1024_bytes() {
    let mut bytes = vec![b' '; 1024];
    bytes.extend_from_slice(b"<meta charset=iso-8859-7>");
    bytes.push(0x93);
    assert!(decode_html_bytes(&bytes, None).contains('“'));
}

#[test]
fn prescan_ignores_meta_lookalikes_in_attributes_and_raw_text_end_prefixes() {
    let mut bytes = b"<div data-note='<meta charset=iso-8859-7>'><script>\"</scriptx><meta charset=iso-8859-7></script><p>\"".to_vec();
    bytes.push(0x93);
    assert!(decode_html_bytes(&bytes, None).contains('“'));
}

#[test]
fn fallback_is_windows_1252_after_invalid_utf8() {
    assert_eq!(decode_html_bytes(&[0x93, b'H', 0x94], None), "“H”");
    assert_eq!(decode_html_bytes(&[0x81], None), "\u{0081}");
}

#[test]
fn malformed_declared_encodings_fall_back_to_valid_utf8() {
    assert_eq!(
        decode_html_bytes("Résumé".as_bytes(), Some("text/html; charset=bogus")),
        "Résumé"
    );
}
