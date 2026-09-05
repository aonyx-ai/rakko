//! The action that examines the dependencies of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps cargo-deny as a subprocess: cargo-deny reads the manifests and the
//! lock file of a workspace, reads its own configuration, and applies the
//! checks that the project asked for, and the action translates what
//! cargo-deny reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, DirectoryPath, Finding, Location, Name, Outcome, ProjectRoot, SkipReason,
    Summary, action_name,
};
use rakko_cargo::{Cargo, CargoRoot};

pub use self::error::CheckDependenciesError;
use crate::deny::Deny;
use crate::problem::{DenyProblem, Severity};

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

/// The word that a summary counts one workspace with
const WORKSPACE: &str = "workspace";

/// The word that a summary counts several workspaces with
const WORKSPACES: &str = "workspaces";

/// The word that a summary counts one warning with
const WARNING: &str = "warning";

/// The word that a summary counts several warnings with
const WARNINGS: &str = "warnings";

/// The action that examines the dependencies of a project
///
/// The action wraps [cargo-deny]: cargo-deny reads the manifests and the lock
/// file of a workspace, reads its own configuration, and answers whether every
/// package carries a license that the project accepts, whether the packages
/// come from a registry that the project trusts, and whether the graph holds a
/// shape that the project banned. The cargo-deny that runs is the one that
/// [mise] installed for the project, at the version that the project pinned,
/// and the action installs nothing.
///
/// A run only reports, and it takes no argument. It checks every workspace of
/// the project, because the harness of a project is a package of its own, and
/// it asks cargo for those workspaces.
///
/// Cargo-deny weighs everything that it reports with the level that the
/// configuration of the project gave the check. An error is a shape that the
/// project said must not appear, and a run that reports one fails. A warning
/// is a shape that the project asked to read about, and a run that reports one
/// passes and says how many warnings it read. The action adds no weight of its
/// own.
///
/// A finding names the workspace that the error came from and no path,
/// because a report of cargo-deny names no file that the project holds: the
/// place that it underlines is a line of a lock file, or of a manifest in the
/// registry cache of the machine.
///
/// The action applies to a project that holds a manifest of cargo, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// tool, when the workspaces of the project cannot be discovered, when
/// cargo-deny stops before it has checked a workspace, and when it writes a
/// record that the action cannot read.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_check_dependencies::CheckDependencies;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckDependencies)];
/// ```
///
/// [cargo-deny]: https://embarkstudios.github.io/cargo-deny/
/// [mise]: https://mise.jdx.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckDependencies;

impl Action for CheckDependencies {
    // checkdependencies[impl args.none]
    type Args = ();

    // checkdependencies[impl name]
    fn name(&self) -> Name {
        action_name!("check-dependencies")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // checkdependencies[impl roots.error]
            // checkdependencies[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves the two tools, discovers the
/// workspaces, and checks each of them. An error that this function returns
/// stops the run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of a
/// tool, the discovery of the workspaces, a cargo-deny run, or the reading of
/// a report.
async fn drive(context: &Context) -> Result<Outcome, CheckDependenciesError> {
    // checkdependencies[impl skip.git]
    // checkdependencies[impl skip.links]
    // checkdependencies[impl skip.missing]
    // checkdependencies[impl skip.target]
    if !Cargo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MANIFEST),
        });
    }

    // checkdependencies[impl tool.cargo]
    // checkdependencies[impl tool.missing]
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| CheckDependenciesError::UnresolvedCargo { source })?;

    // checkdependencies[impl tool.deny]
    // checkdependencies[impl tool.missing]
    let deny = Deny::resolve(context.root().clone())
        .await
        .map_err(|source| CheckDependenciesError::UnresolvedDeny { source })?;

    // checkdependencies[impl roots.error]
    let roots = cargo
        .roots()
        .await
        .map_err(|source| CheckDependenciesError::UndiscoveredRoots { source })?;

    let mut findings = Vec::new();
    let mut warnings = 0_usize;

    // checkdependencies[impl roots.all]
    for root in &roots {
        for problem in deny.check(root).await? {
            if problem.denied() {
                findings.push(finding(&problem, root, context.root())?);
            } else if problem.severity() == Severity::Warning {
                warnings = warnings.saturating_add(1);
            }
        }
    }

    if findings.is_empty() {
        // checkdependencies[impl check.passed]
        // checkdependencies[impl check.warning]
        Ok(Outcome::Passed {
            summary: Some(summary(roots.len(), warnings)),
        })
    } else {
        // checkdependencies[impl check.failed]
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Returns the finding that reports one error of cargo-deny
///
/// The finding names the workspace that the error came from, because that is
/// the most that the run knows about its place. The message comes from
/// cargo-deny, so a reader of a finding reads what the tool itself would have
/// told them.
///
/// # Errors
///
/// Returns [`ForeignWorkspace`][foreign] when the project root does not
/// contain the workspace root.
///
/// [foreign]: CheckDependenciesError::ForeignWorkspace
// checkdependencies[impl check.finding]
fn finding(
    problem: &DenyProblem,
    root: &CargoRoot,
    project: &ProjectRoot,
) -> Result<Finding, CheckDependenciesError> {
    Ok(Finding::builder()
        .message(problem.description())
        .location(location(root, project)?)
        .build())
}

/// Returns the place that a finding of a workspace belongs to
///
/// A workspace below the project root is a directory of the project, and the
/// workspace at the project root is the project itself. The directory of the
/// project root has no name of its own, and the empty path is that name, so
/// an empty answer means the project.
///
/// # Errors
///
/// Returns [`ForeignWorkspace`][foreign] when the project root does not
/// contain the workspace root.
///
/// [foreign]: CheckDependenciesError::ForeignWorkspace
// checkdependencies[impl check.location]
fn location(root: &CargoRoot, project: &ProjectRoot) -> Result<Location, CheckDependenciesError> {
    let directory = root.relative_directory(project).ok_or_else(|| {
        CheckDependenciesError::ForeignWorkspace {
            path: root.directory().clone(),
        }
    })?;

    if directory.as_os_str().is_empty() {
        return Ok(Location::Project);
    }

    let path = DirectoryPath::try_from(directory).map_err(|_| {
        CheckDependenciesError::ForeignWorkspace {
            path: root.directory().clone(),
        }
    })?;

    Ok(Location::Directory { path })
}

/// Returns the summary that tells what a passing run examined and read
///
/// The summary counts the workspaces, so that a reader can question a pass
/// that covered less than they expect. It counts the warnings next to them,
/// because a warning is a shape that the project asked to read about, and a
/// warning that no outcome carried would be one that nobody reads. A run
/// without a warning says nothing about warnings.
// checkdependencies[impl check.passed]
fn summary(roots: usize, warnings: usize) -> Summary {
    let workspaces = if roots == 1 { WORKSPACE } else { WORKSPACES };
    let checked = format!("checked {roots} {workspaces}");

    if warnings == 0 {
        return Summary::new(checked);
    }

    let word = if warnings == 1 { WARNING } else { WARNINGS };

    Summary::new(format!("{checked}, {warnings} {word}"))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::{Path, PathBuf};

    use super::*;
    use crate::problem::Package;

    /// Returns the workspace root that lies at the root of the project
    fn project_workspace() -> CargoRoot {
        CargoRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns a problem of the given weight about one package
    fn problem(severity: Severity) -> DenyProblem {
        DenyProblem::new(
            severity,
            "rejected".to_owned(),
            "failed to satisfy license requirements".to_owned(),
            vec![Package::new("option-ext".to_owned(), "0.2.0".to_owned())],
        )
    }

    /// Returns the root of the project that the tests are about
    fn root() -> ProjectRoot {
        ProjectRoot::new(PathBuf::from("/home/otter/project"))
    }

    /// Returns a workspace root below the root of the project
    fn tool_workspace() -> CargoRoot {
        CargoRoot::new(PathBuf::from("/home/otter/project/tools/harness"))
    }

    // checkdependencies[verify check.finding]
    #[test]
    fn finding_of_a_problem_carries_the_message_of_cargo_deny() {
        let finding = finding(&problem(Severity::Error), &project_workspace(), &root()).unwrap();

        assert_eq!(
            finding.message().get(),
            "[rejected] failed to satisfy license requirements (option-ext 0.2.0)"
        );
    }

    // checkdependencies[verify check.location]
    #[test]
    fn location_of_a_workspace_below_the_project_names_its_directory() {
        let location = location(&tool_workspace(), &root()).unwrap();

        assert_eq!(
            location,
            Location::Directory {
                path: DirectoryPath::try_from("tools/harness").unwrap(),
            }
        );
    }

    // checkdependencies[verify check.location]
    #[test]
    fn location_of_the_workspace_at_the_project_root_names_the_project() {
        let location = location(&project_workspace(), &root()).unwrap();

        assert_eq!(location, Location::Project);
    }

    #[test]
    fn location_of_a_workspace_outside_the_project_reports_the_path() {
        let outside = CargoRoot::new(PathBuf::from("/home/otter/elsewhere"));

        let error = location(&outside, &root()).unwrap_err();

        assert!(
            matches!(&error, CheckDependenciesError::ForeignWorkspace { path } if path == Path::new("/home/otter/elsewhere")),
            "expected the path of the workspace, got {error:?}"
        );
    }

    // checkdependencies[verify check.passed]
    #[test]
    fn summary_of_one_workspace_counts_it_in_the_singular() {
        let summary = summary(1, 0);

        assert_eq!(summary.get(), "checked 1 workspace");
    }

    // checkdependencies[verify check.passed]
    #[test]
    fn summary_of_several_workspaces_counts_them() {
        let summary = summary(2, 0);

        assert_eq!(summary.get(), "checked 2 workspaces");
    }

    // checkdependencies[verify check.passed]
    #[test]
    fn summary_with_one_warning_counts_it_in_the_singular() {
        let summary = summary(2, 1);

        assert_eq!(summary.get(), "checked 2 workspaces, 1 warning");
    }

    // checkdependencies[verify check.passed]
    #[test]
    fn summary_with_several_warnings_counts_them() {
        let summary = summary(2, 3);

        assert_eq!(summary.get(), "checked 2 workspaces, 3 warnings");
    }
}
