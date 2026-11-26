//! How a reply repeats a value the client sent.
//!
//! A message may repeat a client's own value when that helps the client find its mistake, within
//! the limits of rule 5 of the client error text ADR
//! (`docs/adrs/20261125175726_tell_clients_what_they_can_act_on_and_keep_failure_detail_on_the_server.md`):
//! at most [`MAX_ECHOED_CHARACTERS`] characters of the value as it was sent, then `...`, with
//! control characters, quotes and backslashes escaped after the cut so that the text stays on one line.
//!
//! The REST API server keeps its own copy of this function: `http-protocol` has no dependency
//! shared with it, and the rule is ten lines.

/// The most characters of a client's value that a reply repeats. It is longer than any well-formed
/// value of the tracker (a hexadecimal info-hash has 40), so a value that is cut is already wrong.
pub const MAX_ECHOED_CHARACTERS: usize = 64;

/// `value` as a reply may repeat it: cut to [`MAX_ECHOED_CHARACTERS`] characters, escaped after the cut.
#[must_use]
pub fn echo(value: &str) -> String {
    let mut characters = value.chars();
    let kept: String = characters.by_ref().take(MAX_ECHOED_CHARACTERS).collect();
    let was_cut = characters.next().is_some();

    let mut echoed: String = kept.escape_debug().collect();
    if was_cut {
        echoed.push_str("...");
    }
    echoed
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::{MAX_ECHOED_CHARACTERS, echo};

    #[rstest]
    #[case::a_plain_value("abc", "abc")]
    #[case::an_empty_value("", "")]
    #[case::a_line_break("a\nb", "a\\nb")]
    #[case::a_tab_and_a_carriage_return("a\tb\rc", "a\\tb\\rc")]
    #[case::a_quote_and_a_backslash("it's \"x\" \\", "it\\'s \\\"x\\\" \\\\")]
    #[case::printable_unicode_as_sent("naïve ✓", "naïve ✓")]
    fn it_should_escape_what_would_break_a_line_and_keep_printable_text(#[case] value: &str, #[case] expected: &str) {
        assert_eq!(echo(value), expected);
    }

    #[test]
    fn it_should_repeat_a_value_of_the_maximum_length_whole() {
        let value = "a".repeat(MAX_ECHOED_CHARACTERS);

        assert_eq!(echo(&value), value);
    }

    #[test]
    fn it_should_cut_a_longer_value_at_the_maximum_and_mark_the_cut() {
        let value = "a".repeat(MAX_ECHOED_CHARACTERS + 1);

        assert_eq!(echo(&value), format!("{}...", "a".repeat(MAX_ECHOED_CHARACTERS)));
    }

    #[test]
    fn it_should_count_the_characters_before_escaping() {
        // 64 line breaks are 64 characters before escaping and 128 after.
        let value = "\n".repeat(MAX_ECHOED_CHARACTERS);

        assert_eq!(echo(&value), "\\n".repeat(MAX_ECHOED_CHARACTERS));
    }

    #[test]
    fn it_should_cut_on_a_character_boundary() {
        let value = "é".repeat(MAX_ECHOED_CHARACTERS + 1);

        assert_eq!(echo(&value), format!("{}...", "é".repeat(MAX_ECHOED_CHARACTERS)));
    }
}
