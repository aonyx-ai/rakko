//! The run of a tool that survives a launcher that Windows stopped
//!
//! A package manager for Node installs a tool as a script, with a launcher
//! beside it under the same name. On Windows the launcher is a batch file,
//! and [`Tool::resolve`][resolve] answers with it, because Windows cannot
//! start the script. A batch file runs in `cmd.exe`, and `cmd.exe` stops when
//! the operating system refuses it a file that another process holds at
//! that moment. It writes the message of the operating system and ends
//! without success, before the tool writes anything.
//!
//! The refusal lasts only as long as the other process holds the file, so a
//! run that starts again a moment later reaches the tool. This module holds
//! the run that does that, so that an action receives the answer of its tool
//! and not the refusal of the launcher.
//!
//! [resolve]: crate::Tool::resolve

use std::future::Future;
use std::time::Duration;

use kawauso_process::error::RunCommandError;
use kawauso_process::{Execution, Invocation};

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
/// [`launch`][launch] runs the invocation as [`Invocation::run`] does. When
/// the program is a batch file and `cmd.exe` reported only that another
/// process holds a file, the run starts again after a short pause, a few
/// times at most. Every other result is the answer, whatever its exit status,
/// so a tool that reports a problem of the project never runs twice.
///
/// Every operating system but Windows starts a tool directly, so there a
/// launch is one run.
///
/// [launch]: Launch::launch
///
/// # Examples
///
/// ```no_run
/// use rakko_tool::{Launch, Tool, ToolName};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let tool = Tool::resolve(ToolName::new("prettier"), "/home/otter/project".into()).await?;
///
/// let execution = tool.invocation().arg("--check").arg(".").launch().await?;
///
/// println!("{}", execution.status());
/// # Ok(())
/// # }
/// ```
pub trait Launch {
    /// Runs the invocation, and starts it again when Windows refused its
    /// launcher
    ///
    /// A refused launcher wrote the message of the operating system for a
    /// file that another process holds, and nothing of its tool. Every
    /// attempt asks the tool the same question, so another attempt changes
    /// no answer. When the last attempt is refused as well, the result is
    /// that attempt, and the caller reports it as a run that said nothing
    /// about the project.
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
}

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
}

/// Returns whether Windows refused the launcher of a run
///
/// A launcher is a batch file, so the extension of the program decides
/// whether `cmd.exe` ran it. A refused one ended without success, wrote
/// nothing on its standard output stream, and wrote only the message of the
/// operating system on its standard error stream.
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
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;
    #[cfg(windows)]
    use std::path::PathBuf;

    #[cfg(windows)]
    use kawauso_process::invocation::Program;
    use rakko_action::ProjectRoot;
    #[cfg(windows)]
    use tempfile::TempDir;

    use super::*;
    use crate::{Tool, ToolName};

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

    // A scheduler runs actions in parallel and can move a run to a different
    // thread while it waits. This test holds the launch to the auto trait
    // that makes this possible, because a change to the future could take it
    // away without a word from the compiler.
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
        let tool = Tool::resolve(ToolName::new("jq"), root())
            .await
            .expect("expected mise to report the location of a tool that this project pins");

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
}
