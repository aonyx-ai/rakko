//! The oxlint that a project runs
//!
//! This module holds the program that mise installed at the version of oxlint
//! that the action declares. It also holds the run that
//! produces a report. An action asks for a run, and everything between the
//! action and the process lives here.

mod error;
pub(crate) mod report;

use rakko::{Action, Context, ToolOutput};
use rakko_tool::{Launch, ResolveToolError, ShownStreams, Tool};

pub(crate) use self::error::ObserveOxlintError;
use crate::observation::Observation;

/// The name that mise knows the tool by, which is also the name of its
/// program
const OXLINT: &str = "oxlint";

/// The option that asks oxlint for its report as data
///
/// Oxlint draws a block of source per diagnostic by default, with the rule
/// above it and the places that it marks drawn into it. The same run writes the
/// diagnostics as one JSON object on request. Each diagnostic then carries the
/// severity and the message in fields instead of in a block that a reader has
/// to take apart. It carries the rule, the file, and the places in fields as
/// well, where oxlint names them. The object also counts the files that the run
/// examined, which no other format reports. The format protects a run from its
/// environment, because the default format changes on a terminal and on a build
/// server. The option selects the presentation of the report and not the
/// behavior of the tool.
const FORMAT: &str = "--format";

/// The name of the format that carries the diagnostics as data
const JSON: &str = "json";

/// The place that a run tells oxlint to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and oxlint reports every path relative to it.
const HERE: &str = ".";

/// The oxlint that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it, so a run reaches
/// the version that the tests of the crate cover. Nothing here installs a tool.
/// Provisioning is the job of mise. An oxlint that mise does not report
/// stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Oxlint {
    /// The program that mise installed at the declared version
    tool: Tool,

    /// The tool output of the context that resolved oxlint
    ///
    /// Each run sends what oxlint writes to it.
    output: ToolOutput,
}

#[bon::bon]
impl Oxlint {
    /// Runs oxlint over the project and reads what it reported
    ///
    /// The run names the root of the project and asks for the report as data.
    /// Oxlint writes the report on its standard output stream. It writes
    /// the diagnosis of a run that it could not start on the same stream.
    ///
    /// The status of the process answers nothing here, and the report answers
    /// everything. Oxlint ends with success for a project whose broken rules
    /// the project weighs as warnings. It ends without success for a project
    /// that holds no file to lint. So a caller that read the status would pass
    /// a project with problems and fail a project with none.
    ///
    /// A run that examined no file is no failure. The observation reports it
    /// through the count of the files. Oxlint writes the same empty
    /// list of diagnostics for a project that it linted and found clean.
    ///
    /// # Errors
    ///
    /// Returns [`OxlintUnavailable`][unavailable] when oxlint does not run,
    /// [`MissingReport`][missing] when oxlint wrote no report, and
    /// [`UnreadableReport`][unreadable] when it wrote a report that this crate
    /// cannot read.
    ///
    /// [missing]: ObserveOxlintError::MissingReport
    /// [unavailable]: ObserveOxlintError::OxlintUnavailable
    /// [unreadable]: ObserveOxlintError::UnreadableReport
    pub(crate) async fn observe(&self) -> Result<Observation, ObserveOxlintError> {
        let execution = self
            .tool
            .invocation()
            .arg(FORMAT)
            .arg(JSON)
            .arg(HERE)
            .launch_with(&self.output, ShownStreams::StandardError)
            .await
            .map_err(|source| ObserveOxlintError::OxlintUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy();

        self::report::read(&stdout).map_err(|source| {
            ObserveOxlintError::of()
                .source(source)
                .stdout(&stdout)
                .stderr(&execution.stderr().to_string_lossy())
                .call()
        })
    }

    /// Returns the oxlint that mise installed at the version that the action
    /// declares
    ///
    /// The lookup reads the tools that the action declares. The mise file of
    /// the crate pins oxlint, so they hold its version. A pin of oxlint in the
    /// project does not change the answer. A run of oxlint starts in the root
    /// of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when the action declares no oxlint, or
    /// when mise does not run. It also returns one when mise reports no
    /// oxlint at that version, for example because nothing installed it.
    #[builder]
    pub(crate) async fn resolve<A: Action>(
        context: &Context,
        action: &A,
    ) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(OXLINT)
            .executable(OXLINT)
            .context(context)
            .call()
            .await?;

        Ok(Self {
            tool,
            output: context.tool_output().clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oxlint_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Oxlint>();
    }
}
