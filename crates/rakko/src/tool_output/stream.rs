//! The stream that a line of a tool came from

/// The stream that a line of a tool came from
///
/// A tool writes its results to one stream and its diagnostics to the other.
/// A reader that keeps the two apart sees the same split as a person who runs
/// the tool by hand.
///
/// # Examples
///
/// ```
/// use rakko::ToolStream;
///
/// let stream = ToolStream::StandardError;
///
/// assert_ne!(stream, ToolStream::StandardOutput);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[non_exhaustive]
pub enum ToolStream {
    /// The standard output stream of the tool
    StandardOutput,

    /// The standard error stream of the tool
    StandardError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_stream_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolStream>();
    }
}
