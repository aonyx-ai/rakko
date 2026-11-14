//! Types for the output that the tools of an action write while they run

mod command;
mod end;
mod event;
mod exit_code;
mod line;
mod stream;
mod text;

use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub use self::command::ToolCommand;
pub use self::end::ToolEnd;
pub use self::event::ToolEvent;
pub use self::exit_code::ToolExitCode;
pub use self::line::ToolLine;
pub use self::stream::ToolStream;
pub use self::text::ToolText;

/// The function that receives the events of a [`ToolOutput`]
type Receiver = dyn Fn(ToolEvent) + Send + Sync;

/// The place where the tools of an action send what they write while they run
///
/// An action turns the output of its tools into an outcome. That outcome
/// arrives when the run is over. A reader who investigates a failure, or who
/// watches a long run in CI, also needs the output itself. So the context of a
/// run carries a tool output. The machinery that starts a tool sends each
/// [`ToolEvent`] to it. These are the start of the tool, each line that the
/// tool writes, and the end of the tool.
///
/// The events never enter the outcome. The action still parses the capture of
/// each tool, and the tool output only receives a copy of the lines. The
/// harness decides what a reader sees. A tool output tells an action nothing
/// about the reader. It has no method that says whether it discards, and two
/// tool outputs always compare equal.
///
/// The author of an action sends no text of its own to a tool output. The
/// type does not enforce this, because the machinery that starts a tool sends
/// its events through the same method.
///
/// A tool output either discards every event or sends it to a receiver. The
/// [`Default`] discards, so a context that a caller builds without a tool
/// output stays silent. A receiver is a function that runs on the thread of
/// the sender. It must return quickly, because the tool waits while it runs.
///
/// # Examples
///
/// ```
/// use std::sync::{Arc, Mutex};
///
/// use rakko::{ToolCommand, ToolEvent, ToolOutput};
///
/// let events = Arc::new(Mutex::new(Vec::new()));
/// let received = Arc::clone(&events);
/// let output = ToolOutput::new(move |event| received.lock().unwrap().push(event));
///
/// output.send(ToolEvent::Started(ToolCommand::new("taplo check")));
///
/// assert_eq!(events.lock().unwrap().len(), 1);
/// ```
#[derive(Clone, Default)]
pub struct ToolOutput {
    /// The function that receives each event, or `None` to discard every event
    receiver: Option<Arc<Receiver>>,
}

impl ToolOutput {
    /// Returns a tool output that discards every event
    ///
    /// This is the tool output of a context that a caller builds without one.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{ToolCommand, ToolEvent, ToolOutput};
    ///
    /// let output = ToolOutput::discard();
    ///
    /// output.send(ToolEvent::Started(ToolCommand::new("taplo check")));
    /// ```
    pub fn discard() -> Self {
        Self { receiver: None }
    }

    /// Returns a tool output that sends every event to a receiver
    ///
    /// The receiver runs once for each event, in the order of the events, on
    /// the thread that sends the event. The tool that wrote the event waits
    /// until the receiver returns. So a receiver that needs time, such as one
    /// that writes to a terminal, hands the event to another task.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::ToolOutput;
    ///
    /// let output = ToolOutput::new(|event| println!("{event:?}"));
    /// ```
    pub fn new(receiver: impl Fn(ToolEvent) + Send + Sync + 'static) -> Self {
        Self {
            receiver: Some(Arc::new(receiver)),
        }
    }

    /// Sends one event to the receiver
    ///
    /// A tool output that discards drops the event.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{ToolCommand, ToolEvent, ToolOutput};
    ///
    /// let output = ToolOutput::discard();
    ///
    /// output.send(ToolEvent::Started(ToolCommand::new("taplo check")));
    /// ```
    pub fn send(&self, event: ToolEvent) {
        if let Some(receiver) = &self.receiver {
            receiver(event);
        }
    }

    /// Returns whether the tool output discards every event
    ///
    /// Only the tests of this crate read it, because an action must not learn
    /// whether anybody reads the events.
    #[cfg(test)]
    pub(crate) fn discards(&self) -> bool {
        self.receiver.is_none()
    }
}

/// Writes the name of the type and nothing about its receiver
impl Debug for ToolOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ToolOutput")
    }
}

/// Compares every tool output as equal, so that a comparison of two contexts
/// depends on their other data alone
impl PartialEq for ToolOutput {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for ToolOutput {}

/// Orders every tool output as equal, for the reason that [`PartialEq`] gives
impl Ord for ToolOutput {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl PartialOrd for ToolOutput {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Adds nothing to the hash, because every tool output is equal
impl Hash for ToolOutput {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::sync::Mutex;

    use super::*;

    /// Returns a tool output and the list that it fills
    fn recording() -> (ToolOutput, Arc<Mutex<Vec<ToolEvent>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&events);

        let output = ToolOutput::new(move |event| {
            received
                .lock()
                .expect("no test panics while it holds the lock")
                .push(event);
        });

        (output, events)
    }

    fn started() -> ToolEvent {
        ToolEvent::Started(ToolCommand::new("taplo check"))
    }

    #[test]
    fn debug_hides_the_receiver() {
        let (output, _events) = recording();

        assert_eq!(
            format!("{output:?}"),
            format!("{:?}", ToolOutput::discard())
        );
    }

    #[test]
    fn default_discards() {
        let output = ToolOutput::default();

        assert!(output.discards());
    }

    #[test]
    fn new_does_not_discard() {
        let (output, _events) = recording();

        assert!(!output.discards());
    }

    #[test]
    fn new_equals_a_tool_output_that_discards() {
        let (output, _events) = recording();

        assert_eq!(output, ToolOutput::discard());
    }

    #[test]
    fn new_equals_its_clone() {
        let (output, _events) = recording();

        assert_eq!(output.clone(), output);
    }

    #[test]
    fn send_reaches_the_receiver() {
        let (output, events) = recording();

        output.send(started());

        assert_eq!(
            *events
                .lock()
                .expect("no test panics while it holds the lock"),
            [started()]
        );
    }

    #[test]
    fn send_through_a_clone_reaches_the_receiver() {
        let (output, events) = recording();

        output.clone().send(started());

        assert_eq!(
            events
                .lock()
                .expect("no test panics while it holds the lock")
                .len(),
            1
        );
    }

    #[test]
    fn tool_output_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolOutput>();
    }
}
