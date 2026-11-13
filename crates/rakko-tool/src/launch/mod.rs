//! The run of a tool that survives a launcher that Windows stopped
//!
//! A package manager for Node installs a tool as a script, with a launcher
//! beside it under the same name. On Windows the launcher is a batch file,
//! and [`Tool::resolve_requirement`][resolve] answers with it, because
//! Windows cannot start the script. A batch file runs in `cmd.exe`, and
//! `cmd.exe` stops when the operating system refuses it a file that another
//! process holds at that moment. It writes the message of the operating
//! system and ends without success, before the tool writes anything.
//!
//! The refusal lasts only as long as the other process holds the file, so a
//! run that starts again a moment later reaches the tool. This module holds
//! the run that does that, so that an action receives the answer of its tool
//! and not the refusal of the launcher.
//!
//! [resolve]: crate::Tool::resolve_requirement

mod shown_streams;

use std::future::Future;
use std::time::Duration;

use kawauso_process::error::RunCommandError;
use kawauso_process::run::Stream;
use kawauso_process::{Execution, Invocation};
use rakko::{ToolCommand, ToolEnd, ToolEvent, ToolExitCode, ToolLine, ToolOutput, ToolStream};

pub use self::shown_streams::ShownStreams;
use crate::seal::Sealed;

/// The number of times that a run starts a launcher that Windows refused
const ATTEMPTS: u32 = 3;

/// The pause before the second attempt, which grows with each attempt
const BACKOFF: Duration = Duration::from_millis(100);

/// The extensions of a batch file, which Windows runs in `cmd.exe`
#[cfg(windows)]
const BATCH_EXTENSIONS: [&str; 2] = ["bat", "cmd"];

/// The code of the operating system for a file that another process holds
#[cfg(windows)]
const SHARING_VIOLATION: i32 = 32;

/// The run of an invocation that starts a refused launcher again
///
/// [`launch`][launch] runs the invocation as [`Invocation::run`] does. When the
/// program is a batch file and `cmd.exe` reported only that another process
/// holds a file, the run starts again. The run waits a short pause before each
/// new start, and it starts again a few times at most. Every other result is
/// the answer, whatever its exit status. So a tool that reports a problem of
/// the project never runs twice.
///
/// Every operating system but Windows starts a tool directly, so there a
/// launch is one run.
///
/// [`launch_with`][launch-with] runs the invocation in the same way, and it
/// sends what the tool writes to a tool output while the tool runs. An action
/// calls `launch_with` for every run of its tools, with the tool output of its
/// context. So the harness can show its tools to a person. The machinery
/// calls [`launch`][launch] for its own runs, such as the question to mise for
/// the location of a tool.
///
/// An action never implements `Launch`. `Launch` has its one implementation in
/// this crate, for [`Invocation`], and no crate outside this one can implement
/// it. So a later compatible release can add a method to `Launch`.
///
/// [launch]: Launch::launch
/// [launch-with]: Launch::launch_with
///
/// # Examples
///
/// ```no_run
/// use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
/// use rakko_tool::{Launch, ShownStreams, Tool};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let requirement = ToolRequirement::builder()
///     .tool("npm:prettier")
///     .executable("prettier")
///     .version(RequiredVersion::Exact(ToolVersion::new("3.9.10")))
///     .build();
/// let context = Context::builder().root("/home/otter/project").build();
///
/// let tool = Tool::resolve_requirement().requirement(&requirement).context(&context).call().await?;
///
/// let execution = tool
///     .invocation()
///     .arg("--check")
///     .arg(".")
///     .launch_with(context.tool_output(), ShownStreams::Both)
///     .await?;
///
/// println!("{}", execution.status());
/// # Ok(())
/// # }
/// ```
///
/// An implementation outside this crate does not compile:
///
/// ```compile_fail
/// use rakko_tool::{Execution, Launch, RunCommandError};
///
/// struct Script;
///
/// impl Launch for Script {
///     async fn launch(&self) -> Result<Execution, RunCommandError> {
///         unimplemented!()
///     }
/// }
/// ```
pub trait Launch: Sealed {
    /// Runs the invocation, and starts it again when Windows refused its
    /// launcher
    ///
    /// A refused launcher wrote the message of the operating system for a file
    /// that another process holds, and nothing of its tool. Every attempt asks
    /// the tool the same question, so another attempt changes no answer. When
    /// the last attempt is refused as well, the result is that attempt. The
    /// caller reports it as a run that said nothing about the project.
    ///
    /// The message arrives in the language of the system. A launcher that
    /// writes it in a code page that this crate does not read is not
    /// recognized, and its run is the answer.
    ///
    /// # Errors
    ///
    /// Returns a [`RunCommandError`] when the command does not start or its
    /// output cannot be read, as [`Invocation::run`] does.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime with a timer drives the future. The run
    /// waits for the program, and for the pause before another attempt.
    fn launch(&self) -> impl Future<Output = Result<Execution, RunCommandError>> + Send;

    /// Runs the invocation as [`launch`][launch] does, and sends what the tool
    /// writes to a tool output while it runs
    ///
    /// An action passes the [`ToolOutput`] of its context, so that the harness
    /// can show the tool to a reader while it runs. The tool output receives a
    /// [`ToolEvent::Started`] event with the command and a [`ToolEvent::Line`]
    /// event for each line of a stream that `shown` lets through. Then it
    /// receives a [`ToolEvent::Ended`] event with the exit code. Each attempt
    /// of a refused launcher sends its own events.
    ///
    /// The command of the start and of the end holds the program and its
    /// arguments. It leaves out the variables that the invocation sets, because
    /// a value such as a whole `PATH` would make each line long and unreadable.
    ///
    /// The result is the same as the result of [`launch`][launch]. The capture
    /// of both streams holds every byte that the tool wrote, so the action
    /// parses it as before. The run reads the lines in the same way for every
    /// tool output, also for one that discards them.
    ///
    /// # Errors
    ///
    /// Returns a [`RunCommandError`] when the command does not start or its
    /// output cannot be read, as [`launch`][launch] does. A run that fails after
    /// the start sends no [`ToolEvent::Ended`] event.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime with a timer drives the future, as
    /// [`launch`][launch] does.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
    /// use rakko_tool::{Launch, ShownStreams, Tool};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let requirement = ToolRequirement::builder()
    ///     .tool("taplo")
    ///     .executable("taplo")
    ///     .version(RequiredVersion::Exact(ToolVersion::new("0.10.0")))
    ///     .build();
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let tool = Tool::resolve_requirement().requirement(&requirement).context(&context).call().await?;
    ///
    /// let execution = tool
    ///     .invocation()
    ///     .arg("check")
    ///     .launch_with(context.tool_output(), ShownStreams::Both)
    ///     .await?;
    ///
    /// println!("{}", execution.status());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [launch]: Launch::launch
    fn launch_with(
        &self,
        output: &ToolOutput,
        shown: ShownStreams,
    ) -> impl Future<Output = Result<Execution, RunCommandError>> + Send;
}

impl Sealed for Invocation {}

impl Launch for Invocation {
    async fn launch(&self) -> Result<Execution, RunCommandError> {
        let mut execution = self.run().await?;

        for attempt in 1..ATTEMPTS {
            if !refused(&execution) {
                break;
            }

            tokio::time::sleep(BACKOFF * attempt).await;
            execution = self.run().await?;
        }

        Ok(execution)
    }

    async fn launch_with(
        &self,
        output: &ToolOutput,
        shown: ShownStreams,
    ) -> Result<Execution, RunCommandError> {
        let mut execution = observe(self, output, shown).await?;

        for attempt in 1..ATTEMPTS {
            if !refused(&execution) {
                break;
            }

            tokio::time::sleep(BACKOFF * attempt).await;
            execution = observe(self, output, shown).await?;
        }

        Ok(execution)
    }
}

/// Runs an invocation once, and sends its start, the lines of the shown
/// streams, and its end to a tool output
///
/// The run reads each line of both streams as the tool writes it, and it keeps
/// the same capture that [`Invocation::run`] keeps.
///
/// # Errors
///
/// Returns a [`RunCommandError`] when the command does not start or when its
/// output cannot be read. It also returns one when the end of the command
/// cannot be waited for.
async fn observe(
    invocation: &Invocation,
    output: &ToolOutput,
    shown: ShownStreams,
) -> Result<Execution, RunCommandError> {
    let command = command(invocation);
    let mut run = invocation.start()?;

    output.send(ToolEvent::Started(command.clone()));

    while let Some(line) = run.next_line().await? {
        if !shown.shows(line.stream()) {
            continue;
        }

        let stream = match line.stream() {
            Stream::StandardError => ToolStream::StandardError,
            Stream::StandardOutput => ToolStream::StandardOutput,
        };

        output.send(ToolEvent::Line(
            ToolLine::builder()
                .stream(stream)
                .text(line.text().get())
                .build(),
        ));
    }

    let execution = run.wait().await?;

    output.send(ToolEvent::Ended(
        ToolEnd::builder()
            .command(command)
            .maybe_code(
                execution
                    .status()
                    .code()
                    .map(|code| ToolExitCode::new(i64::from(code))),
            )
            .build(),
    ));

    Ok(execution)
}

/// Returns the command of an invocation as a reader reads it
///
/// The text holds the program and its arguments, separated by spaces. A word
/// that is empty or that holds a space goes between quotation marks. So a
/// reader sees where it starts and where it ends. The text leaves out the
/// variables that the invocation sets and takes out.
fn command(invocation: &Invocation) -> ToolCommand {
    let words: Vec<String> = std::iter::once(invocation.program().get().to_string_lossy())
        .chain(
            invocation
                .arguments()
                .iter()
                .map(|argument| argument.get().to_string_lossy()),
        )
        .map(|word| {
            if word.is_empty() || word.contains(char::is_whitespace) {
                format!("\"{word}\"")
            } else {
                word.into_owned()
            }
        })
        .collect();

    ToolCommand::new(words.join(" "))
}

/// Returns whether Windows refused the launcher of a run
///
/// A launcher is a batch file, so the extension of the program decides whether
/// `cmd.exe` ran it. A refused launcher ended without success. It wrote nothing
/// on its standard output stream and only the message of the operating system
/// on its standard error stream.
#[cfg(windows)]
fn refused(execution: &Execution) -> bool {
    let batch = execution
        .invocation()
        .program()
        .get()
        .extension()
        .is_some_and(|extension| {
            BATCH_EXTENSIONS
                .iter()
                .any(|batch| extension.eq_ignore_ascii_case(batch))
        });

    batch
        && !execution.status().success()
        && execution.stdout().to_string_lossy().trim().is_empty()
        && execution.stderr().to_string_lossy().trim() == sharing_violation()
}

/// Returns whether Windows refused the launcher of a run
///
/// No launcher stands between a run and its tool on this operating system.
#[cfg(not(windows))]
fn refused(_execution: &Execution) -> bool {
    false
}

/// Returns the message of the operating system for a file that another
/// process holds
///
/// The standard library asks the operating system for the text, in the
/// language of the system, and appends the code in parentheses. A launcher
/// writes the text alone, so the code comes off again.
#[cfg(windows)]
fn sharing_violation() -> String {
    let message = std::io::Error::from_raw_os_error(SHARING_VIOLATION).to_string();
    let code = format!(" (os error {SHARING_VIOLATION})");

    message
        .strip_suffix(&code)
        .map_or_else(|| message.clone(), str::to_owned)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::Path;
    #[cfg(windows)]
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    #[cfg(windows)]
    use kawauso_process::invocation::Program;
    use rakko::{Context, ProjectRoot, RequiredVersion, ToolRequirement, ToolVersion};
    #[cfg(windows)]
    use tempfile::TempDir;

    use super::*;
    use crate::{MiseFile, Tool};

    /// A launcher that the operating system refuses once and then runs
    #[cfg(windows)]
    const REFUSED_ONCE: &str = "@echo off\r\n\
                                echo.>>\"%~dp0attempts\"\r\n\
                                if exist \"%~dp0refused\" (\r\n\
                                \x20 echo ran\r\n\
                                \x20 exit /b 0\r\n\
                                )\r\n\
                                type nul > \"%~dp0refused\"\r\n\
                                type \"%~dp0message.txt\" 1>&2\r\n\
                                exit /b 1\r\n";

    /// A launcher that the operating system refuses every time
    #[cfg(windows)]
    const REFUSED_ALWAYS: &str = "@echo off\r\n\
                                  echo.>>\"%~dp0attempts\"\r\n\
                                  type \"%~dp0message.txt\" 1>&2\r\n\
                                  exit /b 1\r\n";

    /// A launcher whose tool fails for a reason of its own
    #[cfg(windows)]
    const FAILING: &str = "@echo off\r\n\
                           echo.>>\"%~dp0attempts\"\r\n\
                           echo the tool stops here 1>&2\r\n\
                           exit /b 1\r\n";

    /// A launcher in a temporary directory, beside the message that it
    /// writes and the record of its attempts
    #[cfg(windows)]
    struct Launcher {
        /// The directory that holds the launcher
        directory: TempDir,

        /// The batch file that a run starts
        script: PathBuf,
    }

    #[cfg(windows)]
    impl Launcher {
        /// Writes a launcher with the given body
        fn new(body: &str) -> Self {
            let directory = tempfile::tempdir().expect("the test creates a temporary directory");
            let script = directory.path().join("launcher.cmd");
            std::fs::write(&script, body).expect("the test writes the launcher");
            std::fs::write(directory.path().join("message.txt"), sharing_violation())
                .expect("the test writes the message of the operating system");

            Self { directory, script }
        }

        /// Returns how often a run started the launcher
        fn attempts(&self) -> usize {
            std::fs::read_to_string(self.directory.path().join("attempts"))
                .map(|record| record.lines().count())
                .unwrap_or_default()
        }

        /// Returns the invocation that starts the launcher
        fn invocation(&self) -> Invocation {
            Invocation::new(Program::from(self.script.clone()))
        }
    }

    /// Returns the root of the repository, which pins the tools of the tests
    ///
    /// The crate lives two directories below the root of the repository.
    fn root() -> ProjectRoot {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository");

        ProjectRoot::from(root)
    }

    /// Shows that the launch type satisfies Send
    ///
    /// A scheduler runs actions in parallel and can move a run to a different
    /// thread while it waits. This test holds the launch to the auto trait that
    /// makes this possible. It does so because a change to the future could
    /// take it away without a word from the compiler.
    #[test]
    fn launch_is_send() {
        fn assert_send<T: Send>(_: &T) {}

        let invocation = Invocation::new("jq");

        assert_send(&invocation.launch());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn launch_of_a_launcher_that_failed_for_another_reason_runs_it_once() {
        let launcher = Launcher::new(FAILING);

        let _execution = launcher.invocation().launch().await;

        assert_eq!(launcher.attempts(), 1);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn launch_of_a_launcher_that_is_always_refused_stops_after_the_last_attempt() {
        let launcher = Launcher::new(REFUSED_ALWAYS);

        let _execution = launcher.invocation().launch().await;

        assert_eq!(launcher.attempts(), ATTEMPTS as usize);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn launch_of_a_launcher_that_was_refused_once_reports_the_tool() {
        let launcher = Launcher::new(REFUSED_ONCE);

        let execution = launcher
            .invocation()
            .launch()
            .await
            .expect("expected the launcher to run");

        assert_eq!(execution.stdout().to_string_lossy().trim(), "ran");
    }

    #[tokio::test]
    async fn launch_of_a_tool_reports_what_it_wrote() {
        let tool = jq().await;

        let execution = tool
            .invocation()
            .arg("--null-input")
            .arg("--raw-output")
            .arg("\"launched\"")
            .launch()
            .await
            .expect("expected the tool to run");

        assert_eq!(execution.stdout().to_string_lossy().trim(), "launched");
    }

    #[tokio::test]
    async fn launch_with_a_discarding_output_returns_the_capture() {
        let tool = jq().await;

        let execution = tool
            .invocation()
            .arg("--null-input")
            .arg("--raw-output")
            .arg("\"launched\"")
            .launch_with(&ToolOutput::discard(), ShownStreams::Both)
            .await
            .expect("expected the tool to run");

        assert_eq!(execution.stdout().to_string_lossy().trim(), "launched");
    }

    /// Shows that the launch with a tool output satisfies Send, as
    /// [`launch_is_send`] does for the plain launch
    #[test]
    fn launch_with_is_send() {
        fn assert_send<T: Send>(_: &T) {}

        let invocation = Invocation::new("jq");
        let output = ToolOutput::discard();

        assert_send(&invocation.launch_with(&output, ShownStreams::Both));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn launch_with_of_a_launcher_that_was_refused_once_reports_the_tool() {
        let launcher = Launcher::new(REFUSED_ONCE);
        let (output, _events) = recording();

        let execution = launcher
            .invocation()
            .launch_with(&output, ShownStreams::Both)
            .await
            .expect("expected the launcher to run");

        assert_eq!(execution.stdout().to_string_lossy().trim(), "ran");
    }

    #[tokio::test]
    async fn launch_with_quotes_an_argument_that_holds_a_space() {
        let tool = jq().await;
        let (output, events) = recording();

        tool.invocation()
            .arg("--null-input")
            .arg("\"two words\"")
            .launch_with(&output, ShownStreams::Both)
            .await
            .expect("expected the tool to run");

        let quoted = events
            .lock()
            .expect("no test panics while it holds the lock")
            .first()
            .is_some_and(|event| match event {
                ToolEvent::Started(command) => command.get().ends_with(" \"\"two words\"\""),
                _ => false,
            });
        assert!(quoted);
    }

    #[tokio::test]
    async fn launch_with_returns_the_capture() {
        let tool = jq().await;
        let (output, _events) = recording();

        let execution = tool
            .invocation()
            .arg("--null-input")
            .arg("--raw-output")
            .arg("\"launched\"")
            .launch_with(&output, ShownStreams::Both)
            .await
            .expect("expected the tool to run");

        assert_eq!(execution.stdout().to_string_lossy().trim(), "launched");
    }

    #[tokio::test]
    async fn launch_with_sends_a_line_of_the_standard_error_stream() {
        let events = failure().await;

        assert!(
            events.contains(&ToolEvent::Line(
                ToolLine::builder()
                    .stream(ToolStream::StandardError)
                    .text("stopped")
                    .build()
            ))
        );
    }

    #[tokio::test]
    async fn launch_with_sends_each_line_of_the_standard_output_stream() {
        let tool = jq().await;
        let (output, events) = recording();

        tool.invocation()
            .arg("--null-input")
            .arg("--raw-output")
            .arg("\"first\", \"second\"")
            .launch_with(&output, ShownStreams::Both)
            .await
            .expect("expected the tool to run");

        let lines: Vec<ToolLine> = events
            .lock()
            .expect("no test panics while it holds the lock")
            .iter()
            .filter_map(|event| match event {
                ToolEvent::Line(line) => Some(line.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            lines,
            [
                ToolLine::builder()
                    .stream(ToolStream::StandardOutput)
                    .text("first")
                    .build(),
                ToolLine::builder()
                    .stream(ToolStream::StandardOutput)
                    .text("second")
                    .build(),
            ]
        );
    }

    #[tokio::test]
    async fn launch_with_sends_the_command_without_the_environment() {
        let tool = jq().await;
        let (output, events) = recording();

        tool.invocation()
            .env("RAKKO_TEST_VARIABLE", "hidden")
            .arg("--null-input")
            .arg("1")
            .launch_with(&output, ShownStreams::Both)
            .await
            .expect("expected the tool to run");

        let hidden = events
            .lock()
            .expect("no test panics while it holds the lock")
            .first()
            .is_some_and(|event| match event {
                ToolEvent::Started(command) => !command.get().contains("RAKKO_TEST_VARIABLE"),
                _ => false,
            });
        assert!(hidden);
    }

    #[tokio::test]
    async fn launch_with_sends_the_exit_code_at_the_end() {
        let events = failure().await;

        let end = events.last().and_then(|event| match event {
            ToolEvent::Ended(end) => end.code(),
            _ => None,
        });
        assert_eq!(end, Some(ToolExitCode::new(3)));
    }

    #[tokio::test]
    async fn launch_with_sends_the_start_first() {
        let events = failure().await;

        let started = events.first().is_some_and(|event| match event {
            ToolEvent::Started(command) => command.get().contains("halt_error"),
            _ => false,
        });
        assert!(started);
    }

    #[tokio::test]
    async fn launch_with_showing_the_standard_error_stream_sends_no_line_of_the_standard_output_stream()
     {
        let lines = both_streams(ShownStreams::StandardError).await;

        assert_eq!(
            lines,
            [ToolLine::builder()
                .stream(ToolStream::StandardError)
                .text("err")
                .build()]
        );
    }

    #[tokio::test]
    async fn launch_with_showing_the_standard_output_stream_sends_no_line_of_the_standard_error_stream()
     {
        let lines = both_streams(ShownStreams::StandardOutput).await;

        assert_eq!(
            lines,
            [ToolLine::builder()
                .stream(ToolStream::StandardOutput)
                .text("out")
                .build()]
        );
    }

    /// Runs jq so that it writes `out` to its standard output stream and `err`
    /// to its standard error stream
    ///
    /// Returns the lines that reached a tool output that shows the given
    /// streams.
    async fn both_streams(shown: ShownStreams) -> Vec<ToolLine> {
        let tool = jq().await;
        let (output, events) = recording();

        tool.invocation()
            .arg("--null-input")
            .arg("--raw-output")
            .arg("\"out\", (\"err\\n\" | halt_error(3))")
            .launch_with(&output, shown)
            .await
            .expect("expected the tool to run");

        events
            .lock()
            .expect("no test panics while it holds the lock")
            .iter()
            .filter_map(|event| match event {
                ToolEvent::Line(line) => Some(line.clone()),
                _ => None,
            })
            .collect()
    }

    /// Runs jq so that it writes `stopped` to its standard error stream and
    /// ends with the exit code 3
    ///
    /// Returns the events that the run sent.
    async fn failure() -> Vec<ToolEvent> {
        let tool = jq().await;
        let (output, events) = recording();

        tool.invocation()
            .arg("--null-input")
            .arg("\"stopped\\n\" | halt_error(3)")
            .launch_with(&output, ShownStreams::Both)
            .await
            .expect("expected the tool to run");

        events
            .lock()
            .expect("no test panics while it holds the lock")
            .clone()
    }

    /// Returns the jq that the repository pins
    async fn jq() -> Tool {
        let pins = std::fs::read_to_string(root().get().join("mise.toml"))
            .expect("the test reads the mise.toml of the repository");
        let requirement = ToolRequirement::builder()
            .tool("jq")
            .executable("jq")
            .version(RequiredVersion::Exact(ToolVersion::new(
                MiseFile::new(&pins).pinned_version("jq"),
            )))
            .build();
        let context = Context::builder().root(root()).build();

        Tool::resolve_requirement()
            .requirement(&requirement)
            .context(&context)
            .call()
            .await
            .expect("expected mise to report the location of a tool that this project pins")
    }

    /// Returns a tool output and the list of the events that it received
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
}
