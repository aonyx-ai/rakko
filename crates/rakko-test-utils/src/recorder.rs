//! A tool output that keeps every event that it receives

use std::sync::{Arc, Mutex, PoisonError};

use rakko::{ToolEvent, ToolOutput};

/// A tool output that keeps every event that it receives, for a test to read
///
/// A test gives [`output`][output] to the context of a run. After the run, it
/// reads [`events`][events] to see what the tools of the action sent.
///
/// [events]: ToolRecorder::events
/// [output]: ToolRecorder::output
///
/// # Examples
///
/// ```
/// use rakko::{Context, ToolCommand, ToolEvent};
/// use rakko_test_utils::ToolRecorder;
///
/// let recorder = ToolRecorder::new();
/// let context = Context::builder()
///     .root("project")
///     .tool_output(recorder.output().clone())
///     .build();
///
/// context
///     .tool_output()
///     .send(ToolEvent::Started(ToolCommand::new("taplo check")));
///
/// assert_eq!(recorder.events().len(), 1);
/// ```
#[derive(Clone, Debug)]
pub struct ToolRecorder {
    /// The events that the output received, in the order of their arrival
    events: Arc<Mutex<Vec<ToolEvent>>>,

    /// The tool output that adds each event to the list
    output: ToolOutput,
}

impl ToolRecorder {
    /// Creates a recorder that holds no event yet
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_test_utils::ToolRecorder;
    ///
    /// let recorder = ToolRecorder::new();
    ///
    /// assert!(recorder.events().is_empty());
    /// ```
    pub fn new() -> Self {
        let events = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&events);

        let output = ToolOutput::new(move |event| {
            received
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(event);
        });

        Self { events, output }
    }

    /// Returns a copy of the events that the output received
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{ToolCommand, ToolEvent};
    /// use rakko_test_utils::ToolRecorder;
    ///
    /// let recorder = ToolRecorder::new();
    ///
    /// recorder
    ///     .output()
    ///     .send(ToolEvent::Started(ToolCommand::new("taplo check")));
    ///
    /// assert_eq!(
    ///     recorder.events(),
    ///     [ToolEvent::Started(ToolCommand::new("taplo check"))]
    /// );
    /// ```
    pub fn events(&self) -> Vec<ToolEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Returns the tool output that keeps each event
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{ToolCommand, ToolEvent};
    /// use rakko_test_utils::ToolRecorder;
    ///
    /// let recorder = ToolRecorder::new();
    /// let output = recorder.output().clone();
    ///
    /// output.send(ToolEvent::Started(ToolCommand::new("taplo check")));
    ///
    /// assert_eq!(recorder.events().len(), 1);
    /// ```
    pub fn output(&self) -> &ToolOutput {
        &self.output
    }
}

impl Default for ToolRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::ToolCommand;

    use super::*;

    #[test]
    fn events_keeps_the_order_of_arrival() {
        let recorder = ToolRecorder::new();
        let first = ToolEvent::Started(ToolCommand::new("first"));
        let second = ToolEvent::Started(ToolCommand::new("second"));

        recorder.output().send(first.clone());
        recorder.output().send(second.clone());

        assert_eq!(recorder.events(), [first, second]);
    }

    #[test]
    fn tool_recorder_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolRecorder>();
    }
}
