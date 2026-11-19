//! The TFLint that a project runs
//!
//! This module holds the program that mise installed at the version that the
//! action declares. It also holds the run that produces a
//! report. An action asks for a run, and everything between the action and
//! the process lives here.

mod error;
mod report;

use rakko::{Action, Context, ToolOutput};
use rakko_tool::{Launch, ResolveToolError, ShownStreams, Tool};

pub(crate) use self::error::ObserveTflintError;
use crate::issue::TflintIssue;

/// The name that mise knows the tool by, which is also the name of its
/// program
const TFLINT: &str = "tflint";

/// The option that asks TFLint for its report as data
///
/// The option selects how the report is shown, not how the tool behaves.
/// Which rules apply comes from the configuration of the project alone.
const FORMAT: &str = "--format=json";

/// The option that keeps the sequences of color out of the report
const NO_COLOR: &str = "--no-color";

/// The option that asks TFLint to inspect every module below the root
///
/// TFLint inspects the module in its working directory by default. It skips a
/// directory whose name starts with a dot during the walk, such as the
/// `.terraform` directory of a module.
const RECURSIVE: &str = "--recursive";

/// The TFLint that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it. So a run reaches
/// the version that the tests of the crate cover. Nothing here installs a tool:
/// provisioning is the job of mise, and a TFLint that mise does not report
/// stops the caller.
///
/// TFLint reads the `.tflint.hcl` file of the project. A plugin that the file
/// enables must be installed with `tflint --init` before a run, because the
/// crate installs nothing.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Tflint {
    /// The program that mise installed at the declared version
    tool: Tool,

    /// The tool output of the context that resolved TFLint
    ///
    /// Each run sends what TFLint writes to it.
    output: ToolOutput,
}

#[bon::bon]
impl Tflint {
    /// Runs TFLint over the project and returns the issues that it reported
    ///
    /// A run starts in the project root and inspects every module below it.
    /// TFLint writes the report on its standard output stream.
    ///
    /// # Errors
    ///
    /// Returns [`TflintUnavailable`][unavailable] when TFLint does not run,
    /// and [`IncompleteInspection`][incomplete] when it could not inspect a
    /// module. Returns [`UnrecognizedReport`][unrecognized] when the crate
    /// cannot read the report, or when the report does not explain a run
    /// that failed.
    ///
    /// [incomplete]: ObserveTflintError::IncompleteInspection
    /// [unavailable]: ObserveTflintError::TflintUnavailable
    /// [unrecognized]: ObserveTflintError::UnrecognizedReport
    pub(crate) async fn observe(&self) -> Result<Vec<TflintIssue>, ObserveTflintError> {
        let execution = self
            .tool
            .invocation()
            .arg(FORMAT)
            .arg(NO_COLOR)
            .arg(RECURSIVE)
            .launch_with(&self.output, ShownStreams::StandardError)
            .await
            .map_err(|source| ObserveTflintError::TflintUnavailable { source })?;

        let unrecognized = || ObserveTflintError::UnrecognizedReport {
            stderr: execution.stderr().to_string_lossy().trim().to_owned(),
        };

        let report = self::report::read(execution.stdout().get()).map_err(|_| unrecognized())?;

        let errors = report.errors();
        if !errors.is_empty() {
            return Err(ObserveTflintError::IncompleteInspection {
                messages: errors.join("\n\n"),
            });
        }

        let issues = report.issues();
        if issues.is_empty() && !execution.status().success() {
            return Err(unrecognized());
        }

        Ok(issues)
    }

    /// Returns the TFLint that mise installed at the version that the action
    /// declares
    ///
    /// The lookup reads the tools that the action declares. The mise file of
    /// the crate pins TFLint, so they hold its version. A pin of TFLint in the
    /// project does not change the answer. A run of TFLint starts in the root
    /// of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when the action declares no TFLint, or
    /// when mise does not run. It also returns one when mise reports no
    /// TFLint at that version, for example because nothing installed it.
    #[builder]
    pub(crate) async fn resolve<A: Action>(
        context: &Context,
        action: &A,
    ) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(TFLINT)
            .executable(TFLINT)
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
    #[test]
    fn tflint_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<super::Tflint>();
    }
}
