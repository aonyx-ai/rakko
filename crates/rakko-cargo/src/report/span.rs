use std::path::PathBuf;

use getset::Getters;
use rakko_action::Span;

/// The source that a diagnostic points at
///
/// The compiler names the file and the range of the file that a diagnostic
/// is about. The path stands as cargo wrote it, which is relative to the
/// root that cargo checked, because the compiler runs there. A caller that
/// reports the diagnostic asks the root for the path relative to the
/// project.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct DiagnosticSpan {
    /// The file that the diagnostic is about, as cargo wrote it
    #[getset(get = "pub")]
    path: PathBuf,

    /// The range of the file that the diagnostic covers
    #[getset(get = "pub")]
    range: Span,
}

impl DiagnosticSpan {
    /// Creates a span from the path and the range that the compiler named
    pub fn new(path: PathBuf, range: Span) -> Self {
        Self { path, range }
    }
}
