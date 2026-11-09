//! The zizmor that a project runs
//!
//! This module holds the version that the crate declares, the program that mise
//! installed at that version, and the run that produces a report. An action
//! asks for a run, and everything between the action and the process lives
//! here.

mod error;
mod report;

use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
use rakko_tool::{Launch, MiseFile, ResolveRequirementError, Tool};

pub(crate) use self::error::ObserveZizmorError;
use crate::observation::{Examination, Observation};

/// The name that mise knows the tool by, which is also the name of its
/// program
const ZIZMOR: &str = "zizmor";

/// The version of zizmor that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(ZIZMOR);

/// The option that asks zizmor for its report as data
///
/// Zizmor writes a block of source per finding by default, with the audit above
/// it and the annotations of its places drawn into it. The same run writes the
/// findings as data on request. Each finding then carries the audit, the
/// severity, the place, and the annotation in fields instead of in a block. A
/// reader would have to take that block apart. The format also protects a run
/// from its environment, because the default format changes on a terminal and
/// on a build server. The option selects the presentation of the report and not
/// the behavior of the tool.
const FORMAT: &str = "--format";

/// The name of the format that carries the findings as data
const JSON: &str = "json";

/// The option that asks zizmor how much of what it sees to report
///
/// Zizmor takes the persona on its command line alone. Its configuration file
/// has no key for one. So a run that names no persona gets the regular persona,
/// and the project has no way to ask for more.
const PERSONA: &str = "--persona";

/// The persona that reports the code smells of a workflow as well
///
/// The regular persona reports what zizmor is confident about, and this one
/// adds what a reviewer of a workflow wants to see. It is the persona that
/// this fleet audits with.
const PEDANTIC: &str = "pedantic";

/// The option that asks zizmor to stop at a file that it cannot read
///
/// Zizmor warns about such a file by default, drops it, and audits the rest. A
/// workflow with a syntax error or a key that GitHub does not define therefore
/// leaves the audit through a warning that no outcome carries. An audit worth
/// running is worth running over every file that it collected.
const STRICT_COLLECTION: &str = "--strict-collection";

/// The place that a run tells zizmor to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and zizmor reports every path relative to it.
const HERE: &str = ".";

/// The status of a zizmor that audited the project and found nothing
const CLEAN: i32 = 0;

/// The status of a zizmor that collected no file to audit
const NOTHING_COLLECTED: i32 = 3;

/// The lowest status of a zizmor that audited the project and found something
///
/// Zizmor separates the findings of a project from its own failures. When the
/// run has a finding, it exits with a status from this range. The status
/// names the highest severity that the run found. Every other status belongs
/// to a run that stopped before it was done.
const LOWEST_FINDING: i32 = 11;

/// The highest status of a zizmor that audited the project and found something
const HIGHEST_FINDING: i32 = 14;

/// The zizmor that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it. So a run reaches
/// the version that the tests of the crate cover. Nothing here installs a tool:
/// provisioning is the job of mise, and a zizmor that mise does not report
/// stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Zizmor {
    /// The program that mise installed at the declared version
    tool: Tool,
}

impl Zizmor {
    /// Runs zizmor over the project and reads what it reported
    ///
    /// The run names the root of the project and asks for the pedantic persona.
    /// It asks zizmor to stop at a file that it cannot read and to give the
    /// report as data. Zizmor writes the report on its standard output stream.
    /// It uses the standard error stream for its log and for the reason of a
    /// run that it stopped.
    ///
    /// A run that collected no file to audit is no failure. The observation
    /// reports it, because zizmor writes the same empty report for a project
    /// that it audited and found clean.
    ///
    /// # Errors
    ///
    /// Returns [`ZizmorUnavailable`][unavailable] when zizmor does not run,
    /// Returns [`IncompleteAudit`][incomplete] when zizmor stopped before it
    /// had audited the project. Returns [`UnreadableReport`][unreadable] when
    /// it wrote a report that this crate cannot read.
    ///
    /// [incomplete]: ObserveZizmorError::IncompleteAudit
    /// [unavailable]: ObserveZizmorError::ZizmorUnavailable
    /// [unreadable]: ObserveZizmorError::UnreadableReport
    pub(crate) async fn observe(&self) -> Result<Observation, ObserveZizmorError> {
        let execution = self
            .tool
            .invocation()
            .arg(PERSONA)
            .arg(PEDANTIC)
            .arg(STRICT_COLLECTION)
            .arg(FORMAT)
            .arg(JSON)
            .arg(HERE)
            .launch()
            .await
            .map_err(|source| ObserveZizmorError::ZizmorUnavailable { source })?;

        let Some(status) = audited(execution.status().code()) else {
            return Err(ObserveZizmorError::IncompleteAudit {
                details: execution.stderr().to_string_lossy().trim().to_owned(),
            });
        };

        match status {
            Collection::Empty => {
                return Ok(Observation::builder()
                    .collected(Examination::Skipped)
                    .build());
            }
            Collection::Audited => {}
        }

        let stdout = execution.stdout().to_string_lossy();

        let problems = self::report::problems(&stdout).map_err(|source| {
            ObserveZizmorError::UnreadableReport {
                report: stdout.to_string(),
                source,
            }
        })?;

        Ok(Observation::builder().problems(problems).build())
    }

    /// Returns the requirement of the zizmor that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(ZIZMOR)
            .executable(ZIZMOR)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the zizmor that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of zizmor in the project does not change the answer.
    /// A run of zizmor starts in the root of the context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run. It also
    /// returns one when mise reports no zizmor at that version, for example
    /// because nothing installed it.
    ///
    /// [requirement]: Zizmor::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
            .context(context)
            .call()
            .await?;

        Ok(Self { tool })
    }
}

/// What a run of zizmor found to audit
///
/// Zizmor reports an empty array for a project that it audited and found
/// clean, and for a project where it collected no file at all. Its exit status
/// is what separates the two.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum Collection {
    /// Zizmor collected no file to audit
    Empty,

    /// Zizmor collected files and audited them
    Audited,
}

/// Returns what a run found to audit, or `None` for a run that stopped
///
/// Zizmor ends with success when it audited the project and found nothing. It
/// ends with one of the statuses of the finding range when it has something to
/// report. It ends with its own status when it collected no file. Every other
/// status, and a run that no status ended, belongs to a zizmor that stopped
/// before it had audited the project.
fn audited(status: Option<i32>) -> Option<Collection> {
    match status? {
        CLEAN => Some(Collection::Audited),
        NOTHING_COLLECTED => Some(Collection::Empty),
        code if (LOWEST_FINDING..=HIGHEST_FINDING).contains(&code) => Some(Collection::Audited),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn audited_of_a_clean_run_reports_an_audit() {
        let collection = audited(Some(CLEAN));

        assert_eq!(collection, Some(Collection::Audited));
    }

    #[test]
    fn audited_of_a_run_that_no_status_ended_reports_nothing() {
        let collection = audited(None);

        assert_eq!(collection, None);
    }

    #[test]
    fn audited_of_a_run_that_reported_a_finding_reports_an_audit() {
        let collection = audited(Some(HIGHEST_FINDING));

        assert_eq!(collection, Some(Collection::Audited));
    }

    #[test]
    fn audited_of_a_run_that_stopped_reports_nothing() {
        let collection = audited(Some(1));

        assert_eq!(collection, None);
    }

    #[test]
    fn audited_of_a_run_without_an_input_reports_an_empty_collection() {
        let collection = audited(Some(NOTHING_COLLECTED));

        assert_eq!(collection, Some(Collection::Empty));
    }

    #[test]
    fn collection_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Collection>();
    }

    #[test]
    fn zizmor_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Zizmor>();
    }
}
