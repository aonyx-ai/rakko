//! One event of a tool that runs

use super::command::ToolCommand;
use super::end::ToolEnd;
use super::line::ToolLine;

/// One event of a tool that runs
///
/// A tool sends one [`Started`][started] event, then one [`Line`][line] event
/// for each line that it writes, and then one [`Ended`][ended] event. A tool
/// that fails to start sends no event, because its error reaches the action.
///
/// The lines of the two streams of a tool arrive in the order that the
/// machinery reads them. That order can differ from the order in which the
/// tool wrote to the two streams.
///
/// [ended]: ToolEvent::Ended
/// [line]: ToolEvent::Line
/// [started]: ToolEvent::Started
///
/// # Examples
///
/// ```
/// use rakko::{ToolCommand, ToolEvent};
///
/// let event = ToolEvent::Started(ToolCommand::new("cargo build"));
///
/// assert!(matches!(event, ToolEvent::Started(_)));
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[non_exhaustive]
pub enum ToolEvent {
    /// The tool started, with the command that started it
    Started(ToolCommand),

    /// The tool wrote one line
    Line(ToolLine),

    /// The tool ended
    Ended(ToolEnd),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_event_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolEvent>();
    }
}
