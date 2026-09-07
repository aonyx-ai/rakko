use std::path::PathBuf;

use getset::Getters;

/// One problem that rustfmt reported about a file
///
/// The path stands as rustfmt wrote it, which is absolute, because rustfmt
/// names the files that it read. A caller that reports the problem asks the
/// root for the path relative to the project, which is the name that a
/// reader, a machine, and a code host all recognize.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct RustfmtProblem {
    /// The path of the file, as rustfmt wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// What rustfmt reported about the file
    #[getset(get = "pub")]
    detail: RustfmtProblemDetail,
}

impl RustfmtProblem {
    /// Creates a problem from the path and the detail that rustfmt reported
    pub fn new(path: PathBuf, detail: RustfmtProblemDetail) -> Self {
        Self { path, detail }
    }
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
