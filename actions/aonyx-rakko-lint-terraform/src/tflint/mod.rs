//! The TFLint that a project runs
//!
//! This module holds the version that the crate declares and the program that
//! mise installed at that version. It also holds the run that produces a
//! report. An action asks for a run, and everything between the action and
//! the process lives here.

mod error;
mod report;

use rakko::{Context, RequiredVersion, ToolOutput, ToolRequirement, ToolVersion};
use rakko_tool::{Launch, MiseFile, ResolveRequirementError, ShownStreams, Tool};

pub(crate) use self::error::ObserveTflintError;
use crate::issue::TflintIssue;

/// The name that mise knows the tool by, which is also the name of its
/// program
const TFLINT: &str = "tflint";

/// The version of TFLint that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(TFLINT);

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

    /// Returns the requirement of the TFLint that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(TFLINT)
            .executable(TFLINT)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the TFLint that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of TFLint in the project does not change the answer.
    /// A run of TFLint starts in the root of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run. It also
    /// returns one when mise reports no TFLint at that version, for example
    /// because nothing installed it.
    ///
    /// [requirement]: Tflint::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
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
