//! The tsc that a project runs
//!
//! This module holds the compiler that mise installed for a project and the
//! run that produces its diagnostics. An action asks for a run, and everything
//! between the action and the process lives here.

mod error;
pub(crate) mod report;

use rakko::ProjectRoot;
use rakko_tool::{Launch, ResolveToolError, Tool, ToolName};

pub(crate) use self::error::ObserveTscError;
use crate::diagnostic::Diagnostic;

/// The name that mise knows the tool by
const TSC: &str = "tsc";

/// The option that stops tsc from writing the output of a build
///
/// Tsc is the compiler of the project as well as its type checker. It writes
/// the JavaScript of every file that it reads unless it is told not to. A check
/// that emitted would leave files behind that whoever started it did not ask
/// for. Those files would sit in the directory that the configuration of the
/// project names for them. The option selects the diagnosis and not the build.
const NO_EMIT: &str = "--noEmit";

/// The option that decides how tsc draws its diagnostics
///
/// Tsc draws the source of a diagnostic underneath it with the place marked in
/// color when it writes to a terminal. It writes one line per diagnostic
/// otherwise. So the report of a run would depend on where the run happened.
/// The option settles that, and it selects the presentation of the report and
/// not the behavior of the compiler.
const PRETTY: &str = "--pretty";

/// The value that asks tsc for its diagnostics without the drawing
const PLAIN: &str = "false";

/// The option that names the project that tsc checks
const PROJECT: &str = "-p";

/// The place that a run tells tsc to look
///
/// A run starts in the root of the project, so the working directory is the
/// project, and tsc reports every path relative to it.
const HERE: &str = ".";

/// The tsc that a project runs
///
/// The value holds the compiler that mise installed for the project, at the
/// version that the project pinned. So a run reaches the same program as the
/// editor and the terminal of a contributor. The version decides what the
/// language is. A release of TypeScript adds rules, and a program that one
/// version accepts is a program that the next one can refuse. Nothing here
/// installs a tool: provisioning is the job of mise, and a tsc that mise does
/// not report stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Tsc {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Tsc {
    /// Runs tsc over the project and reads what it reported
    ///
    /// The run names the root of the project, asks for no output, and asks for
    /// the diagnostics without the drawing. Tsc writes its diagnostics on the
    /// standard output stream, whatever they are about.
    ///
    /// The diagnostics are the answer. The status of the process decides only
    /// whether a run without one is trusted. Tsc reports a diagnostic for
    /// everything that it finds. That includes a configuration that it refuses
    /// and a project that it finds no configuration of. So a run that reported
    /// nothing and ended without success stopped for a reason of its own.
    ///
    /// # Errors
    ///
    /// Returns [`TscUnavailable`][unavailable] when tsc does not run. Returns
    /// [`MissingReport`][missing] when tsc reported nothing and ended without
    /// success. Returns [`UnreadableReport`][unreadable] when it wrote a report
    /// that this crate cannot read.
    ///
    /// [missing]: ObserveTscError::MissingReport
    /// [unavailable]: ObserveTscError::TscUnavailable
    /// [unreadable]: ObserveTscError::UnreadableReport
    pub(crate) async fn observe(&self) -> Result<Vec<Diagnostic>, ObserveTscError> {
        let execution = self
            .tool
            .invocation()
            .arg(NO_EMIT)
            .arg(PRETTY)
            .arg(PLAIN)
            .arg(PROJECT)
            .arg(HERE)
            .launch()
            .await
            .map_err(|source| ObserveTscError::TscUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy().into_owned();

        let diagnostics =
            self::report::read(&stdout).map_err(|source| ObserveTscError::UnreadableReport {
                report: stdout.clone(),
                source,
            })?;

        if diagnostics.is_empty() && !execution.status().success() {
            let stderr = execution.stderr().to_string_lossy();

            return Err(ObserveTscError::silent()
                .stdout(&stdout)
                .stderr(&stderr)
                .call());
        }

        Ok(diagnostics)
    }

    /// Returns the tsc that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names. So
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no tsc for the
    /// project.
    pub(crate) async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve()
            .name(ToolName::new(TSC))
            .root(root)
            .call()
            .await?;

        Ok(Self { tool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsc_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Tsc>();
    }
}
