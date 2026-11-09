//! The source that a diagnostic points at

use std::path::PathBuf;

use bon::Builder;
use getset::Getters;
use rakko::Span;

/// The source that a diagnostic points at
///
/// The compiler names the file and the range of the file that a diagnostic
/// is about. The path stands as cargo wrote it, which is relative to the
/// root that cargo checked, because the compiler runs there. A caller that
/// reports the diagnostic asks the root for the path relative to the
/// project.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, Getters)]
pub(crate) struct DiagnosticSpan {
    /// The file that the diagnostic is about, as cargo wrote it
    #[getset(get = "pub(crate)")]
    path: PathBuf,

    /// The range of the file that the diagnostic covers
    #[getset(get = "pub(crate)")]
    range: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_span_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<DiagnosticSpan>();
    }
}
