//! The action that finds the dependencies a project declares and never uses
//!
//! This module holds the action and the error that stops a run. The action
//! wraps cargo as a subprocess: cargo builds the targets of a workspace,
//! cargo-udeps reads which crates those targets loaded, and the action
//! translates what both reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{Action, Context, Finding, Name, Outcome, SkipReason, Summary, action_name};
use rakko_cargo::{Cargo, CargoReport, CargoRoot, Channel, Toolchain};
use rakko_tool::Execution;

pub use self::error::CheckUnusedDepsError;
use crate::report::UdepsReport;

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

/// The channel that cargo-udeps needs
///
/// The record that names the crates a target loaded comes from an unstable
/// option of the compiler, and a stable toolchain refuses it.
const NIGHTLY: &str = "nightly";

/// The arguments that ask cargo-udeps to examine every target of every
/// package with every feature
///
/// A dependency that only a test reaches, and a dependency that only a
/// feature reaches, are used as much as one that the library reaches, so a
/// run that examined less would call them unused. The reports of cargo-udeps
/// and of cargo arrive as JSON, because the action reads them as data. The
/// formats select the presentation of the reports and not the behavior of
/// the tools: which dependencies a run passes over comes from the
/// configuration of the project alone.
const UDEPS: [&str; 6] = [
    "udeps",
    "--workspace",
    "--all-targets",
    "--all-features",
    "--output=json",
    "--message-format=json",
];

/// The action that finds the dependencies a project declares and never uses
///
/// The action wraps [cargo-udeps]: cargo builds every target of every
/// package with every feature, and cargo-udeps holds the crates that those
/// targets loaded against the dependencies of the manifests, so a run agrees
/// with a contributor that runs cargo-udeps bare. The cargo that runs is the
/// one that [mise] installed for the project, on the nightly toolchain that
/// mise installed for it, and the action installs nothing.
///
/// A run only reports, and it takes no argument. It examines every workspace
/// of the project, because the harness of a project is a package of its own
/// and declares dependencies of its own. Every unused dependency becomes a
/// finding at the manifest that declares it. A build that does not finish
/// leaves cargo-udeps without an answer, and the diagnostics of the compiler
/// become the findings instead.
///
/// The action applies to a project that holds a manifest of cargo, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// cargo or no nightly toolchain, when the workspaces of the project cannot
/// be discovered, and when the tools write a report that the action does not
/// recognize.
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
    // checkunuseddeps[impl args.none]
    type Args = ();

    // checkunuseddeps[impl name]
    fn name(&self) -> Name {
        action_name!("check-unused-deps")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // checkunuseddeps[impl roots.error]
            // checkunuseddeps[impl tool.missing]
            // checkunuseddeps[impl tool.unpinned]
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
/// Returns the error of the step that could not finish: the resolution of
/// the tool or of the toolchain, the discovery of the workspaces, a run of
/// cargo, or the reading of a report.
async fn drive(context: &Context) -> Result<Outcome, CheckUnusedDepsError> {
    // checkunuseddeps[impl skip.git]
    // checkunuseddeps[impl skip.links]
    // checkunuseddeps[impl skip.missing]
    // checkunuseddeps[impl skip.target]
    if !Cargo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MANIFEST),
        });
    }

    // checkunuseddeps[impl tool.cargo]
    // checkunuseddeps[impl tool.missing]
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| CheckUnusedDepsError::UnresolvedTool { source })?;

    // checkunuseddeps[impl tool.toolchain]
    // checkunuseddeps[impl tool.unpinned]
    let toolchain = Toolchain::resolve(Channel::new(NIGHTLY), context.root())
        .await
        .map_err(|source| CheckUnusedDepsError::UnresolvedToolchain { source })?;

    // checkunuseddeps[impl roots.error]
    let roots = cargo
        .roots()
        .await
        .map_err(|source| CheckUnusedDepsError::UndiscoveredRoots { source })?;

    let mut findings = Vec::new();

    // checkunuseddeps[impl roots.every]
    for root in &roots {
        findings.extend(check(&cargo, root, &toolchain, context).await?);
    }

    if findings.is_empty() {
        // checkunuseddeps[impl check.passed]
        Ok(Outcome::Passed {
            summary: Some(summary(roots.len())),
        })
    } else {
        // checkunuseddeps[impl check.failed]
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
/// The diagnostics of a build that finished stay behind: they belong to the
/// action that lints the code, and the answer of this run is the report of
/// cargo-udeps.
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run,
/// [`UnrecognizedReport`][unrecognized] when the run wrote no report that
/// the action can answer from, and the reading errors when a report holds a
/// record that the action cannot read.
///
/// [unavailable]: CheckUnusedDepsError::CargoUnavailable
/// [unrecognized]: CheckUnusedDepsError::UnrecognizedReport
// checkunuseddeps[impl check.operation]
// checkunuseddeps[impl check.read]
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

    let stdout = execution.stdout().to_string_lossy();

    // checkunuseddeps[impl check.unreadable]
    let build = CargoReport::read(&stdout).map_err(|source| {
        CheckUnusedDepsError::UnreadableCargoReport {
            root: root.directory().clone(),
            source,
        }
    })?;

    // checkunuseddeps[impl check.unreadable]
    let report = UdepsReport::read(&stdout).map_err(|source| {
        CheckUnusedDepsError::UnreadableUdepsReport {
            root: root.directory().clone(),
            source,
        }
    })?;

    // checkunuseddeps[impl check.unrecognized]
    if !recognized(report.as_ref(), &build, &execution) {
        return Err(CheckUnusedDepsError::UnrecognizedReport {
            root: root.directory().clone(),
            stderr: execution.stderr().to_string_lossy().into_owned(),
        });
    }

    // checkunuseddeps[impl check.diagnostic]
    let Some(report) = report else {
        return Ok(build
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.finding(root, context.root()))
            .collect());
    };

    // checkunuseddeps[impl check.failed]
    // checkunuseddeps[impl check.finding]
    // checkunuseddeps[impl check.foreign]
    Ok(report
        .dependencies()
        .iter()
        .map(|dependency| dependency.finding(root, context.root()))
        .collect())
}

/// Returns whether the action can answer from a run
///
/// A run that reached an answer wrote the report of cargo-udeps, whatever
/// its exit status says: the status only tells a clean workspace from one
/// with an unused dependency. A run without that report answers only when it
/// ended without success and the compiler said why, because that is a build
/// that did not finish. Anything else leaves the action without an answer.
// checkunuseddeps[impl check.unrecognized]
fn recognized(report: Option<&UdepsReport>, build: &CargoReport, execution: &Execution) -> bool {
    if report.is_some() {
        return true;
    }

    !execution.status().success() && !build.diagnostics().is_empty()
}

/// Returns the summary that tells how many workspaces the run examined
// checkunuseddeps[impl check.passed]
fn summary(roots: usize) -> Summary {
    if roots == 1 {
        Summary::new("examined 1 workspace")
    } else {
        Summary::new(format!("examined {roots} workspaces"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // checkunuseddeps[verify check.passed]
    #[test]
    fn summary_of_one_workspace_says_so() {
        let summary = summary(1);

        assert_eq!(summary.get(), "examined 1 workspace");
    }

    // checkunuseddeps[verify check.passed]
    #[test]
    fn summary_of_two_workspaces_counts_them() {
        let summary = summary(2);

        assert_eq!(summary.get(), "examined 2 workspaces");
    }
}
