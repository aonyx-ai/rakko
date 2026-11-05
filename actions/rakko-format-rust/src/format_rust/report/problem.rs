//! One problem that rustfmt reported about a file

use std::path::PathBuf;

use bon::Builder;
use getset::Getters;

/// One problem that rustfmt reported about a file
///
/// The path is absolute, because rustfmt names the files that it read, and
/// it carries the plain spelling, without the verbatim prefix of Windows,
/// unless the plain spelling cannot describe it. A caller that reports the
/// problem asks the root for the path relative to the project, which is the
/// name that a reader, a machine, and a code host all recognize.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, Getters)]
pub struct RustfmtProblem {
    /// The absolute path of the file
    #[getset(get = "pub")]
    path: PathBuf,

    /// What rustfmt reported about the file
    #[getset(get = "pub")]
    detail: RustfmtProblemDetail,
}

/// What rustfmt reported about a file
///
/// Rustfmt names a problem as precisely as it can. A file that it would
/// rewrite gets a path and nothing else, because the difference is the whole
/// file. A file that it cannot parse gets the position where the parse
/// stopped and the message of the parser.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum RustfmtProblemDetail {
    /// Rustfmt cannot parse the file
    Invalid {
        /// The line where the parse stopped
        line: u32,

        /// The column where the parse stopped
        column: u32,

        /// The message of the parser
        message: String,
    },

    /// Rustfmt would rewrite the file
    Unformatted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustfmt_problem_detail_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<RustfmtProblemDetail>();
    }

    #[test]
    fn rustfmt_problem_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<RustfmtProblem>();
    }
}
