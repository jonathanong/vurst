pub fn decode_numeric_char_ref(input: &str) -> Option<(char, usize)> {
    let digits = input.strip_prefix("&#")?;
    let (digits, radix, prefix_len) = digits
        .strip_prefix(['x', 'X'])
        .map_or((digits, 10, 2), |hex_digits| (hex_digits, 16, 3));

    let digit_len = digits
        .bytes()
        .take_while(|b| {
            if radix == 16 {
                b.is_ascii_hexdigit()
            } else {
                b.is_ascii_digit()
            }
        })
        .count();
    if digit_len == 0 {
        return None;
    }

    let codepoint = u32::from_str_radix(&digits[..digit_len], radix).ok()?;
    let ch = char::from_u32(codepoint)?;
    let semicolon_len = usize::from(digits[digit_len..].starts_with(';'));
    Some((ch, prefix_len + digit_len + semicolon_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_numeric_char_ref_covers_decimal_hex_and_invalid_refs() {
        assert_eq!(decode_numeric_char_ref("&#58alert"), Some((':', 4)));
        assert_eq!(decode_numeric_char_ref("&#58;alert"), Some((':', 5)));
        assert_eq!(decode_numeric_char_ref("&#x3cscript"), Some(('<', 5)));
        assert_eq!(decode_numeric_char_ref("plain"), None);
        assert_eq!(decode_numeric_char_ref("&#x;"), None);
        assert_eq!(decode_numeric_char_ref("&#99999999;"), None);
    }
}
