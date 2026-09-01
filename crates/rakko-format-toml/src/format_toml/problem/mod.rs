/// What taplo reported about a file
mod detail;

use std::path::PathBuf;

use getset::Getters;

pub(super) use self::detail::ProblemDetail;

/// One problem that taplo reported about a file
///
/// The path stands as taplo wrote it, which is absolute, because taplo
/// starts in the project root. The caller makes it relative when it turns
/// the problem into a finding.
#[derive(Clone, Eq, PartialEq, Debug, Getters)]
pub(super) struct TaploProblem {
    /// The path of the file, as taplo wrote it
    #[getset(get = "pub(super)")]
    path: PathBuf,

    /// What taplo reported about the file
    #[getset(get = "pub(super)")]
    detail: ProblemDetail,
}

impl TaploProblem {
    /// Creates a problem from the path and the detail that taplo reported
    pub(super) fn new(path: PathBuf, detail: ProblemDetail) -> Self {
        Self { path, detail }
    }
}
