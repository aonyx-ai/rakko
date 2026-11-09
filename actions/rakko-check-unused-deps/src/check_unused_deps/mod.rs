//! The action that finds the dependencies a project declares and never uses
//!
//! This module holds the action and the error that stops a run. The action
//! wraps cargo as a subprocess. Cargo builds the targets of a workspace.
//! Cargo-udeps reads which crates those targets loaded. The action
//! translates what both reported into an outcome.

mod error;

use rakko::{
    Access, Action, Context, Finding, Name, Outcome, SkipReason, Summary, ToolRequirement,
    action_name,
};
use rakko_cargo::lockfile::{self, LOCKED};
use rakko_cargo::{Cargo, CargoReport, CargoRoot, Toolchain};
use rakko_tool::Execution;

pub use self::error::CheckUnusedDepsError;
use crate::report::UdepsReport;
use crate::udeps::Udeps;

/// The reason of a run whose cargo discovered no workspace
const NO_WORKSPACE: &str = "cargo discovered no workspace in the project";

/// The arguments that ask cargo-udeps to examine every target of every
/// package with every feature
///
/// The first argument is the subcommand of cargo that cargo-udeps provides.
/// Cargo hands it to the program of a plugin, and the program expects it
/// when it starts without cargo as well.
///
/// A dependency that only a test reaches is used as much as one that the
/// library reaches. The same holds for a dependency that only a feature
/// reaches. So a run that examined less would call them unused. Cargo builds
/// the versions that the lockfile holds and does not write the lockfile,
/// because the action only examines the project. The reports of cargo-udeps and
/// cargo arrive as JSON because the action reads them as data. The formats
/// select the presentation of the reports and not the behavior of the tools.
/// Which dependencies a run passes over comes from the configuration of the
/// project alone.
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
/// contributor that runs cargo-udeps bare. The cargo-udeps that runs is the
/// version that the action declares, which [mise] installed, whatever version
/// the project pins. Its builds run on the nightly toolchain that the project
/// pins, and the action installs nothing.
///
/// A run only reports, and it takes no argument. It examines every workspace
/// of the project, because the harness of a project is a package of its own
/// and declares dependencies of its own. Every unused dependency becomes a
/// finding at the manifest that declares it. A build that does not finish
/// leaves cargo-udeps without an answer, and the diagnostics of the compiler
/// become the findings instead.
///
/// A project whose cargo discovers no workspace skips visibly, and says so. A
/// run stops with an error when the project pins no stable or no nightly Rust
/// toolchain that the action accepts. It stops when mise reports no cargo-udeps
/// at the declared version. It stops when the workspaces of the project cannot
/// be discovered, or when the lockfile of a workspace needs an update. It stops
/// when the tools write a report that the action does not recognize.
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
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use rakko_check_unused_deps::CheckUnusedDeps;
///
/// let actions = Bundle::default().with(CheckUnusedDeps);
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

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![
            Cargo::requirement(),
            Toolchain::nightly(),
            Udeps::requirement(),
        ]
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
/// The run examines the project, resolves cargo, the nightly toolchain, and
/// cargo-udeps, discovers the workspaces, and examines each of them. An error
/// that this function returns stops the run, and the caller reports it in the
/// outcome.
///
/// The run asks for the nightly toolchain, because the record that names the
/// crates a target loaded comes from an unstable option of the compiler. A
/// stable toolchain refuses that option.
///
/// # Errors
///
/// Returns the error of the step that could not finish. That step is the
/// resolution of a tool or of the toolchain. It can also be the discovery of
/// the workspaces, a run of cargo-udeps, or the reading of a report.
async fn drive(context: &Context) -> Result<Outcome, CheckUnusedDepsError> {
    let cargo = Cargo::resolve(context)
        .await
        .map_err(|source| CheckUnusedDepsError::UnresolvedCargo { source })?;

    let toolchain = Toolchain::resolve_requirement()
        .requirement(&Toolchain::nightly())
        .context(context)
        .call()
        .await
        .map_err(|source| CheckUnusedDepsError::UnresolvedToolchain { source })?;

    let udeps = Udeps::resolve(context)
        .await
        .map_err(|source| CheckUnusedDepsError::UnresolvedUdeps { source })?;

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
        findings.extend(check(&cargo, &udeps, root, &toolchain, context).await?);
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
/// Returns [`UdepsUnavailable`][unavailable] when cargo-udeps does not run. It
/// returns [`StaleLockfile`][stale] when cargo refused to build because the
/// lockfile needs an update. It returns [`UnrecognizedReport`][unrecognized]
/// when the run wrote no report that the action can answer from. It returns the
/// reading errors when a report holds a record that the action cannot read.
///
/// [stale]: CheckUnusedDepsError::StaleLockfile
/// [unavailable]: CheckUnusedDepsError::UdepsUnavailable
/// [unrecognized]: CheckUnusedDepsError::UnrecognizedReport
async fn check(
    cargo: &Cargo,
    udeps: &Udeps,
    root: &CargoRoot,
    toolchain: &Toolchain,
    context: &Context,
) -> Result<Vec<Finding>, CheckUnusedDepsError> {
    let execution = cargo
        .plugin_invocation_with_toolchain(root, udeps.tool(), toolchain)
        .args(UDEPS)
        .run()
        .await
        .map_err(|source| CheckUnusedDepsError::UdepsUnavailable { source })?;

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
