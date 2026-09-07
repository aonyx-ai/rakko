//! The cargo-deny that a project runs
//!
//! This module holds the program that mise installed for a project and the
//! run that checks one workspace with it. An action asks for a run, and
//! everything between the action and the process lives here.

/// The error that leaves a workspace unchecked
mod error;
/// The reading of the report that cargo-deny wrote
mod report;

use rakko_action::ProjectRoot;
use rakko_cargo::CargoRoot;
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::CheckWorkspaceError;
use self::report::Entry;
use crate::problem::DenyProblem;

/// The name that mise knows the tool by
const CARGO_DENY: &str = "cargo-deny";

/// The option that asks cargo-deny for its report as data
///
/// Cargo-deny draws a block per finding by default, with the source of the
/// manifest and the inclusion graph of the package in it. The same run writes
/// the findings as data on request, and each of them then carries the check,
/// the level, the message, and the packages in fields instead of in a block
/// that a reader has to take apart. The format also protects a run from its
/// environment, because the default format changes on a terminal and on a
/// build server. The option selects the presentation of the report and not
/// the behavior of the tool.
const FORMAT: &str = "--format";

/// The name of the format that carries the report as data
const JSON: &str = "json";

/// The option that asks cargo-deny to take every member of the workspace
///
/// Cargo-deny takes the manifest that it starts at as the only root of the
/// graph. A workspace whose root manifest is a package of its own would then
/// contribute that package and nothing else, so a member that no other member
/// depends on would leave the check. The option selects how much of the
/// workspace the run covers, and it changes nothing about what cargo-deny
/// does with what it collected.
const WORKSPACE: &str = "--workspace";

/// The subcommand that examines the dependencies of a workspace
const CHECK: &str = "check";

/// The checks that a run asks for
///
/// The three read the project alone. The fourth check of cargo-deny reads the
/// advisory database of RustSec, which it fetches over the network and keeps
/// outside the project, and which reports a project that stood still as
/// broken on the day that an advisory lands. That check belongs to an action
/// of its own.
const CHECKS: [&str; 3] = ["bans", "licenses", "sources"];

/// The details of a run of cargo-deny that stopped and wrote nothing
const NO_DIAGNOSIS: &str = "cargo-deny wrote nothing about it";

/// The cargo-deny that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// terminal of a contributor. Nothing here installs a tool: provisioning is
/// the job of mise, and a cargo-deny that mise does not report stops the
/// caller.
///
/// Cargo-deny is its own program, and not a subcommand that cargo carries, so
/// a run starts it directly and no cargo stands between the two.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_cargo::CargoRoot;
/// use rakko_check_dependencies::Deny;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
/// let deny = Deny::resolve(root).await?;
///
/// let problems = deny
///     .check(&CargoRoot::new("/home/otter/project".into()))
///     .await?;
///
/// println!("{} problems", problems.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Deny {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Deny {
    /// Runs cargo-deny over one workspace and reads what it reported
    ///
    /// The run starts in the directory of the workspace root, so cargo-deny
    /// reads the manifests of that workspace and looks for its configuration
    /// from that directory upwards. It asks for the bans, the licenses, and
    /// the sources check, for every member of the workspace, and for the
    /// report as data.
    ///
    /// Cargo-deny writes its report on the standard error stream, one record
    /// per line, and it logs what it is doing on the same stream. The reading
    /// keeps the records that say something about the workspace and leaves
    /// the log.
    ///
    /// A problem that the run returns is no failure of the run. The weight
    /// that the configuration of the project gave it travels with it, and the
    /// caller decides what it means for an outcome.
    ///
    /// # Errors
    ///
    /// Returns [`DenyUnavailable`][unavailable] when cargo-deny does not run,
    /// [`IncompleteCheck`][incomplete] when it stopped before it had checked
    /// the workspace, and [`UnreadableReport`][unreadable] when it wrote a
    /// record that this crate cannot read.
    ///
    /// [incomplete]: CheckWorkspaceError::IncompleteCheck
    /// [unavailable]: CheckWorkspaceError::DenyUnavailable
    /// [unreadable]: CheckWorkspaceError::UnreadableReport
    // checkdependencies[impl check.read]
    // checkdependencies[impl roots.members]
    // checkdependencies[impl run.checks]
    // checkdependencies[impl run.directory]
    // checkdependencies[impl run.structured]
    pub async fn check(&self, root: &CargoRoot) -> Result<Vec<DenyProblem>, CheckWorkspaceError> {
        let execution = self
            .tool
            .invocation()
            .in_directory(root.directory().as_path())
            .arg(FORMAT)
            .arg(JSON)
            .arg(WORKSPACE)
            .arg(CHECK)
            .args(CHECKS)
            .run()
            .await
            .map_err(|source| CheckWorkspaceError::DenyUnavailable {
                root: root.directory().clone(),
                source,
            })?;

        let report = execution.stderr().to_string_lossy();

        let mut problems = Vec::new();
        let mut checked = false;

        for record in report.lines().filter(|line| !line.trim().is_empty()) {
            // checkdependencies[impl check.unreadable]
            let entry = self::report::read(record).map_err(|source| {
                CheckWorkspaceError::UnreadableReport {
                    root: root.directory().clone(),
                    record: record.to_owned(),
                    source,
                }
            })?;

            match entry {
                Entry::Problem(problem) => problems.push(problem),
                Entry::Summary => checked = true,
                Entry::Ignored => {}
            }
        }

        // checkdependencies[impl check.configuration]
        // checkdependencies[impl check.incomplete]
        if !checked {
            return Err(CheckWorkspaceError::IncompleteCheck {
                root: root.directory().clone(),
                details: details(&report),
            });
        }

        Ok(problems)
    }

    /// Returns the cargo-deny that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no cargo-deny for the
    /// project.
    // checkdependencies[impl tool.deny]
    // checkdependencies[impl tool.missing]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(CARGO_DENY), root).await?;

        Ok(Self { tool })
    }
}

/// Returns what cargo-deny wrote about a run that it stopped
///
/// The stream carries the report and the log of the run together, and a run
/// that stopped wrote its reason into that log. The whole stream is the
/// answer, because the reason can arrive as one record or as several, and a
/// stream that says nothing gets a sentence that says so.
// checkdependencies[impl check.incomplete]
fn details(report: &str) -> String {
    let details = report.trim();

    if details.is_empty() {
        return NO_DIAGNOSIS.to_owned();
    }

    details.to_owned()
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // checkdependencies[verify check.incomplete]
    #[test]
    fn details_of_a_run_that_wrote_nothing_says_so() {
        let details = details("   \n");

        assert_eq!(details, NO_DIAGNOSIS);
    }

    // checkdependencies[verify check.incomplete]
    #[test]
    fn details_of_a_run_that_stopped_holds_what_cargo_deny_wrote() {
        let details = details("  failed to deserialize config from 'deny.toml'\n");

        assert_eq!(details, "failed to deserialize config from 'deny.toml'");
    }
}
