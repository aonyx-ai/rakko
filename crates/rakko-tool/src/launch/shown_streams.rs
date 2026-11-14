//! The streams of a tool that reach the tool output of a run

use kawauso_process::run::Stream;

/// The streams of a tool that reach the tool output of a run
///
/// An action often asks its tool for a report in a machine format, such as
/// JSON, because the action parses it. A reader cannot read such a report,
/// and the outcome of the action already shows what it holds. Such a tool
/// usually writes its output for people to the other stream, such as the
/// progress of a build. So the call that starts a tool says which streams the
/// tool output receives. The capture of the run holds both streams in every
/// case, so the parsing of the action does not change.
///
/// The start and the end of the tool reach the tool output in every case.
///
/// # Examples
///
/// ```
/// use rakko_tool::ShownStreams;
///
/// // Cargo writes its JSON messages to the standard output stream, and its
/// // progress to the standard error stream.
/// let shown = ShownStreams::StandardError;
///
/// assert_ne!(shown, ShownStreams::Both);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ShownStreams {
    /// Both streams, for a tool that writes text for people to both
    Both,

    /// Only the standard error stream, for a tool that writes its report to
    /// the standard output stream
    StandardError,

    /// Only the standard output stream, for a tool that writes its report to
    /// the standard error stream
    StandardOutput,
}

impl ShownStreams {
    /// Returns whether the lines of a stream reach the tool output
    pub(crate) fn shows(self, stream: Stream) -> bool {
        match (self, stream) {
            (Self::Both, _)
            | (Self::StandardError, Stream::StandardError)
            | (Self::StandardOutput, Stream::StandardOutput) => true,
            (Self::StandardError, Stream::StandardOutput)
            | (Self::StandardOutput, Stream::StandardError) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn shown_streams_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ShownStreams>();
    }

    #[test]
    fn shows_with_both_shows_the_standard_error_stream() {
        let shows = ShownStreams::Both.shows(Stream::StandardError);

        assert!(shows);
    }

    #[test]
    fn shows_with_both_shows_the_standard_output_stream() {
        let shows = ShownStreams::Both.shows(Stream::StandardOutput);

        assert!(shows);
    }

    #[test]
    fn shows_with_the_standard_error_stream_hides_the_standard_output_stream() {
        let shows = ShownStreams::StandardError.shows(Stream::StandardOutput);

        assert!(!shows);
    }

    #[test]
    fn shows_with_the_standard_error_stream_shows_it() {
        let shows = ShownStreams::StandardError.shows(Stream::StandardError);

        assert!(shows);
    }

    #[test]
    fn shows_with_the_standard_output_stream_hides_the_standard_error_stream() {
        let shows = ShownStreams::StandardOutput.shows(Stream::StandardError);

        assert!(!shows);
    }

    #[test]
    fn shows_with_the_standard_output_stream_shows_it() {
        let shows = ShownStreams::StandardOutput.shows(Stream::StandardOutput);

        assert!(shows);
    }
}
