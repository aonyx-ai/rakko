use bon::bon;
use getset::Getters;

use super::Position;

/// The range of a file that a problem covers
///
/// A span has the position where the range starts and the position where it
/// ends. The range can cross lines, because a tool can report a problem that
/// starts on one line and ends on a later one.
///
/// A span does not make sure that the end is at or after the start. A range
/// that runs backwards comes from an action that read its tool wrong, and no
/// reader of a span can repair it.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Span {
    /// The position where the range starts
    #[getset(get = "pub")]
    start: Position,
    /// The position where the range ends
    #[getset(get = "pub")]
    end: Position,
}

#[bon]
impl Span {
    /// Creates a span from the position where a range starts and the position
    /// where it ends
    // action[impl span.start]
    // action[impl span.end]
    #[builder]
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // action[verify span.end]
    #[test]
    fn end_returns_given_end() {
        let end = Position::builder().line(9).column(2).build();

        let span = Span::builder()
            .start(Position::builder().line(7).build())
            .end(end)
            .build();

        assert_eq!(span.end(), &end);
    }

    // action[verify span.start]
    #[test]
    fn start_returns_given_start() {
        let start = Position::builder().line(7).column(4).build();

        let span = Span::builder()
            .start(start)
            .end(Position::builder().line(9).build())
            .build();

        assert_eq!(span.start(), &start);
    }
}
