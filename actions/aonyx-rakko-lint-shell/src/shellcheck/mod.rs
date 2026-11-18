//! The ShellCheck that a project runs
//!
//! This module holds the program that mise installed at the version that the
//! action declares, and the run that produces a report. An
//! action asks for a run, and everything between the action and the process
//! lives here.

mod error;
mod report;

use rakko::{Action, Context, FilePath, ToolOutput};
use rakko_tool::{Launch, ResolveToolError, ShownStreams, Tool};

pub(crate) use self::error::ObserveShellcheckError;
use crate::comment::ShellcheckComment;

/// The name that mise knows the tool by, which is also the name of its
/// program
const SHELLCHECK: &str = "shellcheck";

/// The option that asks ShellCheck for its report as data
///
/// ShellCheck draws each comment under the line of the script by default. The
/// same run writes the comments as data on request. Each comment then carries
/// the path, the range, the level, the code, and the message in fields. The
/// option selects how the report is shown, not how the tool behaves. Which
/// checks apply comes from the configuration of the project alone.
const FORMAT: &str = "--format";

/// The name of the format that carries the comments as data
///
/// ShellCheck also knows an older `json` format. That format counts a tab as
/// eight columns, so its columns do not agree with an editor. This format
/// counts a tab as one column.
const JSON1: &str = "json1";

/// The marker that ends the options of the command line
///
/// Every argument after it is a script. A script whose name starts with a dash
/// therefore stays a script.
const END_OF_OPTIONS: &str = "--";

/// The status of a ShellCheck that examined every script and found nothing
const CLEAN: i32 = 0;

/// The status of a ShellCheck that examined every script and found a problem
///
/// ShellCheck separates the problems of the scripts from its own failures. A
/// script that it cannot open ends the run with another status, and so does a
/// command line that it does not accept.
const COMMENTED: i32 = 1;

/// The ShellCheck that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it. So a run reaches
/// the version that the tests of the crate cover. Nothing here installs a tool:
/// provisioning is the job of mise, and a ShellCheck that mise does not report
/// stops the caller.
///
/// ShellCheck reads the `.shellcheckrc` files of the project when the command
/// line gives no option for a check. The command line of this crate gives none.
/// So a run agrees with a contributor that runs ShellCheck bare.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Shellcheck {
    /// The program that mise installed at the declared version
    tool: Tool,

    /// The tool output of the context that resolved ShellCheck
    ///
    /// Each run sends what ShellCheck writes to it.
    output: ToolOutput,
}

#[bon::bon]
impl Shellcheck {
    /// Runs ShellCheck over the given scripts and reads what it reported
    ///
    /// A run starts in the project root, and each script is relative to it.
    /// ShellCheck writes the report on its standard output stream. It uses the
    /// standard error stream for a script that it cannot open and for a
    /// command line that it does not accept.
    ///
    /// # Errors
    ///
    /// Returns [`ShellcheckUnavailable`][unavailable] when ShellCheck does
    /// not run. Returns [`IncompleteExamination`][incomplete] when it stopped
    /// before it examined every script. Returns
    /// [`UnreadableReport`][unreadable] when it wrote a report that this crate
    /// cannot read.
    ///
    /// [incomplete]: ObserveShellcheckError::IncompleteExamination
    /// [unavailable]: ObserveShellcheckError::ShellcheckUnavailable
    /// [unreadable]: ObserveShellcheckError::UnreadableReport
    pub(crate) async fn observe(
        &self,
        scripts: &[FilePath],
    ) -> Result<Vec<ShellcheckComment>, ObserveShellcheckError> {
        let execution = self
            .tool
            .invocation()
            .arg(FORMAT)
            .arg(JSON1)
            .arg(END_OF_OPTIONS)
            .args(scripts.iter().map(FilePath::get))
            .launch_with(&self.output, ShownStreams::StandardError)
            .await
            .map_err(|source| ObserveShellcheckError::ShellcheckUnavailable { source })?;

        if !examined(execution.status().code()) {
            return Err(ObserveShellcheckError::IncompleteExamination {
                details: execution.stderr().to_string_lossy().trim().to_owned(),
            });
        }

        let stdout = execution.stdout().to_string_lossy();

        self::report::comments(&stdout).map_err(|source| ObserveShellcheckError::UnreadableReport {
            report: stdout.to_string(),
            source,
        })
    }

    /// Returns the ShellCheck that mise installed at the version that the
    /// action declares
    ///
    /// The lookup reads the tools that the action declares. The mise file of
    /// the crate pins ShellCheck, so they hold its version. A pin of ShellCheck in
    /// the project does not change the answer. A run of ShellCheck starts in
    /// the root of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when the action declares no ShellCheck, or
    /// when mise does not run. It also returns one when mise reports no
    /// ShellCheck at that version, for example because nothing installed it.
    #[builder]
    pub(crate) async fn resolve<A: Action>(
        context: &Context,
        action: &A,
    ) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(SHELLCHECK)
            .executable(SHELLCHECK)
            .context(context)
            .call()
            .await?;

        Ok(Self {
            tool,
            output: context.tool_output().clone(),
        })
    }
}

/// Returns whether a run examined every script that it was given
///
/// ShellCheck ends with success when it found nothing, and with its own status
/// when it found a problem. Every other status, and a run that no status
/// ended, belongs to a ShellCheck that stopped before it was done.
fn examined(status: Option<i32>) -> bool {
    matches!(status, Some(CLEAN | COMMENTED))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn examined_of_a_clean_run_is_true() {
        let examined = examined(Some(CLEAN));

        assert!(examined);
    }

    #[test]
    fn examined_of_a_run_that_found_a_problem_is_true() {
        let examined = examined(Some(COMMENTED));

        assert!(examined);
    }

    #[test]
    fn examined_of_a_run_that_no_status_ended_is_false() {
        let examined = examined(None);

        assert!(!examined);
    }

    #[test]
    fn examined_of_a_run_with_an_unreadable_script_is_false() {
        let examined = examined(Some(2));

        assert!(!examined);
    }

    #[test]
    fn shellcheck_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Shellcheck>();
    }
}
