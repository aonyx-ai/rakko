//! The action that finds the dependencies a project declares and never uses
//!
//! This module holds the action and the error that stops a run. The action
//! wraps cargo as a subprocess. Cargo builds the targets of a workspace.
//! Cargo-udeps reads which crates those targets loaded. The action
//! translates what both reported into an outcome.

mod error;

use rakko_action::{
    Access, Action, Context, Finding, Name, Outcome, SkipReason, Summary, action_name,
};
use rakko_cargo::lockfile::{self, LOCKED};
use rakko_cargo::{Cargo, CargoReport, CargoRoot, Channel, Toolchain};
use rakko_tool::Execution;

pub use self::error::CheckUnusedDepsError;
use crate::report::UdepsReport;

/// The reason of a run whose cargo discovered no workspace
const NO_WORKSPACE: &str = "cargo discovered no workspace in the project";

/// The channel that cargo-udeps needs
///
/// The record that names the crates a target loaded comes from an unstable
/// option of the compiler, and a stable toolchain refuses it.
const NIGHTLY: &str = "nightly";

/// The arguments that ask cargo-udeps to examine every target of every
/// package with every feature
///
/// A dependency that only a test reaches, and a dependency that only a feature
/// reaches, are used as much as one that the library reaches. So a run that
/// examined less would call them unused. Cargo builds the versions that the
/// lockfile holds and does not write the lockfile, because the action only
/// examines the project. The reports of cargo-udeps and cargo arrive as JSON,
/// because the action reads them as data. The formats select the presentation
/// of the reports. They do not select the behavior of the tools: which
/// dependencies a run passes over comes from the configuration of the project
/// alone.
const UDEPS: [&str; 7] = [
    "udeps",
    LOCKED,
    "--workspace",
    "--all-targets",
    "--all-features",
    "--output=json",
    "--message-format=json",
];

/// The action that finds the dependencies a project declares and never uses
///
/// The action wraps [cargo-udeps]. Cargo builds every target of every package
/// with every feature. Cargo-udeps holds the crates that those targets loaded
/// against the dependencies of the manifests. So a run agrees with a
/// contributor that runs cargo-udeps bare. The cargo that runs is the one that
/// [mise] installed for the project, on the nightly toolchain that mise
/// installed for it. The action installs nothing.
///
/// A run only reports, and it takes no argument. It examines every workspace
/// of the project, because the harness of a project is a package of its own
/// and declares dependencies of its own. Every unused dependency becomes a
/// finding at the manifest that declares it. A build that does not finish
/// leaves cargo-udeps without an answer, and the diagnostics of the compiler
/// become the findings instead.
///
/// A project whose cargo discovers no workspace skips visibly, and says so.
/// A run stops with an error when mise reports no cargo or no nightly
/// toolchain. A run stops when the workspaces of the project cannot be
/// discovered. A run stops when the lockfile of a workspace needs an update.
/// A run stops when the tools write a report that the action does not
/// recognize.
///
/// A run declares a read of the project. It holds cargo to the lockfile of
/// each workspace. Cargo writes what it builds into the `target` directory.
/// Cargo locks that directory against parallel use itself. The declaration
/// covers the action and cargo. It does not cover the build scripts and
/// procedural macros of the project. Those can write files of their own. A
/// caller that orders runs by their declarations runs it after the actions
/// that write.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_check_unused_deps::CheckUnusedDeps;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckUnusedDeps)];
/// ```
///
/// [cargo-udeps]: https://github.com/est31/cargo-udeps
/// [mise]: https://mise.jdx.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckUnusedDeps;

impl Action for CheckUnusedDeps {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("check-unused-deps")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves cargo and the nightly toolchain,
/// discovers the workspaces, and examines each of them. An error that this
/// function returns stops the run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish. That step is the
/// resolution of the tool or of the toolchain, or the discovery of the
/// workspaces. It can also be a run of cargo, or the reading of a report.
async fn drive(context: &Context) -> Result<Outcome, CheckUnusedDepsError> {
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| CheckUnusedDepsError::UnresolvedTool { source })?;

    let toolchain = Toolchain::resolve()
        .channel(Channel::new(NIGHTLY))
        .root(context.root())
        .call()
        .await
        .map_err(|source| CheckUnusedDepsError::UnresolvedToolchain { source })?;

    let roots = cargo
        .roots()
        .await
        .map_err(|source| CheckUnusedDepsError::UndiscoveredRoots { source })?;

    if roots.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_WORKSPACE),
        });
    }

    let mut findings = Vec::new();

    for root in &roots {
        findings.extend(check(&cargo, root, &toolchain, context).await?);
    }

    if findings.is_empty() {
        Ok(Outcome::Passed {
            summary: Some(summary(roots.len())),
        })
    } else {
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Examines one workspace of the project and returns the findings
///
/// A run that reached an answer reports the dependencies that no target
/// loaded. A run whose build did not finish has no such answer, and the
/// diagnostics of the compiler say why, so they become the findings instead.
/// The diagnostics of a build that finished stay behind. They belong to the
/// action that lints the code. The answer of this run is the report of
/// cargo-udeps.
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run.
/// Returns [`StaleLockfile`][stale] when cargo refused to build because
/// the lockfile needs an update. Returns [`UnrecognizedReport`][unrecognized]
/// when the run wrote no report the action can answer from. Returns a
/// reading error when a report holds a record that the action cannot read.
///
/// [stale]: CheckUnusedDepsError::StaleLockfile
/// [unavailable]: CheckUnusedDepsError::CargoUnavailable
/// [unrecognized]: CheckUnusedDepsError::UnrecognizedReport
async fn check(
    cargo: &Cargo,
    root: &CargoRoot,
    toolchain: &Toolchain,
    context: &Context,
) -> Result<Vec<Finding>, CheckUnusedDepsError> {
    let execution = cargo
        .invocation_with_toolchain(root, toolchain)
        .args(UDEPS)
        .run()
        .await
        .map_err(|source| CheckUnusedDepsError::CargoUnavailable { source })?;

    if lockfile::refused(&execution.stderr().to_string_lossy()) {
        return Err(CheckUnusedDepsError::StaleLockfile {
            root: root.directory().clone(),
        });
    }

    let stdout = execution.stdout().to_string_lossy();

    let build = CargoReport::read(&stdout).map_err(|source| {
        CheckUnusedDepsError::UnreadableCargoReport {
            root: root.directory().clone(),
            source,
        }
    })?;

    let report = UdepsReport::read(&stdout).map_err(|source| {
        CheckUnusedDepsError::UnreadableUdepsReport {
            root: root.directory().clone(),
            source,
        }
    })?;

    if !recognized(report.as_ref(), &build, &execution) {
        return Err(CheckUnusedDepsError::UnrecognizedReport {
            root: root.directory().clone(),
            stderr: execution.stderr().to_string_lossy().into_owned(),
        });
    }

    let Some(report) = report else {
        return Ok(build
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.finding(root, context.root()))
            .collect());
    };

    Ok(report
        .dependencies()
        .iter()
        .map(|dependency| dependency.finding(root, context.root()))
        .collect())
}

/// Returns whether the action can answer from a run
///
/// A run that reached an answer wrote the report of cargo-udeps, whatever its
/// exit status says. The status only tells a clean workspace from one with an
/// unused dependency. A run without that report answers only when it ended
/// without success and the compiler said why. It answers then because that is a
/// build that did not finish. Anything else leaves the action without an
/// answer.
fn recognized(report: Option<&UdepsReport>, build: &CargoReport, execution: &Execution) -> bool {
    if report.is_some() {
        return true;
    }

    !execution.status().success() && !build.diagnostics().is_empty()
}

/// Returns the summary that tells how many workspaces the run examined
fn summary(roots: usize) -> Summary {
    if roots == 1 {
        Summary::new("examined 1 workspace")
    } else {
        Summary::new(format!("examined {roots} workspaces"))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    #[test]
    fn access_is_read() {
        let access = CheckUnusedDeps.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn check_unused_deps_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CheckUnusedDeps>();
    }

    #[test]
    fn summary_of_one_workspace_says_so() {
        let summary = summary(1);

        assert_eq!(summary.get(), "examined 1 workspace");
    }

    #[test]
    fn summary_of_two_workspaces_counts_them() {
        let summary = summary(2);

        assert_eq!(summary.get(), "examined 2 workspaces");
    }
}
