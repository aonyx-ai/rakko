//! One line that a tool wrote

use bon::bon;
use getset::{CopyGetters, Getters};

use super::stream::ToolStream;
use super::text::ToolText;

/// One line that a tool wrote, and the stream that it came from
///
/// # Examples
///
/// ```
/// use rakko::{ToolLine, ToolStream};
///
/// let line = ToolLine::builder()
///     .stream(ToolStream::StandardError)
///     .text("warning: unused import")
///     .build();
///
/// assert_eq!(line.text().get(), "warning: unused import");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct ToolLine {
    /// The stream that the line came from
    #[getset(get_copy = "pub")]
    stream: ToolStream,

    /// The text of the line
    #[getset(get = "pub")]
    text: ToolText,
}

#[bon]
impl ToolLine {
    /// Creates a line from its stream and its text
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{ToolLine, ToolStream};
    ///
    /// let line = ToolLine::builder()
    ///     .stream(ToolStream::StandardOutput)
    ///     .text("Compiling rakko v0.1.0")
    ///     .build();
    ///
    /// assert_eq!(line.stream(), ToolStream::StandardOutput);
    /// ```
    #[builder]
    pub fn new(stream: ToolStream, #[builder(into)] text: ToolText) -> Self {
        Self { stream, text }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_line_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolLine>();
    }
}
