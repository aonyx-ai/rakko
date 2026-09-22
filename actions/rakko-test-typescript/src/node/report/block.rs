use std::path::PathBuf;

use super::error::ReadReportError;
use crate::failure::Origin;

/// The text that closes the indented block below a line about a test
const CLOSE: &str = "...";

/// The text that separates the key of an entry from its value
const KEY_CLOSE: &str = ": ";

/// The character that ends the key of an entry that states no value
const KEY_END: char = ':';

/// The characters that open a value which the report writes over several lines
///
/// The report writes a value that holds a line break as a key, one of these
/// characters, and the value below it, indented further than the key.
const LONG: [char; 2] = ['|', '>'];

/// The character that quotes a value which the report writes verbatim
const LITERAL_QUOTE: char = '\'';

/// The character that quotes a value which the report writes with escapes
const ESCAPED_QUOTE: char = '"';

/// The character that escapes a character of a quoted value
const ESCAPE: char = '\\';

/// The character that separates the parts of a place
const PLACE_SEPARATOR: char = ':';

/// One indented block of a report
///
/// The report writes a block below every line that states a result, and the
/// block carries what the runner knows about that test: where the project
/// declares it, how long it took, and, for a test that failed, what went
/// wrong. A block is a set of entries, and a value spans several lines where
/// it holds a line break.
///
/// A block reads its own lines and keeps every entry, including the ones that
/// no caller asks for, because a block that refused an entry it does not know
/// would break on the next release of Node that adds one.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Block {
    /// The indentation of the lines that state the entries of the block
    indent: usize,

    /// Whether the report has closed the block
    closed: bool,

    /// The entries of the block, in the order that the report wrote them
    entries: Vec<(String, Scalar)>,

    /// The key of the entry whose value the block still collects
    collecting: Option<String>,
}

impl Block {
    /// Creates a block whose entries stand at the given indentation
    pub fn new(indent: usize) -> Self {
        Self {
            indent,
            closed: false,
            entries: Vec::new(),
            collecting: None,
        }
    }

    /// Returns whether the block still takes lines
    ///
    /// A block takes every line until the report closes it, so that a caller
    /// hands it the lines of a value that reads like anything else.
    pub fn open(&self) -> bool {
        !self.closed
    }

    /// Adds a line of the report to the block
    ///
    /// # Errors
    ///
    /// Returns [`UnreadableLine`][unreadable] for a line that states no entry
    /// of the block and is no part of a value.
    ///
    /// [unreadable]: ReadReportError::UnreadableLine
    pub fn read(&mut self, line: &str) -> Result<(), ReadReportError> {
        if self.collecting.is_some() {
            if line.trim().is_empty() || indent(line) > self.indent {
                self.extend(line);
                return Ok(());
            }

            self.collecting = None;
        }

        if line.trim().is_empty() {
            return Ok(());
        }

        if indent(line) != self.indent {
            return Err(unreadable(line));
        }

        let stated = line.trim();

        if stated == CLOSE {
            self.closed = true;
            return Ok(());
        }

        let (key, value) = entry(stated).ok_or_else(|| unreadable(line))?;

        if value.starts_with(LONG) {
            self.entries
                .push((key.to_owned(), Scalar::Lines(Vec::new())));
            self.collecting = Some(key.to_owned());

            return Ok(());
        }

        self.entries
            .push((key.to_owned(), Scalar::Text(unquote(value)?)));

        Ok(())
    }

    /// Returns the value of the named entry, when the block holds it
    pub fn value(&self, key: &str) -> Option<&Scalar> {
        self.entries
            .iter()
            .find(|(stated, _)| stated == key)
            .map(|(_, value)| value)
    }

    /// Adds a line to the value that the block collects
    fn extend(&mut self, line: &str) {
        let Some((_, Scalar::Lines(lines))) = self.entries.last_mut() else {
            return;
        };

        lines.push(line.trim().to_owned());
    }
}

/// One value of a block
///
/// The report writes a value on the line of its key, or below it over several
/// lines where the value holds a line break. A caller reads either of them the
/// same way, because a finding carries no formatting.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Scalar {
    /// A value that the report wrote over several lines
    Lines(Vec<String>),

    /// A value that the report wrote on the line of its key
    Text(String),
}

impl Scalar {
    /// Returns the value as one line
    ///
    /// The parts of the value join with a single space, and a run of spaces
    /// inside it becomes one, because the report indents a value that it wrote
    /// over several lines and a finding carries no formatting.
    pub fn flattened(&self) -> String {
        let joined = match self {
            Self::Lines(lines) => lines.join(" "),
            Self::Text(text) => text.clone(),
        };

        joined.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Returns the place that the value names
    ///
    /// A place reads as the path, the line, and the column, separated by the
    /// character that also separates the parts of a path on Windows, so the
    /// reading starts from the end.
    ///
    /// Returns `None` when the value names no place.
    pub fn place(&self) -> Option<Origin> {
        let stated = self.flattened();
        let mut parts = stated.rsplitn(3, PLACE_SEPARATOR);

        let column = parts.next()?.parse().ok()?;
        let line = parts.next()?.parse().ok()?;
        let path = parts.next().filter(|path| !path.is_empty())?;

        Some(
            Origin::builder()
                .path(PathBuf::from(path))
                .line(line)
                .column(column)
                .build(),
        )
    }
}

/// Returns the key and the value that a line of a block states
///
/// Returns `None` when the line states no entry at all.
fn entry(stated: &str) -> Option<(&str, &str)> {
    if let Some((key, value)) = stated.split_once(KEY_CLOSE) {
        return Some((key, value));
    }

    stated.strip_suffix(KEY_END).map(|key| (key, ""))
}

/// Returns the indentation of a line, in characters
fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Returns the error of a line that is no part of a block
fn unreadable(line: &str) -> ReadReportError {
    ReadReportError::UnreadableLine {
        line: line.trim().to_owned(),
    }
}

/// Returns the value without the quotes that the report wrote around it
///
/// The report quotes a value in one of two ways, and each of them escapes the
/// quote differently: a value written verbatim doubles it, and a value written
/// with escapes puts the character that escapes in front of it. A value that
/// needs no quote at all, such as a number, stands as it is.
///
/// # Errors
///
/// Returns [`UnreadableLine`][unreadable] for a value that opens a quote and
/// does not close it.
///
/// [unreadable]: ReadReportError::UnreadableLine
fn unquote(value: &str) -> Result<String, ReadReportError> {
    let value = value.trim();

    if let Some(quoted) = value.strip_prefix(LITERAL_QUOTE) {
        let quoted = quoted
            .strip_suffix(LITERAL_QUOTE)
            .ok_or_else(|| unreadable(value))?;

        return Ok(quoted.replace("''", "'"));
    }

    if let Some(quoted) = value.strip_prefix(ESCAPED_QUOTE) {
        let quoted = quoted
            .strip_suffix(ESCAPED_QUOTE)
            .ok_or_else(|| unreadable(value))?;

        return Ok(unescape(quoted));
    }

    Ok(value.to_owned())
}

/// Returns a quoted value with the escapes of the report undone
///
/// A value that the report writes with escapes carries a line break, a tab, a
/// quote, or the character that escapes, and each of them arrives as two
/// characters. An escape that this crate does not know keeps the character
/// behind it, which is what the standard says for every escape of one
/// character.
fn unescape(quoted: &str) -> String {
    let mut unescaped = String::with_capacity(quoted.len());
    let mut characters = quoted.chars();

    while let Some(character) = characters.next() {
        if character != ESCAPE {
            unescaped.push(character);
            continue;
        }

        match characters.next() {
            Some('n') => unescaped.push('\n'),
            Some('r') => unescaped.push('\r'),
            Some('t') => unescaped.push('\t'),
            Some(escaped) => unescaped.push(escaped),
            None => unescaped.push(ESCAPE),
        }
    }

    unescaped
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]
    #![allow(clippy::unwrap_used)]

    use rakko_test_utils::path;

    use super::*;

    /// Returns a block that read the given lines, which stand two characters
    /// in, as the report writes them below a result at the top level
    fn block(lines: &[&str]) -> Block {
        let mut block = Block::new(2);

        for line in lines {
            block.read(line).unwrap();
        }

        block
    }

    #[test]
    fn block_closes_on_the_line_that_closes_it() {
        let block = block(&["  type: 'test'", "  ..."]);

        assert!(!block.open(), "expected the block to be closed");
    }

    #[test]
    fn value_of_an_entry_the_block_does_not_hold_is_none() {
        let block = block(&["  type: 'test'"]);

        assert_eq!(block.value("location"), None);
    }

    #[test]
    fn value_of_a_verbatim_entry_drops_the_quotes() {
        let block = block(&["  failureType: 'testCodeFailure'"]);

        assert_eq!(
            block.value("failureType").unwrap().flattened(),
            "testCodeFailure"
        );
    }

    // The report writes a value with escapes where the value holds the quote
    // that it would otherwise be written with.
    #[test]
    fn value_of_an_escaped_entry_drops_the_quotes() {
        let block = block(&["  error: \"it's broken\""]);

        assert_eq!(block.value("error").unwrap().flattened(), "it's broken");
    }

    #[test]
    fn value_of_an_escaped_entry_undoes_the_escapes() {
        let block = block(&["  error: \"one\\ttwo\""]);

        assert_eq!(
            block.value("error").unwrap(),
            &Scalar::Text("one\ttwo".to_owned())
        );
    }

    #[test]
    fn value_of_an_unquoted_entry_stands_as_it_is() {
        let block = block(&["  exitCode: 1"]);

        assert_eq!(block.value("exitCode").unwrap().flattened(), "1");
    }

    #[test]
    fn value_of_an_entry_without_a_value_is_empty() {
        let block = block(&["  cause:"]);

        assert_eq!(block.value("cause").unwrap().flattened(), "");
    }

    #[test]
    fn value_that_the_report_wrote_over_several_lines_joins_into_one() {
        let block = block(&[
            "  error: |-",
            "    Expected values to be strictly equal:",
            "",
            "    2 !== 3",
            "  code: 'ERR_ASSERTION'",
        ]);

        assert_eq!(
            block.value("error").unwrap().flattened(),
            "Expected values to be strictly equal: 2 !== 3"
        );
    }

    // A value written over several lines ends where the next entry starts, and
    // that entry belongs to the block like any other.
    #[test]
    fn entry_after_a_value_of_several_lines_reads_as_an_entry() {
        let block = block(&[
            "  error: |-",
            "    broke",
            "  failureType: 'testCodeFailure'",
        ]);

        assert_eq!(
            block.value("failureType").unwrap().flattened(),
            "testCodeFailure"
        );
    }

    // A line of a value can read exactly like the line that closes a block,
    // and it belongs to the value because it stands further in.
    #[test]
    fn value_of_several_lines_keeps_a_line_that_reads_like_the_close() {
        let block = block(&["  stack: |-", "    ...", "  ..."]);

        assert_eq!(block.value("stack").unwrap().flattened(), "...");
    }

    #[test]
    fn place_of_a_value_that_names_one_reads_it() {
        let block = block(&["  location: '/home/otter/project/src/add.test.ts:5:1'"]);

        assert_eq!(
            block.value("location").unwrap().place(),
            Some(
                Origin::builder()
                    .path(path("/home/otter/project/src/add.test.ts"))
                    .line(5)
                    .column(1)
                    .build()
            )
        );
    }

    #[test]
    fn place_of_a_value_that_names_none_is_none() {
        let block = block(&["  location: '~'"]);

        assert_eq!(block.value("location").unwrap().place(), None);
    }

    #[test]
    fn read_of_a_line_that_states_no_entry_stops() {
        let mut block = Block::new(2);

        let error = block.read("  not an entry").unwrap_err();

        assert!(
            matches!(&error, ReadReportError::UnreadableLine { line } if line == "not an entry"),
            "expected the line that the reading could not place, got {error:?}"
        );
    }

    #[test]
    fn read_of_a_line_that_stands_at_another_indentation_stops() {
        let mut block = Block::new(2);

        let error = block.read("      type: 'test'").unwrap_err();

        assert!(
            matches!(error, ReadReportError::UnreadableLine { .. }),
            "expected the line that the reading could not place, got {error:?}"
        );
    }

    #[test]
    fn read_of_a_value_that_opens_a_quote_without_closing_it_stops() {
        let mut block = Block::new(2);

        let error = block.read("  error: 'broke").unwrap_err();

        assert!(
            matches!(error, ReadReportError::UnreadableLine { .. }),
            "expected the line that the reading could not place, got {error:?}"
        );
    }
}
