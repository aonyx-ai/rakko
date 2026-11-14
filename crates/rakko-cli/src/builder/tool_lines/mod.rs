//! The part of the command line that shows what the tools of an action write
//!
//! An action sends the start, the lines, and the end of each tool that it runs
//! to the tool output of its context. The command line chooses that tool
//! output. Under `--verbose`, it shows the events to the reader. Otherwise it
//! gives the action a tool output that discards them, so nothing is shown and
//! nothing is held.
//!
//! An action that runs alone shows its events as they arrive. A member of a
//! group that runs beside other members holds its events in a [`Buffer`]
//! until it finishes. So the events of two members never mix.
//!
//! The start and the end of a tool go out as details, and each line goes out
//! as a line of a process. In text mode, a line from the standard error
//! stream of the tool goes to the standard error stream of the command line.
//! The other events go to its standard output stream. In JSON mode,
//! every event goes to the standard error stream, and the standard output
//! stream carries only the document.

mod buffer;

use std::future::Future;
use std::pin::pin;

use clap::ArgMatches;
use clawless::event::SendError;
use clawless::event::process::{ProcessEvent, RunId};
use clawless::output::{Output, OutputFlags, Verbosity};
use clawless::process::{Line, Stream};
use rakko::{Context, ToolEnd, ToolEvent, ToolOutput, ToolStream};
use tokio::sync::mpsc;

pub(crate) use self::buffer::Buffer;

/// Whether a run shows what the tools of its actions write
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum ToolLines {
    /// The run discards every event of a tool
    Hidden,

    /// The run shows every event of a tool to the reader
    Shown,
}

impl ToolLines {
    /// Returns whether the flags of a run ask for the events of the tools
    ///
    /// Only `--verbose` asks for them. The environment of the run plays no
    /// part, so the same command shows the same output everywhere.
    pub(crate) fn from_matches(matches: &ArgMatches) -> Self {
        match OutputFlags::from_arg_matches(matches).verbosity() {
            Verbosity::Verbose => Self::Shown,
            Verbosity::Quiet | Verbosity::Default => Self::Hidden,
        }
    }
}

/// Runs an action and shows the events of its tools as they arrive
///
/// The run receives a context with the root of the project. When the run
/// shows the events of the tools, that context sends each event to the output
/// while the action runs. Otherwise its tool output discards every event.
///
/// # Errors
///
/// Returns a [`SendError`] when an event did not reach the reader. The action
/// still runs to its end, and its value is lost. The events that the action
/// sends after that are dropped, because the run already fails.
pub(crate) async fn stream<R, F, T>(
    lines: ToolLines,
    project: &Context,
    output: &Output,
    run: R,
) -> Result<T, SendError>
where
    R: FnOnce(Context) -> F,
    F: Future<Output = T>,
{
    match lines {
        ToolLines::Hidden => Ok(run(project.clone()).await),
        ToolLines::Shown => forward(project, output, run).await,
    }
}

/// Runs an action with a tool output that sends each event to the output while
/// the action runs
///
/// When the action finishes, the run closes the receiver and shows the queued
/// events. A retained context or tool output cannot keep the run open.
///
/// # Errors
///
/// Returns a [`SendError`] when an event did not reach the reader, as
/// [`stream`] describes.
async fn forward<R, F, T>(project: &Context, output: &Output, run: R) -> Result<T, SendError>
where
    R: FnOnce(Context) -> F,
    F: Future<Output = T>,
{
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let context = Context::builder()
        .root(project.root().clone())
        .tool_output(ToolOutput::new(move |event| {
            drop(sender.send(event));
        }))
        .build();
    let id = RunId::next();

    let mut run = pin!(run(context));

    let value = loop {
        tokio::select! {
            value = &mut run => break value,
            event = receiver.recv() => {
                let Some(event) = event else {
                    return Ok(run.await);
                };

                if let Err(source) = present(event, id, output).await {
                    drop(receiver);
                    drop(run.await);

                    return Err(source);
                }
            }
        }
    };

    receiver.close();
    while let Some(event) = receiver.recv().await {
        present(event, id, output).await?;
    }

    Ok(value)
}

/// Shows one event of a tool to the reader
///
/// A line goes out under the identity of the run. So a reader that groups the
/// lines of a process keeps the lines of one action together. A kind of event
/// that this command line does not know is not shown.
///
/// # Errors
///
/// Returns a [`SendError`] when the event did not reach the reader.
async fn present(event: ToolEvent, id: RunId, output: &Output) -> Result<(), SendError> {
    match event {
        ToolEvent::Started(command) => output.detail(format!("$ {command}")).await,
        ToolEvent::Line(line) => {
            let stream = match line.stream() {
                ToolStream::StandardOutput => Stream::StandardOutput,
                _ => Stream::StandardError,
            };

            output
                .process_event(ProcessEvent::Line {
                    id,
                    line: Line::new(stream, line.text().get()),
                })
                .await
        }
        ToolEvent::Ended(end) => output.detail(ended(&end)).await,
        _ => Ok(()),
    }
}

/// Returns the text that tells the reader how a tool ended
fn ended(end: &ToolEnd) -> String {
    match end.code() {
        Some(code) => format!("{} exited with code {code}", end.command()),
        None => format!("{} ended without an exit code", end.command()),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
    use std::time::Duration;

    use clap::Command;
    use clawless::event::{Event, EventReceiver, event_channel};
    use rakko::{Outcome, ToolCommand, ToolExitCode, ToolLine};

    use super::*;

    /// An error that keeps the context of a run for its diagnosis
    #[derive(Debug, thiserror::Error)]
    enum RunActionError {
        /// The tool is unavailable in the project of the run
        #[error("the tool is unavailable in {}", context.root().get().display())]
        ToolUnavailable { context: Context },
    }

    /// Returns an output and the receiver of what it sends
    fn channel() -> (Output, EventReceiver) {
        let (sender, receiver) = event_channel();

        (Output::new(sender), receiver)
    }

    /// Returns the end of a tool that exited with the given code
    fn end(code: Option<i64>) -> ToolEvent {
        ToolEvent::Ended(
            ToolEnd::builder()
                .command("taplo check")
                .maybe_code(code.map(ToolExitCode::new))
                .build(),
        )
    }

    /// Returns a line that a tool wrote to a stream
    fn line(stream: ToolStream, text: &str) -> ToolEvent {
        ToolEvent::Line(ToolLine::builder().stream(stream).text(text).build())
    }

    /// Returns what the output of a command line shows for the given flags
    fn lines(arguments: &[&str]) -> ToolLines {
        let command = OutputFlags::augment_command(Command::new("rakko"));
        let matches = command.get_matches_from(arguments);

        ToolLines::from_matches(&matches)
    }

    /// Returns the text of every event that the output sent
    ///
    /// The output goes away first, so the receiver ends after the last event.
    async fn received(output: Output, mut receiver: EventReceiver) -> Vec<String> {
        drop(output);
        let mut texts = Vec::new();

        while let Some(event) = receiver.recv().await {
            let text = match event {
                Event::Detail(detail) => format!("detail: {detail}"),
                Event::Process(process) => match *process {
                    ProcessEvent::Line { line, .. } => format!("{}: {line}", line.stream()),
                    other => format!("process: {other}"),
                },
                Event::Message(message) => format!("message: {message}"),
                Event::Artifact(_) | Event::Prompt(_) => String::from("other"),
            };
            texts.push(text);
        }

        texts
    }

    /// Runs an action that sends the given events, and returns what the reader
    /// received
    async fn streamed(lines: ToolLines, events: Vec<ToolEvent>) -> Vec<String> {
        let (output, receiver) = channel();
        let project = Context::builder().root("/tmp/project").build();

        stream(lines, &project, &output, |context| async move {
            for event in events {
                context.tool_output().send(event);
            }
        })
        .await
        .expect("the receiver lives until the run ends");

        received(output, receiver).await
    }

    #[tokio::test]
    async fn stream_returns_the_value_of_the_run() {
        let (output, _receiver) = channel();
        let project = Context::builder().root("/tmp/project").build();

        let value = stream(ToolLines::Shown, &project, &output, |_context| async { 7 }).await;

        assert_eq!(value.ok(), Some(7));
    }

    #[tokio::test]
    async fn stream_with_hidden_lines_presents_nothing() {
        let texts = streamed(
            ToolLines::Hidden,
            vec![ToolEvent::Started(ToolCommand::new("taplo check"))],
        )
        .await;

        assert!(texts.is_empty());
    }

    #[tokio::test]
    async fn stream_with_shown_lines_and_a_retained_context_returns_the_error_and_all_events() {
        let (output, receiver) = channel();
        let project = Context::builder().root("/tmp/project").build();
        let run = stream(ToolLines::Shown, &project, &output, |context| async move {
            context
                .tool_output()
                .send(ToolEvent::Started(ToolCommand::new("taplo check")));
            context
                .tool_output()
                .send(line(ToolStream::StandardError, "the tool stopped"));
            context.tool_output().send(end(Some(1)));

            Outcome::Errored {
                source: Box::new(RunActionError::ToolUnavailable { context }),
            }
        });

        let outcome = tokio::time::timeout(Duration::from_secs(5), run)
            .await
            .expect("the run must finish while its error retains the context")
            .expect("the receiver lives until the run ends");
        let diagnosis = match &outcome {
            Outcome::Errored { source } => Some(source.to_string()),
            _ => None,
        };
        let texts = received(output, receiver).await;

        assert_eq!(
            (diagnosis, texts),
            (
                Some(String::from("the tool is unavailable in /tmp/project")),
                vec![
                    String::from("detail: $ taplo check"),
                    String::from("standard error: the tool stopped"),
                    String::from("detail: taplo check exited with code 1"),
                ],
            )
        );
    }

    #[tokio::test]
    async fn stream_with_shown_lines_keeps_the_root_of_the_project() {
        let (output, _receiver) = channel();
        let project = Context::builder().root("/tmp/project").build();

        let root = stream(ToolLines::Shown, &project, &output, |context| async move {
            context.root().clone()
        })
        .await;

        assert_eq!(root.ok().as_ref(), Some(project.root()));
    }

    #[tokio::test]
    async fn stream_with_shown_lines_presents_a_line_of_the_standard_error_stream() {
        let texts = streamed(
            ToolLines::Shown,
            vec![line(ToolStream::StandardError, "warning: unused")],
        )
        .await;

        assert_eq!(texts, ["standard error: warning: unused"]);
    }

    #[tokio::test]
    async fn stream_with_shown_lines_presents_a_line_of_the_standard_output_stream() {
        let texts = streamed(
            ToolLines::Shown,
            vec![line(ToolStream::StandardOutput, "Compiling rakko")],
        )
        .await;

        assert_eq!(texts, ["standard output: Compiling rakko"]);
    }

    #[tokio::test]
    async fn stream_with_shown_lines_presents_an_event_while_the_run_still_runs() {
        let (output, mut receiver) = channel();
        let project = Context::builder().root("/tmp/project").build();
        let seen = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&seen);

        let run = stream(ToolLines::Shown, &project, &output, |context| async move {
            context
                .tool_output()
                .send(ToolEvent::Started(ToolCommand::new("taplo check")));

            while !signal.load(AtomicOrdering::SeqCst) {
                tokio::task::yield_now().await;
            }
        });
        let watch = async {
            let first = receiver.recv().await;
            seen.store(true, AtomicOrdering::SeqCst);

            first.is_some()
        };
        let joined =
            tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(run, watch) }).await;

        assert!(matches!(joined, Ok((Ok(()), true))));
    }

    #[tokio::test]
    async fn stream_with_shown_lines_presents_the_end_of_a_stopped_tool() {
        let texts = streamed(ToolLines::Shown, vec![end(None)]).await;

        assert_eq!(texts, ["detail: taplo check ended without an exit code"]);
    }

    #[tokio::test]
    async fn stream_with_shown_lines_presents_the_end_with_its_exit_code() {
        let texts = streamed(ToolLines::Shown, vec![end(Some(1))]).await;

        assert_eq!(texts, ["detail: taplo check exited with code 1"]);
    }

    #[tokio::test]
    async fn stream_with_shown_lines_presents_the_start_with_its_command() {
        let texts = streamed(
            ToolLines::Shown,
            vec![ToolEvent::Started(ToolCommand::new("taplo check"))],
        )
        .await;

        assert_eq!(texts, ["detail: $ taplo check"]);
    }

    #[tokio::test]
    async fn stream_without_a_reader_reports_an_error() {
        let (output, receiver) = channel();
        drop(receiver);
        let project = Context::builder().root("/tmp/project").build();

        let result = stream(ToolLines::Shown, &project, &output, |context| async move {
            context
                .tool_output()
                .send(ToolEvent::Started(ToolCommand::new("taplo check")));
        })
        .await;

        assert!(result.is_err());
    }

    #[test]
    fn tool_lines_from_matches_with_quiet_hides_the_lines() {
        let lines = lines(&["rakko", "--quiet"]);

        assert_eq!(lines, ToolLines::Hidden);
    }

    #[test]
    fn tool_lines_from_matches_with_verbose_shows_the_lines() {
        let lines = lines(&["rakko", "--verbose"]);

        assert_eq!(lines, ToolLines::Shown);
    }

    #[test]
    fn tool_lines_from_matches_without_a_flag_hides_the_lines() {
        let lines = lines(&["rakko"]);

        assert_eq!(lines, ToolLines::Hidden);
    }

    #[test]
    fn tool_lines_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolLines>();
    }
}
