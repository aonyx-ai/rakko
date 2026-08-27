/// The column in a line that a problem is at
mod column;
/// The line in a file that a problem is on
mod line;

pub use column::Column;
pub use line::Line;

use bon::bon;
use getset::Getters;

/// The position of a problem in a file
///
/// A position tells where in a file a problem is. It always has a line, and it
/// can have a column. The first line of a file is line 1, and the first column
/// of a line is column 1.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Position {
    /// The line that the problem is on
    #[getset(get = "pub")]
    line: Line,
    /// The column that the problem is at, or `None` when the position does not
    /// name a column
    #[getset(get = "pub")]
    column: Option<Column>,
}

#[bon]
impl Position {
    /// Creates a position from a line and an optional column
    // action[impl position.line]
    // action[impl position.column]
    #[builder]
    pub fn new(#[builder(into)] line: Line, #[builder(into)] column: Option<Column>) -> Self {
        Self { line, column }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // action[verify position.column]
    #[test]
    fn column_returns_given_column() {
        let position = Position::builder().line(1).column(5).build();

        assert_eq!(position.column(), &Some(Column::new(5)));
    }

    // action[verify position.column]
    #[test]
    fn column_without_value_returns_none() {
        let position = Position::builder().line(1).build();

        assert_eq!(position.column(), &None);
    }

    // action[verify position.line]
    #[test]
    fn line_returns_given_line() {
        let position = Position::builder().line(1).build();

        assert_eq!(position.line().get(), 1);
    }
}
