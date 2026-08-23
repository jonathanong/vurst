use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8, WINDOWS_1252, X_USER_DEFINED};

const HTML_ENCODING_PRESCAN_BYTES: usize = 1024;

/// Decode HTML bytes using the HTML charset precedence order: BOM, HTTP
/// `Content-Type`, early `<meta>` declaration, UTF-8, then Windows-1252.
pub fn decode_html_bytes(bytes: &[u8], content_type: Option<&str>) -> String {
    let (encoding, bom_len) = bom_encoding(bytes)
        .map(|encoding| (encoding, bom_len(bytes)))
        .or_else(|| {
            content_type
                .and_then(header_encoding)
                .map(|encoding| (encoding, 0))
        })
        .or_else(|| prescan_encoding(bytes).map(|encoding| (encoding, 0)))
        .unwrap_or((UTF_8, 0));

    if let Some(decoded) = decode_without_errors(encoding, &bytes[bom_len..]) {
        return decoded;
    }
    if !std::ptr::eq(encoding, UTF_8) {
        if let Some(decoded) = decode_without_errors(UTF_8, &bytes[bom_len..]) {
            return decoded;
        }
    }
    WINDOWS_1252.decode(&bytes[bom_len..]).0.into_owned()
}

fn bom_encoding(bytes: &[u8]) -> Option<&'static Encoding> {
    match bytes {
        [0xef, 0xbb, 0xbf, ..] => Some(UTF_8),
        [0xfe, 0xff, ..] => Some(UTF_16BE),
        [0xff, 0xfe, ..] => Some(UTF_16LE),
        _ => None,
    }
}

fn bom_len(bytes: &[u8]) -> usize {
    match bytes {
        [0xef, 0xbb, 0xbf, ..] => 3,
        [0xfe, 0xff, ..] | [0xff, 0xfe, ..] => 2,
        _ => 0,
    }
}

fn decode_without_errors(encoding: &'static Encoding, bytes: &[u8]) -> Option<String> {
    let (decoded, _, had_errors) = encoding.decode(bytes);
    (!had_errors).then(|| decoded.into_owned())
}

fn header_encoding(content_type: &str) -> Option<&'static Encoding> {
    parse_http_parameters(content_type)
        .into_iter()
        .find_map(|(name, value)| {
            (name.eq_ignore_ascii_case("charset")).then(|| encoding_for_label(&value))?
        })
}

fn prescan_encoding(bytes: &[u8]) -> Option<&'static Encoding> {
    let mut prescan = bytes[..bytes.len().min(HTML_ENCODING_PRESCAN_BYTES)].to_vec();
    mask_comments_and_raw_text(&mut prescan);
    let mut index = 0;
    while index < prescan.len() {
        if prescan[index] != b'<' {
            index += 1;
            continue;
        }
        let after_name = index + 5;
        if !starts_with_ascii_case_insensitive(&prescan, index, b"<meta")
            || !is_tag_boundary(prescan.get(after_name).copied())
        {
            index = if is_html_tag_prefix(prescan.get(index + 1).copied()) {
                find_tag_end(&prescan, index + 1).map_or(prescan.len(), |end| end + 1)
            } else {
                index + 1
            };
            continue;
        }
        let Some(end) = find_tag_end(&prescan, after_name) else {
            break;
        };
        if let Some(encoding) = meta_encoding(&prescan[after_name..end]) {
            return Some(encoding);
        }
        index = end + 1;
    }
    None
}

fn meta_encoding(tag: &[u8]) -> Option<&'static Encoding> {
    let attributes = parse_attributes(tag);
    if let Some(charset) = attributes
        .iter()
        .find(|(name, _)| name == "charset")
        .and_then(|(_, value)| value.as_deref())
    {
        if let Some(encoding) = meta_encoding_for_label(charset) {
            return Some(encoding);
        }
    }
    let is_content_type = attributes.iter().any(|(name, value)| {
        name == "http-equiv"
            && value
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case("content-type"))
    });
    is_content_type
        .then(|| {
            attributes
                .iter()
                .find(|(name, _)| name == "content")
                .and_then(|(_, value)| value.as_deref())
                .and_then(pragma_encoding)
        })
        .flatten()
}

fn pragma_encoding(content: &str) -> Option<&'static Encoding> {
    let lower = content.as_bytes();
    let mut from = 0;
    while let Some(start) = find_ascii_case_insensitive(lower, b"charset", from) {
        let mut index = skip_ascii_whitespace(lower, start + b"charset".len());
        if lower.get(index) == Some(&b'=') {
            index = skip_ascii_whitespace(lower, index + 1);
            if let Some(encoding) =
                parse_charset_value(lower, index).and_then(meta_encoding_for_label)
            {
                return Some(encoding);
            }
        }
        from = start + b"charset".len();
    }
    None
}

fn encoding_for_label(label: &str) -> Option<&'static Encoding> {
    let label = label.trim();
    (!label.is_empty())
        .then(|| Encoding::for_label(label.as_bytes()))
        .flatten()
}

fn meta_encoding_for_label(label: &str) -> Option<&'static Encoding> {
    let encoding = encoding_for_label(label)?;
    if std::ptr::eq(encoding, UTF_16LE) || std::ptr::eq(encoding, UTF_16BE) {
        return Some(UTF_8);
    }
    if std::ptr::eq(encoding, X_USER_DEFINED) {
        return Some(WINDOWS_1252);
    }
    Some(encoding)
}

fn parse_http_parameters(input: &str) -> Vec<(String, String)> {
    let bytes = input.as_bytes();
    let mut parameters = Vec::new();
    let Some(first_parameter) = find_unquoted_byte(bytes, b';', 0) else {
        return parameters;
    };
    let mut index = first_parameter + 1;
    while index < bytes.len() {
        index = skip_ascii_whitespace(bytes, index);
        let name_start = index;
        while bytes
            .get(index)
            .is_some_and(|byte| *byte != b'=' && *byte != b';')
        {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            if bytes.get(index) == Some(&b';') {
                index += 1;
                continue;
            }
            break;
        }
        if name_start == index {
            index += 1;
            continue;
        }
        let name = String::from_utf8_lossy(&bytes[name_start..index])
            .trim()
            .to_ascii_lowercase();
        index = skip_ascii_whitespace(bytes, index + 1);
        let Some((value, end)) = parse_http_value(bytes, index) else {
            let Some(next) = find_unquoted_byte(bytes, b';', index) else {
                break;
            };
            index = next + 1;
            continue;
        };
        if name == "charset" && !parameters.iter().any(|(existing, _)| existing == &name) {
            parameters.push((name, value));
        }
        index = end.saturating_add(1);
    }
    parameters
}

fn parse_http_value(bytes: &[u8], index: usize) -> Option<(String, usize)> {
    if matches!(bytes.get(index), Some(b'\'' | b'"')) {
        let quote = bytes[index];
        let mut end = index + 1;
        let mut value = Vec::new();
        while let Some(byte) = bytes.get(end).copied() {
            if byte == quote {
                break;
            }
            if byte == b'\\' {
                end += 1;
                value.push(*bytes.get(end)?);
            } else {
                value.push(byte);
            }
            end += 1;
        }
        if bytes.get(end) != Some(&quote) {
            return None;
        }
        let trailing = skip_ascii_whitespace(bytes, end + 1);
        if bytes.get(trailing).is_some_and(|byte| *byte != b';') {
            return None;
        }
        return Some((String::from_utf8_lossy(&value).into_owned(), trailing));
    }
    let end = bytes[index..]
        .iter()
        .position(|byte| *byte == b';')
        .map_or(bytes.len(), |offset| index + offset);
    Some((
        String::from_utf8_lossy(&bytes[index..end])
            .trim()
            .to_owned(),
        end,
    ))
}

fn parse_charset_value(bytes: &[u8], index: usize) -> Option<&str> {
    let end = if matches!(bytes.get(index), Some(b'\'' | b'"')) {
        let quote = bytes[index];
        let end = bytes[index + 1..].iter().position(|byte| *byte == quote)? + index + 1;
        end
    } else {
        bytes[index..]
            .iter()
            .position(|byte| byte.is_ascii_whitespace() || *byte == b';')
            .map_or(bytes.len(), |offset| index + offset)
    };
    std::str::from_utf8(
        &bytes[index + usize::from(matches!(bytes.get(index), Some(b'\'' | b'"')))..end],
    )
    .ok()
}

fn skip_ascii_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

fn find_unquoted_byte(bytes: &[u8], needle: u8, mut index: usize) -> Option<usize> {
    let mut quote = None;
    while let Some(byte) = bytes.get(index).copied() {
        if let Some(open) = quote {
            if byte == open {
                quote = None;
            }
        } else if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
        } else if byte == needle {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn parse_attributes(tag: &[u8]) -> Vec<(String, Option<String>)> {
    let mut attributes = Vec::new();
    let mut index = 0;
    while index < tag.len() {
        while tag
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'/')
        {
            index += 1;
        }
        let name_start = index;
        while tag
            .get(index)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'=' && *byte != b'/')
        {
            index += 1;
        }
        if name_start == index {
            break;
        }
        let name = String::from_utf8_lossy(&tag[name_start..index]).to_ascii_lowercase();
        while tag.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        let value = if tag.get(index) == Some(&b'=') {
            index += 1;
            while tag.get(index).is_some_and(u8::is_ascii_whitespace) {
                index += 1;
            }
            let quote = tag
                .get(index)
                .copied()
                .filter(|byte| *byte == b'\'' || *byte == b'"');
            if quote.is_some() {
                index += 1;
            }
            let value_start = index;
            while tag.get(index).is_some_and(|byte| {
                quote.map_or(!byte.is_ascii_whitespace(), |quote| *byte != quote)
            }) {
                index += 1;
            }
            let value = String::from_utf8_lossy(&tag[value_start..index]).into_owned();
            if quote.is_some() && tag.get(index) == quote.as_ref() {
                index += 1;
            }
            Some(value)
        } else {
            None
        };
        if !attributes.iter().any(|(existing, _)| existing == &name) {
            attributes.push((name, value));
        }
    }
    attributes
}

fn mask_comments_and_raw_text(bytes: &mut [u8]) {
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        if starts_with_ascii_case_insensitive(bytes, index, b"<!--") {
            let end = find_ascii_case_insensitive(bytes, b"-->", index + 4)
                .map_or(bytes.len(), |end| end + 3);
            bytes[index..end].fill(b' ');
            index = end;
            continue;
        }
        let tag = [b"script".as_slice(), b"style".as_slice()]
            .into_iter()
            .find(|tag| {
                starts_with_ascii_case_insensitive(bytes, index + 1, tag)
                    && is_tag_boundary(bytes.get(index + 1 + tag.len()).copied())
            });
        let Some(tag) = tag else {
            index = if is_html_tag_prefix(bytes.get(index + 1).copied()) {
                find_tag_end(bytes, index + 1).map_or(bytes.len(), |end| end + 1)
            } else {
                index + 1
            };
            continue;
        };
        let Some(open_end) = find_tag_end(bytes, index + 1 + tag.len()) else {
            break;
        };
        let content_start = open_end + 1;
        let closing_prefix = [b"</".as_slice(), tag].concat();
        let close = find_raw_text_end(bytes, &closing_prefix, content_start);
        let content_end = close.unwrap_or(bytes.len());
        bytes[content_start..content_end].fill(b' ');
        index = close
            .and_then(|start| find_tag_end(bytes, start + closing_prefix.len()).map(|end| end + 1))
            .unwrap_or(bytes.len());
    }
}

fn find_raw_text_end(bytes: &[u8], prefix: &[u8], mut from: usize) -> Option<usize> {
    while let Some(start) = find_ascii_case_insensitive(bytes, prefix, from) {
        if is_tag_boundary(bytes.get(start + prefix.len()).copied()) {
            return Some(start);
        }
        from = start + prefix.len();
    }
    None
}

fn find_tag_end(bytes: &[u8], from: usize) -> Option<usize> {
    let mut quote = None;
    for (index, byte) in bytes.iter().copied().enumerate().skip(from) {
        if let Some(open) = quote {
            if byte == open {
                quote = None;
            }
        } else if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
        } else if byte == b'>' {
            return Some(index);
        }
    }
    None
}

fn find_ascii_case_insensitive(bytes: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    (from..=bytes.len().saturating_sub(needle.len()))
        .find(|&index| starts_with_ascii_case_insensitive(bytes, index, needle))
}

fn starts_with_ascii_case_insensitive(bytes: &[u8], start: usize, needle: &[u8]) -> bool {
    bytes
        .get(start..start + needle.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(needle))
}

fn is_tag_boundary(byte: Option<u8>) -> bool {
    byte.is_none_or(|byte| byte == b'>' || byte == b'/' || byte.is_ascii_whitespace())
}

fn is_html_tag_prefix(byte: Option<u8>) -> bool {
    byte.is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'/' || byte == b'!')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(encoding: Option<&'static Encoding>) -> Option<&'static str> {
        encoding.map(Encoding::name)
    }

    #[test]
    fn parses_http_charset_parameters_without_splitting_quoted_values() {
        assert_eq!(
            name(header_encoding("TEXT/HTML; Charset=\"ISO-8859-1\"")),
            Some("windows-1252")
        );
        assert_eq!(
            name(header_encoding("text/html; charset='windows-1252'")),
            Some("windows-1252")
        );
        assert_eq!(
            name(header_encoding("text/html; title=\"a;b\"; charset=utf-8")),
            Some("UTF-8")
        );
        assert_eq!(
            name(header_encoding("text/html; charset=windows-1252; title=x")),
            Some("windows-1252")
        );
        assert_eq!(
            header_encoding("text/html; charset=\"windows-1252\" junk"),
            None
        );
        assert_eq!(
            name(header_encoding(
                "text/html; charset=utf-8; charset=windows-1252"
            )),
            Some("UTF-8")
        );
        assert_eq!(
            name(header_encoding(
                "text/html; malformed; charset=windows-1252"
            )),
            Some("windows-1252")
        );
    }

    #[test]
    fn prescans_the_downstream_meta_charset_fixture_matrix() {
        let cases = [
            ("<meta charset=windows-1252>", Some("windows-1252")),
            ("<meta\ncharset=windows-1252>", Some("windows-1252")),
            ("<meta http-equiv=content-type content=text/html;charset=windows-1252>", Some("windows-1252")),
            ("<meta http-equiv='Content-Type' content='text/html; charset=windows-1252'>", Some("windows-1252")),
            ("<meta name=description content='charset=windows-1252'>", None),
            ("<meta http-equiv=content-type content='text/html; foocharset=windows-1252'>", Some("windows-1252")),
            ("<meta charset=bogus><meta charset=windows-1252>", Some("windows-1252")),
            ("<meta charset=' '><meta charset=windows-1252>", Some("windows-1252")),
            ("<meta charset=' ' http-equiv=content-type content='text/html; charset=windows-1252'>", Some("windows-1252")),
            ("<meta charset=utf-8 charset=windows-1252>", Some("UTF-8")),
            ("<meta foo charset=windows-1252>", Some("windows-1252")),
            ("<meta http-equiv=content-type content='text/html; charset=\"windows-1252 foo\"'>", None),
        ];
        for (html, expected) in cases {
            assert_eq!(name(prescan_encoding(html.as_bytes())), expected, "{html}");
        }
    }

    #[test]
    fn prescan_applies_html_label_remapping() {
        assert_eq!(
            name(prescan_encoding(b"<meta charset=utf-16le>")),
            Some("UTF-8")
        );
        assert_eq!(
            name(prescan_encoding(b"<meta charset=UTF-16BE>")),
            Some("UTF-8")
        );
        assert_eq!(
            name(prescan_encoding(b"<meta charset=x-user-defined>")),
            Some("windows-1252")
        );
    }

    #[test]
    fn prescan_bounds_and_masks_the_downstream_raw_text_fixture_matrix() {
        let ignored = [
            "<style><meta charset=windows-1252></style>",
            "<style media=screen><meta charset=windows-1252></style>",
            "<style><meta charset=windows-1252>",
            "<script><meta charset=windows-1252></script>",
            "<script>const x='</scriptx><meta charset=windows-1252>';</script>",
        ];
        for html in ignored {
            assert_eq!(prescan_encoding(html.as_bytes()), None, "{html}");
        }
        assert_eq!(
            name(prescan_encoding(
                b"<!-- <script> --><meta charset=windows-1252>"
            )),
            Some("windows-1252")
        );
        assert_eq!(
            name(prescan_encoding(
                b"<script><!-- opener</script><meta charset=windows-1252>"
            )),
            Some("windows-1252")
        );
        assert_eq!(
            name(prescan_encoding(
                b"<meta data-note='<script>'><meta charset=windows-1252>"
            )),
            Some("windows-1252")
        );
        let mut late = vec![b' '; HTML_ENCODING_PRESCAN_BYTES];
        late.extend_from_slice(b"<meta charset=windows-1252>");
        assert_eq!(prescan_encoding(&late), None);
    }

    #[test]
    fn stray_angle_brackets_do_not_hide_real_tags_or_raw_text() {
        assert_eq!(
            name(prescan_encoding(b"<<meta charset=windows-1252>")),
            Some("windows-1252")
        );
        assert_eq!(
            prescan_encoding(b"<<script><meta charset=windows-1252></script>"),
            None
        );
    }

    #[test]
    fn decodes_bom_and_fallback_edge_cases() {
        assert_eq!(
            decode_html_bytes(
                &[0xfe, 0xff, 0, b'A'],
                Some("text/html; charset=windows-1252")
            ),
            "A"
        );
        assert_eq!(decode_html_bytes(&[0xff, 0xfe, 0x93], None), "“");
        assert_eq!(decode_html_bytes(&[0xef, 0xbb, 0xbf, 0x93], None), "“");
        for byte in [0x81, 0x8d, 0x8f, 0x90, 0x9d] {
            assert_eq!(
                decode_html_bytes(&[byte], None).chars().next().unwrap() as u32,
                u32::from(byte)
            );
        }
    }

    #[test]
    fn covers_malformed_charset_syntax_without_losing_later_valid_declarations() {
        assert_eq!(
            decode_html_bytes(b"abc", Some("text/html; charset=utf-16le")),
            "abc"
        );
        assert_eq!(header_encoding("text/html"), None);
        assert_eq!(
            name(header_encoding("text/html; =bad; charset=windows-1252")),
            Some("windows-1252")
        );
        assert_eq!(
            name(header_encoding(
                "text/html; charset=\"bad\" junk; charset=windows-1252"
            )),
            Some("windows-1252")
        );
        assert_eq!(
            name(pragma_encoding("charset nope; charset=windows-1252")),
            Some("windows-1252")
        );
        assert_eq!(
            name(pragma_encoding("charset=\"windows-1252\"junk")),
            Some("windows-1252")
        );
        assert_eq!(
            name(pragma_encoding(
                "charset=unsupported-charset; charset=windows-1252"
            )),
            Some("windows-1252")
        );
    }

    #[test]
    fn covers_http_parser_terminal_and_escaped_quoted_value_branches() {
        assert_eq!(bom_len(b"no bom"), 0);
        assert_eq!(header_encoding("text/html; malformed"), None);
        assert_eq!(
            parse_http_value(b"\"windows\\-1252\";", 0),
            Some(("windows-1252".to_owned(), 15))
        );
        assert_eq!(parse_http_value(b"\"unterminated", 0), None);
    }

    #[test]
    fn handles_incomplete_tags_and_attributes_without_hiding_following_metadata() {
        assert_eq!(prescan_encoding(b"<meta"), None);
        assert_eq!(prescan_encoding(b"<style"), None);
        assert_eq!(find_tag_end(b"<meta charset=windows-1252", 1), None);
        assert_eq!(
            parse_attributes(b" =x"),
            Vec::<(String, Option<String>)>::new()
        );
        assert_eq!(
            name(prescan_encoding(b"<meta charset =  windows-1252>")),
            Some("windows-1252")
        );
    }
}
