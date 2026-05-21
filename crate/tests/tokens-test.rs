use vurst::default_length_counter;

#[test]
fn test_empty_string() {
    assert_eq!(default_length_counter(""), 0);
    assert_eq!(default_length_counter("   "), 0);
}

#[test]
fn test_single_word() {
    assert_eq!(default_length_counter("hello"), 5);
}

#[test]
fn test_whitespace_normalization() {
    assert_eq!(default_length_counter("hello   world"), 11);
    assert_eq!(default_length_counter("hello world"), 11);
}

#[test]
fn test_two_words() {
    assert_eq!(default_length_counter("hello world"), 11);
}

#[test]
fn test_minimum_one_char() {
    assert_eq!(default_length_counter("a"), 1);
}
