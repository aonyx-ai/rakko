//! One problem that tracey reported about the specifications of a project
//!
//! Tracey validates the specifications of a project against the code that
//! answers them, and reports every link that no longer holds. This module
//! holds one such report, in the shape that an action needs to turn it into a
//! finding.

use bon::bon;
use getset::Getters;

/// A problem that tracey found in the specifications of a project
///
/// Tracey places every problem in a file, at a line and a column, and writes
/// a message about it. The message is the one that tracey wrote, so a reader
/// of a finding reads the answer of the tool instead of one that Rakko wrote
/// about it.
///
/// The path is the one that tracey reported. Tracey starts in the root of the
/// project and names a file below it, so the path is already the one that a
/// finding needs.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct TraceyProblem {
    /// The path of the file that the problem is in
    #[getset(get = "pub")]
    path: String,

    /// The line of the file that the problem is on
    #[getset(get = "pub")]
    line: u32,

    /// The column of the line that the problem is at
    #[getset(get = "pub")]
    column: u32,

    /// What tracey wrote about the problem
    #[getset(get = "pub")]
    message: String,
}

#[bon]
impl TraceyProblem {
    /// Creates a problem that tracey placed in a file
    #[builder]
    pub fn new(
        #[builder(into)] path: String,
        line: u32,
        column: u32,
        #[builder(into)] message: String,
    ) -> Self {
        Self {
            path,
            line,
            column,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn builder_keeps_what_tracey_reported() {
        let problem = TraceyProblem::builder()
            .path("crates/rakko/src/lib.rs")
            .line(12)
            .column(4)
            .message("Unknown rule reference")
            .build();

        assert_eq!(
            (
                problem.path().as_str(),
                *problem.line(),
                *problem.column(),
                problem.message().as_str()
            ),
            ("crates/rakko/src/lib.rs", 12, 4, "Unknown rule reference")
        );
    }
}
