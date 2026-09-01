//! The action that lints the Rust code of a project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps cargo as a subprocess: cargo reads the manifests, selects the lints
//! that the project configured, and runs clippy, and the action translates
//! what cargo reported into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{Action, Context, Finding, Name, Outcome, SkipReason, Summary, action_name};
use rakko_cargo::{Cargo, CargoReport, CargoRoot};
use rakko_tool::Execution;

pub use self::error::LintRustError;

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

/// The arguments that ask cargo to lint every target with every feature
///
/// The report arrives as JSON, because the action reads it as data. The
/// format selects the presentation of the report and not the behavior of the
/// tool: which lints apply, and at which level, comes from the configuration
/// of the project alone.
const CLIPPY: [&str; 4] = [
    "clippy",
    "--all-targets",
    "--all-features",
    "--message-format=json",
];

/// The action that lints the Rust code of a project
///
/// The action wraps [clippy]: cargo reads the manifests of the project,
/// selects the lints that the project configured, and examines every target
/// with every feature, so a run agrees with an editor and with a contributor
/// that runs clippy bare. The cargo that runs is the one that [mise]
/// installed for the project, at the version that the project pinned, and
/// the action installs nothing.
///
/// A run only reports, and it takes no argument. It lints every workspace of
/// the project, because the harness of a project is a package of its own.
/// Every diagnostic becomes a finding at the range that the compiler named,
/// with the message of the compiler and the code of the lint, and a run with
/// a finding fails, whether the project warns about the lint or denies it.
///
/// The action applies to a project that holds a manifest of cargo, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// cargo, when the workspaces of the project cannot be discovered, and when
/// cargo writes a report that the action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_lint_rust::LintRust;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(LintRust)];
/// ```
///
/// [clippy]: https://doc.rust-lang.org/clippy/
/// [mise]: https://mise.jdx.dev
#[derive(Copy, Clone, Debug, Default)]
pub struct LintRust;

impl Action for LintRust {
    // lintrust[impl args.none]
    type Args = ();

    // lintrust[impl name]
    fn name(&self) -> Name {
        action_name!("lint-rust")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // lintrust[impl roots.error]
            // lintrust[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves cargo, discovers the workspaces,
/// and lints each of them. An error that this function returns stops the
/// run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, the discovery of the workspaces, a cargo run, or the reading
/// of a report.
async fn drive(context: &Context) -> Result<Outcome, LintRustError> {
    // lintrust[impl skip.git]
    // lintrust[impl skip.links]
    // lintrust[impl skip.missing]
    // lintrust[impl skip.target]
    if !Cargo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MANIFEST),
        });
    }

    // lintrust[impl tool.cargo]
    // lintrust[impl tool.missing]
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| LintRustError::UnresolvedTool { source })?;

    // lintrust[impl roots.error]
    let roots = cargo
        .roots()
        .await
        .map_err(|source| LintRustError::UndiscoveredRoots { source })?;

    let mut findings = Vec::new();

    // lintrust[impl roots.all]
    for root in &roots {
        findings.extend(lint(&cargo, root, context).await?);
    }

    if findings.is_empty() {
        // lintrust[impl check.passed]
        Ok(Outcome::Passed {
            summary: Some(summary(roots.len())),
        })
    } else {
        // lintrust[impl check.failed]
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Lints one workspace of the project and returns the findings
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run, and
/// [`UnrecognizedReport`][unrecognized] when cargo writes a report that the
/// action cannot answer from.
///
/// [unavailable]: LintRustError::CargoUnavailable
/// [unrecognized]: LintRustError::UnrecognizedReport
// lintrust[impl check.diagnostic]
// lintrust[impl check.operation]
// lintrust[impl check.read]
async fn lint(
    cargo: &Cargo,
    root: &CargoRoot,
    context: &Context,
) -> Result<Vec<Finding>, LintRustError> {
    let execution = cargo
        .invocation(root)
        .args(CLIPPY)
        .run()
        .await
        .map_err(|source| LintRustError::CargoUnavailable { source })?;

    let report = CargoReport::read(&execution.stdout().to_string_lossy());

    // lintrust[impl check.unrecognized]
    if !recognized(&report, &execution) {
        return Err(LintRustError::UnrecognizedReport {
            root: root.directory().clone(),
            stderr: execution.stderr().to_string_lossy().into_owned(),
        });
    }

    Ok(report
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.finding(root, context.root()))
        .collect())
}

/// Returns whether the action can answer from a report
///
/// A run that ended without success and named no diagnostic, and a run that
/// ended with success without saying that the build finished, both wrote a
/// report that the action cannot read, and an answer built on such a report
/// would hide problems behind a green result.
// lintrust[impl check.unrecognized]
fn recognized(report: &CargoReport, execution: &Execution) -> bool {
    if execution.status().success() {
        report.finished() == Some(true)
    } else {
        !report.diagnostics().is_empty()
    }
}

/// Returns the summary that tells how many workspaces the run checked
// lintrust[impl check.passed]
fn summary(roots: usize) -> Summary {
    if roots == 1 {
        Summary::new("checked 1 workspace")
    } else {
        Summary::new(format!("checked {roots} workspaces"))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // lintrust[verify check.passed]
    #[test]
    fn summary_of_one_workspace_says_so() {
        let summary = summary(1);

        assert_eq!(summary.get(), "checked 1 workspace");
    }

    // lintrust[verify check.passed]
    #[test]
    fn summary_of_two_workspaces_counts_them() {
        let summary = summary(2);

        assert_eq!(summary.get(), "checked 2 workspaces");
    }
}
