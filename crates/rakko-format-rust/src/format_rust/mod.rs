//! The action that formats the Rust code of a project
//!
//! This module holds the action, the arguments that a run reads, the error
//! that stops a run, and the reading of what rustfmt reports. The action
//! wraps cargo as a subprocess: cargo reads the manifests, rustfmt reads its
//! own configuration and does the formatting, and the action translates
//! what rustfmt reported into an outcome.

/// The arguments that a run of the action reads
mod args;
/// The error that stops a run of the action
mod error;
/// What rustfmt reported about a run
mod report;

use std::collections::HashSet;
use std::path::PathBuf;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, SkipReason, Summary, action_name,
};
use rakko_cargo::{Cargo, CargoRoot, Channel, Toolchain};

pub use self::args::FormatRustArgs;
pub use self::error::FormatRustError;
pub use self::report::{RustfmtProblem, RustfmtProblemDetail, RustfmtReport};

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

/// The channel of the toolchain that rustfmt runs on
///
/// Rustfmt honors the unstable options of a configuration only on this
/// channel, and a stable rustfmt formats without them.
const NIGHTLY: &str = "nightly";

/// The arguments that ask rustfmt to report the files it would rewrite
///
/// The short report lists the files instead of showing a diff of each,
/// because the action reads the report as data. The format selects the
/// presentation of the report and not the behavior of the tool: what
/// rustfmt does to a project comes from the configuration of that project
/// alone.
const CHECK: [&str; 4] = ["fmt", "--check", "--message-format", "short"];

/// The arguments that ask rustfmt to rewrite the files and list them
const FORMAT: [&str; 3] = ["fmt", "--message-format", "short"];

/// The message of a finding about a file that is not formatted
const UNFORMATTED_FINDING: &str = "the file is not properly formatted";

/// The message of a repair of a file that was not formatted
const UNFORMATTED_REPAIR: &str = "the file was not properly formatted";

/// The action that formats the Rust code of a project
///
/// The action wraps [rustfmt]: cargo reads the manifests of the project, and
/// rustfmt reads its own configuration and formats every target, so a run
/// agrees with an editor and with a contributor that runs rustfmt bare. The
/// cargo that runs is the one that [mise] installed for the project, on the
/// nightly toolchain that the project pinned, and the action installs
/// nothing.
///
/// A run reports by default. A file that is not formatted becomes a finding
/// that names the file, and a file that rustfmt cannot parse becomes a
/// finding at the position that rustfmt reports. With the `fix` argument,
/// rustfmt rewrites the files that it can format, and the outcome carries
/// one repair for each file that it rewrote, next to the problems that
/// remain. A run formats every workspace of the project, because the harness
/// of a project is a package of its own.
///
/// The action applies to a project that holds a manifest of cargo, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// cargo or no nightly toolchain, when the workspaces of the project cannot
/// be discovered, when rustfmt warns about its configuration, and when
/// rustfmt writes a report that the action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_format_rust::FormatRust;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(FormatRust)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [rustfmt]: https://github.com/rust-lang/rustfmt
#[derive(Copy, Clone, Debug, Default)]
pub struct FormatRust;

impl Action for FormatRust {
    type Args = FormatRustArgs;

    // formatrust[impl name]
    fn name(&self) -> Name {
        action_name!("format-rust")
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args).await {
            Ok(outcome) => outcome,
            // formatrust[impl roots.error]
            // formatrust[impl tool.missing]
            // formatrust[impl tool.unpinned]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves cargo and the nightly toolchain,
/// discovers the workspaces, checks each of them, and fixes when the
/// arguments ask for it. An error that this function returns stops the run,
/// and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool or the toolchain, the discovery of the workspaces, a rustfmt
/// run, or the reading of a report.
async fn drive(context: &Context, args: &FormatRustArgs) -> Result<Outcome, FormatRustError> {
    // formatrust[impl skip.git]
    // formatrust[impl skip.links]
    // formatrust[impl skip.missing]
    // formatrust[impl skip.target]
    if !Cargo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MANIFEST),
        });
    }

    // formatrust[impl tool.cargo]
    // formatrust[impl tool.missing]
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| FormatRustError::UnresolvedTool { source })?;

    // formatrust[impl tool.toolchain]
    // formatrust[impl tool.unpinned]
    let toolchain = Toolchain::resolve(Channel::new(NIGHTLY), context.root())
        .await
        .map_err(|source| FormatRustError::UnresolvedToolchain { source })?;

    // formatrust[impl roots.error]
    let roots = cargo
        .roots()
        .await
        .map_err(|source| FormatRustError::UndiscoveredRoots { source })?;

    let mut findings = Vec::new();
    let mut repairs = Vec::new();

    // formatrust[impl roots.all]
    for root in &roots {
        // formatrust[impl check.read]
        let checked = observe(&cargo, &toolchain, root, &CHECK).await?;

        if checked.listed().is_empty() && checked.invalid().is_empty() {
            continue;
        }

        if args.fix() {
            let (repaired, remaining) = fix(&cargo, &toolchain, root, &checked).await?;
            repairs.extend(
                repaired
                    .iter()
                    .map(|problem| finding(problem, root, context, UNFORMATTED_REPAIR))
                    .collect::<Result<Vec<Finding>, FormatRustError>>()?,
            );
            findings.extend(findings_of(&remaining, root, context)?);
        } else {
            // formatrust[impl check.invalid]
            // formatrust[impl check.unformatted]
            findings.extend(findings_of(&checked.problems(), root, context)?);
        }
    }

    Ok(outcome(findings, repairs, roots.len()))
}

/// Returns the finding that reports one problem of the project
///
/// A file that is not formatted gets a finding at the level of the file, and
/// a file that rustfmt cannot parse gets one at the position that rustfmt
/// named. The message for an unformatted file comes from the caller,
/// because a finding states a problem that the project has and a repair
/// states one that the run took away.
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of the problem.
///
/// [foreign]: FormatRustError::ForeignPath
fn finding(
    problem: &RustfmtProblem,
    root: &CargoRoot,
    context: &Context,
    unformatted_message: &str,
) -> Result<Finding, FormatRustError> {
    let path = root
        .relative_path(problem.path(), context.root())
        .ok_or_else(|| FormatRustError::ForeignPath {
            path: problem.path().clone(),
        })?;

    let finding = match problem.detail() {
        RustfmtProblemDetail::Invalid {
            line,
            column,
            message,
        } => Finding::builder()
            .message(message.clone())
            .location(Location::Position {
                path,
                position: Position::builder().line(*line).column(*column).build(),
            })
            .build(),
        RustfmtProblemDetail::Unformatted => Finding::builder()
            .message(unformatted_message)
            .location(Location::File { path })
            .build(),
    };

    Ok(finding)
}

/// Returns the findings that report the given problems
///
/// # Errors
///
/// Returns [`ForeignPath`][foreign] when the project root does not contain
/// the path of a problem.
///
/// [foreign]: FormatRustError::ForeignPath
fn findings_of(
    problems: &[RustfmtProblem],
    root: &CargoRoot,
    context: &Context,
) -> Result<Vec<Finding>, FormatRustError> {
    problems
        .iter()
        .map(|problem| finding(problem, root, context, UNFORMATTED_FINDING))
        .collect()
}

/// Rewrites one workspace and splits the problems of the check into the
/// repaired ones and the remaining ones
///
/// The rewrite lists the files that it rewrote, so a problem of the check
/// whose file the rewrite lists was repaired, and every other problem
/// remains. A file that rustfmt cannot parse remains this way, and so does
/// every file of the package that holds it, because rustfmt rewrites
/// nothing in a package that it cannot read as a whole.
///
/// # Errors
///
/// Returns the error of a rewrite that could not finish: cargo did not run,
/// rustfmt warned about its configuration, or it wrote a report that the
/// action does not recognize.
// formatrust[impl fix.write]
async fn fix(
    cargo: &Cargo,
    toolchain: &Toolchain,
    root: &CargoRoot,
    checked: &RustfmtReport,
) -> Result<(Vec<RustfmtProblem>, Vec<RustfmtProblem>), FormatRustError> {
    let rewritten = observe(cargo, toolchain, root, &FORMAT).await?;
    let repaired: HashSet<&PathBuf> = rewritten.listed().iter().collect();

    Ok(checked
        .problems()
        .into_iter()
        .partition(|problem| repaired.contains(problem.path())))
}

/// Runs rustfmt once at a root and reads what it reported
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run,
/// [`RejectedConfiguration`][rejected] when rustfmt warned about its
/// configuration, and [`UnrecognizedReport`][unrecognized] when the run
/// ended without success and named no problem.
///
/// [rejected]: FormatRustError::RejectedConfiguration
/// [unavailable]: FormatRustError::CargoUnavailable
/// [unrecognized]: FormatRustError::UnrecognizedReport
// formatrust[impl check.operation]
// formatrust[impl tool.toolchain]
async fn observe(
    cargo: &Cargo,
    toolchain: &Toolchain,
    root: &CargoRoot,
    arguments: &[&str],
) -> Result<RustfmtReport, FormatRustError> {
    let execution = cargo
        .invocation_with_toolchain(root, toolchain)
        .args(arguments.iter().copied())
        .run()
        .await
        .map_err(|source| FormatRustError::CargoUnavailable { source })?;

    let report = RustfmtReport::read(&execution);

    // formatrust[impl check.configuration]
    if let Some(details) = report.warning() {
        return Err(FormatRustError::RejectedConfiguration {
            details: details.clone(),
        });
    }

    // formatrust[impl check.unrecognized]
    if !report.succeeded() && report.listed().is_empty() && report.invalid().is_empty() {
        return Err(FormatRustError::UnrecognizedReport {
            root: root.directory().clone(),
            stderr: report.stderr().clone(),
        });
    }

    Ok(report)
}

/// Returns the outcome of a run from what it found and what it repaired
///
/// A run without findings and without repairs passed, a run that repaired
/// everything changed the project, and a run with a finding failed, whether
/// it repaired something on the way or not.
// formatrust[impl check.passed]
// formatrust[impl fix.changed]
// formatrust[impl fix.partial]
fn outcome(findings: Vec<Finding>, repairs: Vec<Finding>, roots: usize) -> Outcome {
    if !findings.is_empty() {
        Outcome::Failed { findings, repairs }
    } else if !repairs.is_empty() {
        Outcome::Changed { repairs }
    } else {
        Outcome::Passed {
            summary: Some(summary(roots)),
        }
    }
}

/// Returns the summary that tells how many workspaces the run checked
// formatrust[impl check.passed]
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

    /// Returns a finding at the level of the project, for the tests of the
    /// outcome
    fn problem() -> Finding {
        Finding::builder()
            .message("a problem")
            .location(Location::Project)
            .build()
    }

    // formatrust[verify fix.partial]
    #[test]
    fn outcome_with_a_finding_and_a_repair_fails() {
        let outcome = outcome(vec![problem()], vec![problem()], 1);

        assert!(
            matches!(outcome, Outcome::Failed { .. }),
            "expected a failure, got {outcome:?}"
        );
    }

    // formatrust[verify fix.changed]
    #[test]
    fn outcome_with_only_repairs_reports_the_change() {
        let outcome = outcome(Vec::new(), vec![problem()], 1);

        assert!(
            matches!(outcome, Outcome::Changed { .. }),
            "expected a change, got {outcome:?}"
        );
    }

    // formatrust[verify check.passed]
    #[test]
    fn outcome_without_findings_or_repairs_passes() {
        let outcome = outcome(Vec::new(), Vec::new(), 1);

        assert!(
            matches!(outcome, Outcome::Passed { .. }),
            "expected a pass, got {outcome:?}"
        );
    }

    // formatrust[verify check.passed]
    #[test]
    fn summary_of_one_workspace_says_so() {
        let summary = summary(1);

        assert_eq!(summary.get(), "checked 1 workspace");
    }

    // formatrust[verify check.passed]
    #[test]
    fn summary_of_two_workspaces_counts_them() {
        let summary = summary(2);

        assert_eq!(summary.get(), "checked 2 workspaces");
    }
}
