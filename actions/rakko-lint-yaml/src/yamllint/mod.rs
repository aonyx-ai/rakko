//! The yamllint that a project runs
//!
//! This module holds the version that the crate declares, the program that mise
//! installed at that version, and the runs that produce a listing and a report.
//! An action asks for a run, and everything between the action and the process
//! lives here.

mod error;
pub(crate) mod listing;
pub(crate) mod report;

use std::path::PathBuf;

use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
use rakko_tool::{Launch, MiseFile, ResolveRequirementError, Tool};

pub(crate) use self::error::ObserveYamllintError;
pub(crate) use self::listing::ListFilesError;
use crate::observation::{Completion, Observation};

/// The name that mise knows the tool by, which is also the name of its
/// program
const YAMLLINT: &str = "yamllint";

/// The version of yamllint that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(YAMLLINT);

/// The option that asks yamllint for its report in the parsable format
///
/// Yamllint groups its findings under a heading per file by default. It
/// writes one self-contained line per problem in this format. The line carries
/// the file, the position, the level, and the description in fields. A block
/// would make a reader take them apart. The format also protects a
/// run from its environment, because the default format changes on a terminal
/// and on a build server. The option selects how the report is shown, not how
/// the tool behaves. Which rules apply to which file comes from the project
/// configuration alone.
const FORMAT: &str = "--format";

/// The name of the format that carries one problem per line
const PARSABLE: &str = "parsable";

/// The place that a run tells yamllint to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and yamllint reports every path relative to it.
const HERE: &str = ".";

/// The status of a yamllint that examined every file and found a problem
///
/// Yamllint separates the problems of a project from its own failures. It
/// ends this way when a rule of an error level was broken. It ends with
/// success when nothing was broken and when the problems were warnings alone.
/// Every other status belongs to a run that stopped before it was done.
const PROBLEMS_FOUND: i32 = 1;

/// The yamllint that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it. So a run reaches
/// the version that the tests of the crate cover. Nothing here installs a tool:
/// provisioning is the job of mise, and a yamllint that mise does not report
/// stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Yamllint {
    /// The program that mise installed at the declared version
    tool: Tool,
}

impl Yamllint {
    /// Returns the files that yamllint examines in the project
    ///
    /// The listing reads the configuration of the project, so it answers with
    /// the selection that a lint run of the same project examines.
    ///
    /// # Errors
    ///
    /// Returns [`YamllintUnavailable`][unavailable] when yamllint does not
    /// run, and [`RejectedConfiguration`][rejected] when yamllint refuses the
    /// configuration of the project.
    ///
    /// [unavailable]: ListFilesError::YamllintUnavailable
    /// [rejected]: ListFilesError::RejectedConfiguration
    pub(crate) async fn list(&self) -> Result<Vec<PathBuf>, ListFilesError> {
        self::listing::files(&self.tool).await
    }

    /// Runs yamllint over the project and reads what it reported
    ///
    /// The run names the root of the project and asks for the report in the
    /// parsable format. Yamllint writes the report on its standard output
    /// stream. It uses the standard error stream for the reason of a run
    /// that it could not finish.
    ///
    /// # Errors
    ///
    /// Returns [`YamllintUnavailable`][unavailable] when yamllint does not
    /// run, and [`UnreadableReport`][unreadable] when it wrote a line that
    /// reports no problem.
    ///
    /// [unavailable]: ObserveYamllintError::YamllintUnavailable
    /// [unreadable]: ObserveYamllintError::UnreadableReport
    pub(crate) async fn observe(&self) -> Result<Observation, ObserveYamllintError> {
        let execution = self
            .tool
            .invocation()
            .arg(FORMAT)
            .arg(PARSABLE)
            .arg(HERE)
            .launch()
            .await
            .map_err(|source| ObserveYamllintError::YamllintUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy();
        let stderr = execution.stderr().to_string_lossy();

        let problems = self::report::problems(&stdout).map_err(|source| {
            ObserveYamllintError::UnreadableReport {
                report: stdout.to_string(),
                source,
            }
        })?;

        let status = execution.status();

        Ok(Observation::builder()
            .problems(problems)
            .finished(
                if status.success() || status.code() == Some(PROBLEMS_FOUND) {
                    Completion::Finished
                } else {
                    Completion::Interrupted
                },
            )
            .stderr(stderr.trim().to_owned())
            .build())
    }

    /// Returns the requirement of the yamllint that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(YAMLLINT)
            .executable(YAMLLINT)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the yamllint that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of yamllint in the project does not change the answer.
    /// A run of yamllint starts in the root of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run. It also
    /// returns one when mise reports no yamllint at that version, for example
    /// because nothing installed it.
    ///
    /// [requirement]: Yamllint::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
            .context(context)
            .call()
            .await?;

        Ok(Self { tool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yamllint_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Yamllint>();
    }
}
