//! The zizmor that a project runs
//!
//! This module holds the program that mise installed for a project, the look
//! that tells whether zizmor has anything to do there, and the run that
//! produces a report. An action asks for a run, and everything between the
//! action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that zizmor wrote
mod report;

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;

use rakko_action::ProjectRoot;
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::ObserveZizmorError;
use crate::observation::Observation;

/// The name that mise knows the tool by
const ZIZMOR: &str = "zizmor";

/// The option that asks zizmor for its report as data
///
/// Zizmor writes a block of source per finding by default, with the audit
/// above it and the annotations of its places drawn into it. The same run
/// writes the findings as data on request, and each of them then carries the
/// audit, the severity, the place, and the annotation in fields instead of in
/// a block that a reader has to take apart. The format also protects a run
/// from its environment, because the default format changes on a terminal and
/// on a build server. The option selects the presentation of the report and
/// not the behavior of the tool.
const FORMAT: &str = "--format";

/// The name of the format that carries the findings as data
const JSON: &str = "json";

/// The option that asks zizmor how much of what it sees to report
///
/// Zizmor takes the persona on its command line alone, and its configuration
/// file has no key for one, so a run that names no persona gets the regular
/// persona and the project has no way to ask for more.
const PERSONA: &str = "--persona";

/// The persona that reports the code smells of a workflow as well
///
/// The regular persona reports what zizmor is confident about, and this one
/// adds what a reviewer of a workflow wants to see. It is the persona that
/// this fleet audits with.
const PEDANTIC: &str = "pedantic";

/// The option that asks zizmor to stop at a file that it cannot read
///
/// Zizmor warns about such a file by default, drops it, and audits the rest,
/// so a workflow with a syntax error or with a key that GitHub does not define
/// leaves the audit through a warning that no outcome carries. An audit that
/// is worth running is worth running over every file that it collected.
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
/// Zizmor separates the findings of a project from its own failures. It ends
/// with one of the statuses of this range when it has a finding to report,
/// and the status names the highest severity that the run found. Every other
/// status belongs to a run that stopped before it was done.
const LOWEST_FINDING: i32 = 11;

/// The highest status of a zizmor that audited the project and found something
const HIGHEST_FINDING: i32 = 14;

/// The directory that a project keeps its GitHub Actions workflows in
const WORKFLOWS: [&str; 2] = [".github", "workflows"];

/// The extensions of the workflow files that GitHub reads
const WORKFLOW_EXTENSIONS: [&str; 2] = ["yaml", "yml"];

/// The zizmor that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a zizmor that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_lint_github_actions::Zizmor;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// if Zizmor::applies(&root).await {
///     let zizmor = Zizmor::resolve(root).await?;
///     let observation = zizmor.observe().await?;
///
///     println!("{} problems", observation.problems().len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Zizmor {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Zizmor {
    /// Returns whether the project holds a GitHub Actions workflow
    ///
    /// The look reads one directory: `.github/workflows` below the root of
    /// the project. GitHub reads a workflow in that directory and nowhere
    /// else, so a project with a workflow has one there. An entry counts when
    /// it is a file with the `.yaml` or the `.yml` extension, which are the
    /// two that GitHub reads.
    ///
    /// The look follows no symbolic link, so a project whose workflows sit
    /// only behind a link reports no workflow here. A directory that the look
    /// cannot read counts as holding workflows: a look that cannot prove
    /// absence must not hide a real check behind a skip, and zizmor reports
    /// its own failure when a run reaches it.
    ///
    /// The look answers narrower than zizmor, which collects an action
    /// definition, a Dependabot configuration, and the files of pre-commit as
    /// well. A project whose only auditable file is one of those holds no
    /// GitHub Actions workflows, which is what this action is named for.
    // lintgithubactions[impl skip.links]
    // lintgithubactions[impl skip.missing]
    pub async fn applies(root: &ProjectRoot) -> bool {
        let directory = WORKFLOWS
            .iter()
            .fold(root.get().to_path_buf(), |path, segment| path.join(segment));

        let mut entries = match tokio::fs::read_dir(&directory).await {
            Ok(entries) => entries,
            Err(error) => return error.kind() != ErrorKind::NotFound,
        };

        loop {
            match entries.next_entry().await {
                Ok(Some(entry)) => {
                    let Ok(kind) = entry.file_type().await else {
                        return true;
                    };

                    if kind.is_file() && workflow(&entry.path()) {
                        return true;
                    }
                }
                Ok(None) => return false,
                Err(_) => return true,
            }
        }
    }

    /// Runs zizmor over the project and reads what it reported
    ///
    /// The run names the root of the project, asks for the pedantic persona,
    /// asks zizmor to stop at a file that it cannot read, and asks for the
    /// report as data. Zizmor writes the report on its standard output stream,
    /// and it uses the standard error stream for its log and for the reason of
    /// a run that it stopped.
    ///
    /// A run that collected no file to audit is no failure. The observation
    /// reports it, because zizmor writes the same empty report for a project
    /// that it audited and found clean.
    ///
    /// # Errors
    ///
    /// Returns [`ZizmorUnavailable`][unavailable] when zizmor does not run,
    /// [`IncompleteAudit`][incomplete] when zizmor stopped before it had
    /// audited the project, and [`UnreadableReport`][unreadable] when it wrote
    /// a report that this crate cannot read.
    ///
    /// [incomplete]: ObserveZizmorError::IncompleteAudit
    /// [unavailable]: ObserveZizmorError::ZizmorUnavailable
    /// [unreadable]: ObserveZizmorError::UnreadableReport
    // lintgithubactions[impl check.read]
    // lintgithubactions[impl run.persona]
    // lintgithubactions[impl run.project]
    // lintgithubactions[impl run.strict]
    // lintgithubactions[impl run.structured]
    pub async fn observe(&self) -> Result<Observation, ObserveZizmorError> {
        let execution = self
            .tool
            .invocation()
            .arg(PERSONA)
            .arg(PEDANTIC)
            .arg(STRICT_COLLECTION)
            .arg(FORMAT)
            .arg(JSON)
            .arg(HERE)
            .run()
            .await
            .map_err(|source| ObserveZizmorError::ZizmorUnavailable { source })?;

        // lintgithubactions[impl check.configuration]
        // lintgithubactions[impl check.incomplete]
        let Some(status) = audited(execution.status().code()) else {
            return Err(ObserveZizmorError::IncompleteAudit {
                details: execution.stderr().to_string_lossy().trim().to_owned(),
            });
        };

        // lintgithubactions[impl skip.uncollected]
        if status == Collection::Empty {
            return Ok(Observation::builder().collected(false).build());
        }

        let stdout = execution.stdout().to_string_lossy();

        // lintgithubactions[impl check.unreadable]
        let problems = self::report::problems(&stdout).map_err(|source| {
            ObserveZizmorError::UnreadableReport {
                report: stdout.to_string(),
                source,
            }
        })?;

        Ok(Observation::builder().problems(problems).build())
    }

    /// Returns the zizmor that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no zizmor for the
    /// project.
    // lintgithubactions[impl tool.missing]
    // lintgithubactions[impl tool.zizmor]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(ZIZMOR), root).await?;

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
/// Zizmor ends with success when it audited the project and found nothing,
/// and with one of the statuses of the finding range when it has something to
/// report. It ends with its own status when it collected no file. Every other
/// status, and a run that no status ended, belongs to a zizmor that stopped
/// before it had audited the project.
// lintgithubactions[impl check.incomplete]
fn audited(status: Option<i32>) -> Option<Collection> {
    match status? {
        CLEAN => Some(Collection::Audited),
        NOTHING_COLLECTED => Some(Collection::Empty),
        code if (LOWEST_FINDING..=HIGHEST_FINDING).contains(&code) => Some(Collection::Audited),
        _ => None,
    }
}

/// Returns whether GitHub would read the file as a workflow
///
/// GitHub reads a file of the workflow directory when it carries one of the
/// two YAML extensions. The comparison ignores case, which is what a file
/// system that ignores case does, and is the wider answer everywhere else. A
/// look that answers for the wider set never hides a check behind a skip.
// lintgithubactions[impl skip.missing]
fn workflow(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };

    WORKFLOW_EXTENSIONS
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    // lintgithubactions[verify check.incomplete]
    #[test]
    fn audited_of_a_clean_run_reports_an_audit() {
        let collection = audited(Some(CLEAN));

        assert_eq!(collection, Some(Collection::Audited));
    }

    // lintgithubactions[verify check.incomplete]
    #[test]
    fn audited_of_a_run_that_no_status_ended_reports_nothing() {
        let collection = audited(None);

        assert_eq!(collection, None);
    }

    // lintgithubactions[verify check.incomplete]
    #[test]
    fn audited_of_a_run_that_reported_a_finding_reports_an_audit() {
        let collection = audited(Some(HIGHEST_FINDING));

        assert_eq!(collection, Some(Collection::Audited));
    }

    // lintgithubactions[verify check.incomplete]
    #[test]
    fn audited_of_a_run_that_stopped_reports_nothing() {
        let collection = audited(Some(1));

        assert_eq!(collection, None);
    }

    // lintgithubactions[verify skip.uncollected]
    #[test]
    fn audited_of_a_run_without_an_input_reports_an_empty_collection() {
        let collection = audited(Some(NOTHING_COLLECTED));

        assert_eq!(collection, Some(Collection::Empty));
    }

    // lintgithubactions[verify skip.missing]
    #[test]
    fn workflow_of_a_file_of_another_extension_reports_no_match() {
        let matched = workflow(&PathBuf::from(".github/workflows/notes.txt"));

        assert!(!matched);
    }

    // lintgithubactions[verify skip.missing]
    #[test]
    fn workflow_of_a_file_without_an_extension_reports_no_match() {
        let matched = workflow(&PathBuf::from(".github/workflows/README"));

        assert!(!matched);
    }

    // lintgithubactions[verify skip.missing]
    #[test]
    fn workflow_of_a_long_extension_reports_a_match() {
        let matched = workflow(&PathBuf::from(".github/workflows/ci.yaml"));

        assert!(matched);
    }

    // lintgithubactions[verify skip.missing]
    #[test]
    fn workflow_of_a_short_extension_reports_a_match() {
        let matched = workflow(&PathBuf::from(".github/workflows/ci.yml"));

        assert!(matched);
    }
}
