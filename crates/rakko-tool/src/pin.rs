//! The version that a mise file of a crate pins for a tool
//!
//! A crate that wraps a tool keeps the version of that tool in a mise file of
//! its own. The crate embeds the file when it compiles. This module reads the
//! version from the text of that file, in a constant context. So a file
//! without the pin fails the build of the crate, and not a run of an action.

/// The header of the table that holds the pins of a mise file
const TOOLS: &[u8] = b"[tools]";

/// Returns the version that a mise file pins for a tool
///
/// The function reads the `[tools]` table of the file and returns the text
/// of the entry for the tool, without its quotation marks. The key of the
/// entry can be bare, as in `taplo = "0.10.0"`, or quoted, as in
/// `"npm:markdownlint-cli" = "0.50.0"`.
///
/// The function reads the plain form of an entry, which the mise manager of
/// Renovate writes. That form is one entry on one line, with a version in
/// double quotation marks. It does not read an entry whose value is a table or an
/// array, such as `taplo = { version = "0.10.0" }`.
///
/// Call the function to give a value to a constant, with the file that
/// [`include_str!`] embeds. The compiler then evaluates it, and a file that
/// pins no version of the tool fails the build.
///
/// # Panics
///
/// Panics when the `[tools]` table of the file holds no plain entry for the
/// tool. In a constant, the panic is an error of the build.
///
/// # Examples
///
/// ```
/// use rakko_tool::pinned_version;
///
/// const PINS: &str = "[tools]\ntaplo = \"0.10.0\"\n";
/// const VERSION: &str = pinned_version(PINS, "taplo");
///
/// assert_eq!(VERSION, "0.10.0");
/// ```
///
/// A file without the pin fails the build:
///
/// ```compile_fail
/// use rakko_tool::pinned_version;
///
/// const VERSION: &str = pinned_version("[tools]\n", "taplo");
/// ```
pub const fn pinned_version<'a>(file: &'a str, tool: &str) -> &'a str {
    let text = file.as_bytes();
    let mut start = 0;
    let mut in_tools = false;

    while start < text.len() {
        let mut end = start;
        while end < text.len() && text[end] != b'\n' {
            end += 1;
        }

        let (first, last) = trim(text, start, end);

        if first < last && text[first] == b'[' {
            in_tools = header_is_tools(text, first, last);
        } else if in_tools && let Some((from, to)) = entry(text, first, last, tool.as_bytes()) {
            let (_, rest) = file.split_at(from);
            let (version, _) = rest.split_at(to - from);

            return version;
        }

        start = end + 1;
    }

    panic!("the mise file pins no plain version of the tool in its `[tools]` table");
}

/// Returns the bounds of a line without the spaces at its ends
///
/// The end of a line on Windows carries a carriage return before the line
/// feed. The readers of a header and of an entry stop at a closing mark, so
/// they never reach it.
const fn trim(text: &[u8], mut first: usize, mut last: usize) -> (usize, usize) {
    while first < last && is_space(text[first]) {
        first += 1;
    }

    while last > first && is_space(text[last - 1]) {
        last -= 1;
    }

    (first, last)
}

/// Returns whether a byte is a space that TOML ignores
const fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// Returns whether the header of a table on a line is `[tools]`
///
/// A comment can follow the header, so only the text up to the first
/// closing bracket counts.
const fn header_is_tools(text: &[u8], first: usize, last: usize) -> bool {
    let mut close = first;
    while close < last && text[close] != b']' {
        close += 1;
    }

    close < last && equals(text, first, close + 1, TOOLS)
}

/// Returns the bounds of the version of an entry, when the entry is the one
/// of the tool
///
/// A line that is not a plain entry of the tool answers `None`. Such lines
/// are:
///
/// - an empty line,
/// - a comment, which starts with `#`, so that its key is never the name of
///   a tool,
/// - an entry of another tool, and
/// - an entry whose value is not a string in double quotation marks.
const fn entry(text: &[u8], first: usize, last: usize, tool: &[u8]) -> Option<(usize, usize)> {
    if first >= last {
        return None;
    }

    let mut cursor = first;

    if text[cursor] == b'"' {
        let key = cursor + 1;
        let Some(close) = find(text, key, last, b'"') else {
            return None;
        };
        if !equals(text, key, close, tool) {
            return None;
        }
        cursor = close + 1;
    } else {
        while cursor < last && text[cursor] != b'=' && !is_space(text[cursor]) {
            cursor += 1;
        }
        if !equals(text, first, cursor, tool) {
            return None;
        }
    }

    cursor = skip_spaces(text, cursor, last);
    if cursor >= last || text[cursor] != b'=' {
        return None;
    }

    cursor = skip_spaces(text, cursor + 1, last);
    if cursor >= last || text[cursor] != b'"' {
        return None;
    }

    let version = cursor + 1;
    let Some(close) = find(text, version, last, b'"') else {
        return None;
    };

    Some((version, close))
}

/// Returns the first position at or after a position that holds no space
const fn skip_spaces(text: &[u8], mut cursor: usize, last: usize) -> usize {
    while cursor < last && is_space(text[cursor]) {
        cursor += 1;
    }

    cursor
}

/// Returns the first position at or after a position that holds a byte
const fn find(text: &[u8], mut cursor: usize, last: usize, byte: u8) -> Option<usize> {
    while cursor < last {
        if text[cursor] == byte {
            return Some(cursor);
        }
        cursor += 1;
    }

    None
}

/// Returns whether a part of the text holds exactly the expected bytes
const fn equals(text: &[u8], first: usize, last: usize, expected: &[u8]) -> bool {
    if last - first != expected.len() {
        return false;
    }

    let mut index = 0;
    while index < expected.len() {
        if text[first + index] != expected[index] {
            return false;
        }
        index += 1;
    }

    true
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn pinned_version_ignores_a_comment_after_the_header() {
        let file = "[tools] # the pins\ntaplo = \"0.10.0\"\n";

        assert_eq!(pinned_version(file, "taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_ignores_a_quoted_key_of_another_tool() {
        let file = "[tools]\n\"npm:prettier\" = \"3.9.10\"\n\"taplo\" = \"0.10.0\"\n";

        assert_eq!(pinned_version(file, "taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_ignores_an_entry_of_another_table() {
        let file = "[env]\ntaplo = \"0.9.0\"\n\n[tools]\ntaplo = \"0.10.0\"\n";

        assert_eq!(pinned_version(file, "taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_ignores_an_entry_of_another_tool() {
        let file = "[tools]\ntaplo-cli = \"1.0.0\"\ntaplo = \"0.10.0\"\n";

        assert_eq!(pinned_version(file, "taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_reads_a_file_with_the_line_ends_of_windows() {
        let file = "[tools]\r\ntaplo = \"0.10.0\"\r\n";

        assert_eq!(pinned_version(file, "taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_reads_a_quoted_key() {
        let file = "[tools]\n\"npm:markdownlint-cli\" = \"0.50.0\"\n";

        assert_eq!(pinned_version(file, "npm:markdownlint-cli"), "0.50.0");
    }

    #[test]
    fn pinned_version_reads_an_entry_after_a_comment() {
        let file = "[tools]\n# taplo = \"0.9.0\"\ntaplo = \"0.10.0\" # the pin\n";

        assert_eq!(pinned_version(file, "taplo"), "0.10.0");
    }

    #[test]
    fn pinned_version_reads_an_entry_without_spaces() {
        let file = "[tools]\ntaplo=\"0.10.0\"\n";

        assert_eq!(pinned_version(file, "taplo"), "0.10.0");
    }

    #[test]
    #[should_panic(expected = "pins no plain version")]
    fn pinned_version_with_a_pin_in_another_table_panics() {
        pinned_version("[env]\ntaplo = \"0.10.0\"\n", "taplo");
    }

    #[test]
    #[should_panic(expected = "pins no plain version")]
    fn pinned_version_with_a_table_as_the_value_panics() {
        pinned_version("[tools]\ntaplo = { version = \"0.10.0\" }\n", "taplo");
    }

    #[test]
    #[should_panic(expected = "pins no plain version")]
    fn pinned_version_with_a_version_without_its_closing_mark_panics() {
        pinned_version("[tools]\ntaplo = \"0.10.0\n", "taplo");
    }

    #[test]
    #[should_panic(expected = "pins no plain version")]
    fn pinned_version_with_an_entry_without_an_equals_sign_panics() {
        pinned_version("[tools]\ntaplo : \"0.10.0\"\n", "taplo");
    }

    #[test]
    #[should_panic(expected = "pins no plain version")]
    fn pinned_version_without_the_tool_before_a_blank_last_line_panics() {
        pinned_version("[tools]\njq = \"1.8.2\"\n  ", "taplo");
    }

    #[test]
    #[should_panic(expected = "pins no plain version")]
    fn pinned_version_without_the_tool_panics() {
        pinned_version("[tools]\njq = \"1.8.2\"\n", "taplo");
    }
}
