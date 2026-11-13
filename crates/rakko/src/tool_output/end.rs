//! The end of a tool

use bon::bon;
use getset::{CopyGetters, Getters};

use super::command::ToolCommand;
use super::exit_code::ToolExitCode;

/// The end of a tool, and how it ended
///
/// A tool that ended by itself gives back an exit code. A tool that a signal of
/// the operating system stopped gives back none.
///
/// # Examples
///
/// ```
/// use rakko::{ToolEnd, ToolExitCode};
///
/// let end = ToolEnd::builder()
///     .command("taplo check")
///     .code(ToolExitCode::new(1))
///     .build();
///
/// assert_eq!(end.code(), Some(ToolExitCode::new(1)));
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, CopyGetters, Getters)]
pub struct ToolEnd {
    /// The command that started the tool
    #[getset(get = "pub")]
    command: ToolCommand,

    /// The exit code of the tool, or `None` when a signal stopped it
    #[getset(get_copy = "pub")]
    code: Option<ToolExitCode>,
}

#[bon]
impl ToolEnd {
    /// Creates the end of a tool from its command and its exit code
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::ToolEnd;
    ///
    /// let end = ToolEnd::builder().command("taplo check").build();
    ///
    /// assert_eq!(end.code(), None);
    /// ```
    #[builder]
    pub fn new(#[builder(into)] command: ToolCommand, code: Option<ToolExitCode>) -> Self {
        Self { command, code }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_end_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolEnd>();
    }
}
