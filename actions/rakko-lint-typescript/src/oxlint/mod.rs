//! The oxlint that a project runs
//!
//! This module holds the version of oxlint that the crate declares and the
//! program that mise installed at that version. It also holds the run that
//! produces a report. An action asks for a run, and everything between the
//! action and the process lives here.

mod error;
pub(crate) mod report;

use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
use rakko_tool::{Launch, MiseFile, ResolveRequirementError, Tool};

pub(crate) use self::error::ObserveOxlintError;
use crate::observation::Observation;

/// The name that mise knows the tool by, which is also the name of its
/// program
const OXLINT: &str = "oxlint";

/// The version of oxlint that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(OXLINT);

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
}

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
            .launch()
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

    /// Returns the requirement of the oxlint that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(OXLINT)
            .executable(OXLINT)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the oxlint that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of oxlint in the project does not change the answer.
    /// A run of oxlint starts in the root of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, or when
    /// it reports no oxlint at that version, for example because nothing
    /// installed it.
    ///
    /// [requirement]: Oxlint::requirement
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
    fn oxlint_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Oxlint>();
    }
}
