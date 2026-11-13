//! The events of the tools of one member of a group, held until it finishes

use std::sync::{Arc, Mutex, PoisonError};

use clawless::event::SendError;
use clawless::event::process::RunId;
use clawless::output::Output;
use rakko::{Context, Name, ToolEvent, ToolOutput};

use super::{ToolLines, present};

/// The events of the tools of one member of a group, held until it finishes
///
/// The members of a group that read run at the same time. If each of them
/// showed its events as they arrive, the lines of two members would mix. So
/// each such member gets a buffer of its own. When the member finishes, the
/// buffer shows its events in one block, under a header that gives the
/// member.
///
/// A buffer for a run that does not show the events of the tools holds
/// nothing, and its member gets a tool output that discards.
#[derive(Debug)]
pub(crate) struct Buffer {
    /// The events that the member sent, in the order of their arrival
    ///
    /// `None` when the run does not show the events of the tools.
    events: Option<Arc<Mutex<Vec<ToolEvent>>>>,
}

impl Buffer {
    /// Returns the context of a member and the buffer that its tool output
    /// fills
    ///
    /// The context holds the root of the project.
    pub(crate) fn attach(lines: ToolLines, project: &Context) -> (Context, Self) {
        match lines {
            ToolLines::Hidden => (project.clone(), Self { events: None }),
            ToolLines::Shown => Self::recording(project),
        }
    }

    /// Returns the context of a member with a tool output that fills a new
    /// buffer, and that buffer
    fn recording(project: &Context) -> (Context, Self) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&events);
        let context = Context::builder()
            .root(project.root().clone())
            .tool_output(ToolOutput::new(move |event| {
                received
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(event);
            }))
            .build();

        (
            context,
            Self {
                events: Some(events),
            },
        )
    }

    /// Shows every event of the member under a header that gives the member
    ///
    /// A buffer that holds no event shows nothing, not even the header.
    ///
    /// # Errors
    ///
    /// Returns a [`SendError`] when the header or an event did not reach the
    /// reader.
    pub(crate) async fn flush(self, member: &Name, output: &Output) -> Result<(), SendError> {
        let Some(events) = self.events else {
            return Ok(());
        };

        let events = std::mem::take(&mut *events.lock().unwrap_or_else(PoisonError::into_inner));
        if events.is_empty() {
            return Ok(());
        }

        output.detail(format!("--- {member} ---")).await?;

        let id = RunId::next();
        for event in events {
            present(event, id, output).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use clawless::event::{Event, EventReceiver, event_channel};
    use rakko::{ToolCommand, action_name};

    use super::*;

    /// Returns the details that an output sent, in their order
    ///
    /// The output goes away first, so the receiver ends after the last event.
    async fn details(output: Output, mut receiver: EventReceiver) -> Vec<String> {
        drop(output);
        let mut details = Vec::new();

        while let Some(event) = receiver.recv().await {
            if let Event::Detail(detail) = event {
                details.push(detail);
            }
        }

        details
    }

    /// Returns a context with a tool output that the buffer fills, the buffer,
    /// and the project that the context belongs to
    fn attached(lines: ToolLines) -> (Context, Buffer, Context) {
        let project = Context::builder().root("/tmp/project").build();
        let (context, buffer) = Buffer::attach(lines, &project);

        (context, buffer, project)
    }

    fn started() -> ToolEvent {
        ToolEvent::Started(ToolCommand::new("taplo check"))
    }

    #[test]
    fn attach_with_shown_lines_keeps_the_root_of_the_project() {
        let (context, _buffer, project) = attached(ToolLines::Shown);

        assert_eq!(context.root(), project.root());
    }

    #[test]
    fn buffer_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Buffer>();
    }

    #[tokio::test]
    async fn flush_of_a_buffer_without_events_shows_nothing() {
        let (_context, buffer, _project) = attached(ToolLines::Shown);
        let (sender, receiver) = event_channel();
        let output = Output::new(sender);

        buffer
            .flush(&action_name!("lint-toml"), &output)
            .await
            .expect("the receiver lives until the flush ends");

        assert!(details(output, receiver).await.is_empty());
    }

    #[tokio::test]
    async fn flush_shows_the_header_before_the_events() {
        let (context, buffer, _project) = attached(ToolLines::Shown);
        context.tool_output().send(started());
        let (sender, receiver) = event_channel();
        let output = Output::new(sender);

        buffer
            .flush(&action_name!("lint-toml"), &output)
            .await
            .expect("the receiver lives until the flush ends");

        assert_eq!(
            details(output, receiver).await,
            ["--- lint-toml ---", "$ taplo check"]
        );
    }

    #[tokio::test]
    async fn flush_with_hidden_lines_shows_nothing() {
        let (context, buffer, _project) = attached(ToolLines::Hidden);
        context.tool_output().send(started());
        let (sender, receiver) = event_channel();
        let output = Output::new(sender);

        buffer
            .flush(&action_name!("lint-toml"), &output)
            .await
            .expect("the receiver lives until the flush ends");

        assert!(details(output, receiver).await.is_empty());
    }

    #[tokio::test]
    async fn flush_without_a_reader_reports_an_error() {
        let (context, buffer, _project) = attached(ToolLines::Shown);
        context.tool_output().send(started());
        let (sender, receiver) = event_channel();
        drop(receiver);

        let result = buffer
            .flush(&action_name!("lint-toml"), &Output::new(sender))
            .await;

        assert!(result.is_err());
    }
}
