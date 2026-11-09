//! The action that formats the Rust code of a project
//!
//! This module holds the action, the arguments that a run reads, the error
//! that stops a run, and the reading of what rustfmt reports. The action
//! wraps cargo as a subprocess. Cargo reads the manifests, rustfmt reads its
//! own configuration and does the formatting. The action translates what
//! rustfmt reported into an outcome.

mod args;
mod error;
mod operation;
mod report;

use std::collections::HashSet;
use std::path::PathBuf;

use rakko::{
    Access, Action, Context, Finding, Location, Name, Outcome, Position, SkipReason, Summary,
    ToolRequirement, action_name,
};
use rakko_cargo::{Cargo, CargoRoot, Toolchain};

pub use self::args::FormatRustArgs;
pub use self::error::FormatRustError;
pub use self::operation::Operation;
pub(crate) use self::report::{Exit, RustfmtProblem, RustfmtProblemDetail, RustfmtReport};

/// The reason of a run whose cargo discovered no workspace
const NO_WORKSPACE: &str = "cargo discovered no workspace in the project";

/// The arguments that ask rustfmt to report the files it would rewrite
///
/// The short report lists the files instead of showing a diff of each,
/// because the action reads the report as data. The format selects the
/// presentation, not the behavior of the tool. What rustfmt does to a
/// project comes from the configuration of that project alone.
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
/// rustfmt reads its own configuration and formats every target. So a run
/// agrees with an editor and with a contributor that runs rustfmt bare. The
/// cargo that runs is the one that [mise] installed for the project. It runs
/// on the nightly toolchain that the project pins. The action installs nothing.
/// The action accepts a range of nightly toolchains by their dates, and the
/// first pin in that range answers.
///
/// A run reports by default. A file that is not formatted becomes a finding
/// that names the file. A file that rustfmt cannot parse becomes a finding at
/// the position that rustfmt reports. With the `fix` argument, rustfmt rewrites
/// the files that it can format. The outcome carries one repair for each file
/// that it rewrote, next to the problems that remain. A run formats every
/// workspace of the project, because the harness of a project is a package of
/// its own.
///
/// A run that reports declares a read of the project, and a run with `fix`
/// declares a write. A caller that orders runs by their declarations can
/// then run a rewrite before the actions that read.
///
/// A project whose cargo discovers no workspace skips visibly, and says so. A
/// run stops with an error when the project pins no stable or no nightly Rust
/// toolchain that the action accepts. The run also stops when the workspaces of
/// the project cannot be discovered, or when rustfmt warns about its
/// configuration. It also stops when rustfmt writes a report that the action
/// does not recognize.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use rakko_format_rust::FormatRust;
///
/// let actions = Bundle::default().with(FormatRust);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [rustfmt]: https://github.com/rust-lang/rustfmt
#[derive(Copy, Clone, Debug, Default)]
pub struct FormatRust;

impl Action for FormatRust {
    type Args = FormatRustArgs;

    fn name(&self) -> Name {
        action_name!("format-rust")
    }

    fn access(&self, _context: &Context, args: &Self::Args) -> Access {
        match args.operation() {
            Operation::Rewrite => Access::Write,
            Operation::Report => Access::Read,
        }
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Cargo::requirement(), Toolchain::nightly()]
    }

    async fn run(&self, context: &Context, args: &Self::Args) -> Outcome {
        match drive(context, args).await {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project and resolves cargo and the nightly toolchain.
/// It discovers the workspaces, checks each of them, and fixes when the
/// arguments ask for it. An error that this function returns stops the run, and
/// the caller reports it in the outcome.
///
/// The run asks for the nightly toolchain, because rustfmt honors the unstable
/// options of a configuration only on the nightly channel. A stable rustfmt
/// formats without them.
///
/// # Errors
///
/// Returns the error of the step that could not finish. That step is the
/// resolution of the tool or the toolchain, or the discovery of the workspaces.
/// It can also be a rustfmt run, or the reading of a report.
async fn drive(context: &Context, args: &FormatRustArgs) -> Result<Outcome, FormatRustError> {
    let cargo = Cargo::resolve(context)
        .await
        .map_err(|source| FormatRustError::UnresolvedTool { source })?;

    let toolchain = Toolchain::resolve_requirement()
        .requirement(&Toolchain::nightly())
        .context(context)
        .call()
        .await
        .map_err(|source| FormatRustError::UnresolvedToolchain { source })?;

    let roots = cargo
        .roots()
        .await
        .map_err(|source| FormatRustError::UndiscoveredRoots { source })?;

    if roots.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_WORKSPACE),
        });
    }

    let mut findings = Vec::new();
    let mut repairs = Vec::new();

    for root in &roots {
        let checked = observe(&cargo, &toolchain, root, &CHECK).await?;

        if checked.listed().is_empty() && checked.invalid().is_empty() {
            continue;
        }

        match args.operation() {
            Operation::Rewrite => {
                let (repaired, remaining) = fix(&cargo, &toolchain, root, &checked).await?;
                repairs.extend(
                    repaired
                        .iter()
                        .map(|problem| finding(problem, root, context, UNFORMATTED_REPAIR))
                        .collect::<Result<Vec<Finding>, FormatRustError>>()?,
                );
                findings.extend(findings_of(&remaining, root, context)?);
            }
            Operation::Report => {
                findings.extend(findings_of(&checked.problems(), root, context)?);
            }
        }
    }

    Ok(outcome(findings, repairs, roots.len()))
}

/// Returns the finding that reports one problem of the project
///
/// A file that is not formatted gets a finding at the level of the file. A file
/// that rustfmt cannot parse gets one at the position that rustfmt named. The
/// message for an unformatted file comes from the caller. This is because a
/// finding states a problem that the project has. A repair states one that the
/// run took away.
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
/// The rewrite lists the files that it rewrote. A problem of the check
/// whose file the rewrite lists was repaired. Every other problem remains.
/// A file that rustfmt cannot parse remains this way. So does every file
/// of the package that holds it, because rustfmt rewrites nothing in a
/// package that it cannot read as a whole.
///
/// # Errors
///
/// Returns the error of a rewrite that could not finish. This occurs when
/// cargo does not run, rustfmt warns about its configuration, or it writes
/// a report the action does not recognize.
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
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run.
/// Returns [`RejectedConfiguration`][rejected] when rustfmt warned about its
/// configuration. Returns [`UnrecognizedReport`][unrecognized] when the run
/// ended without success and named no problem.
///
/// [rejected]: FormatRustError::RejectedConfiguration
/// [unavailable]: FormatRustError::CargoUnavailable
/// [unrecognized]: FormatRustError::UnrecognizedReport
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

    if let Some(details) = report.warning() {
        return Err(FormatRustError::RejectedConfiguration {
            details: details.clone(),
        });
    }

    match report.exit() {
        Exit::Failed if report.listed().is_empty() && report.invalid().is_empty() => {
            Err(FormatRustError::UnrecognizedReport {
                root: root.directory().clone(),
                stderr: report.stderr().clone(),
            })
        }
        Exit::Failed | Exit::Succeeded => Ok(report),
    }
}

/// Returns the outcome of a run from what it found and what it repaired
///
/// A run without findings and without repairs passed. A run that repaired
/// everything changed the project. A run with a finding failed, whether it
/// repaired something on the way or not.
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
fn summary(roots: usize) -> Summary {
    if roots == 1 {
        Summary::new("checked 1 workspace")
    } else {
        Summary::new(format!("checked {roots} workspaces"))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::{Args, ArgsValues, ArgumentValue, argument_name};

    use super::*;

    /// Returns a finding at the level of the project, for the tests of the
    /// outcome
    fn problem() -> Finding {
        Finding::builder()
            .message("a problem")
            .location(Location::Project)
            .build()
    }

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    /// Returns the arguments of a run that asks for a repair
    fn fixing() -> FormatRustArgs {
        let values = ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);

        FormatRustArgs::from_values(&values).unwrap()
    }

    #[test]
    fn access_with_fix_is_write() {
        let args = fixing();

        let access = FormatRust.access(&context(), &args);

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn access_without_fix_is_read() {
        let args = FormatRustArgs::default();

        let access = FormatRust.access(&context(), &args);

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn format_rust_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<FormatRust>();
    }

    #[test]
    fn outcome_with_a_finding_and_a_repair_fails() {
        let outcome = outcome(vec![problem()], vec![problem()], 1);

        assert!(
            matches!(outcome, Outcome::Failed { .. }),
            "expected a failure, got {outcome:?}"
        );
    }

    #[test]
    fn outcome_with_only_repairs_reports_the_change() {
        let outcome = outcome(Vec::new(), vec![problem()], 1);

        assert!(
            matches!(outcome, Outcome::Changed { .. }),
            "expected a change, got {outcome:?}"
        );
    }

    #[test]
    fn outcome_without_findings_or_repairs_passes() {
        let outcome = outcome(Vec::new(), Vec::new(), 1);

        assert!(
            matches!(outcome, Outcome::Passed { .. }),
            "expected a pass, got {outcome:?}"
        );
    }

    #[test]
    fn summary_of_one_workspace_says_so() {
        let summary = summary(1);

        assert_eq!(summary.get(), "checked 1 workspace");
    }

    #[test]
    fn summary_of_two_workspaces_counts_them() {
        let summary = summary(2);

        assert_eq!(summary.get(), "checked 2 workspaces");
    }
}
