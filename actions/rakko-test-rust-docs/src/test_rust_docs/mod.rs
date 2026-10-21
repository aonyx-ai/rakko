//! The action that runs the examples in the documentation of a project
//!
//! This module holds the action and the error that stops a run. The action
//! looks at the project, resolves cargo, discovers the workspaces, and runs
//! the examples of every workspace whose documentation cargo can test. What
//! cargo and the test harness reported becomes the outcome of the run.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Access, Action, Context, Finding, Name, Outcome, SkipReason, Summary, action_name,
};
use rakko_cargo::lockfile::{self, LOCKED};
use rakko_cargo::{Cargo, CargoReport, CargoRoot, Documentation};
use rakko_tool::Execution;

pub use self::error::TestRustDocsError;
use crate::report::DoctestReport;

/// The reason of a run whose cargo discovered no workspace
const NO_WORKSPACE: &str = "cargo discovered no workspace in the project";

/// The reason of a run whose workspaces hold no library
const NO_LIBRARY: &str = "no workspace of the project holds a library";

/// The arguments that ask cargo to run the examples of a workspace
///
/// Every package of the workspace answers, with every feature, so that an
/// example behind a feature runs as well. Cargo stops at the first package
/// whose examples fail unless a run asks it to continue, and a run that
/// stopped there would leave the packages behind that one unexamined.
///
/// Cargo builds the versions that the lockfile holds and does not write the
/// lockfile, because the action only examines the project.
///
/// The report of cargo arrives as JSON, because the action reads it as data.
/// The format selects the presentation of the report and not the behavior of
/// the tool.
const TEST: [&str; 7] = [
    "test",
    LOCKED,
    "--workspace",
    "--doc",
    "--all-features",
    "--no-fail-fast",
    "--message-format=json",
];

/// The action that runs the examples in the documentation of a project
///
/// The action wraps cargo: it builds the examples of every package of a
/// workspace with every feature, against the library that documents them,
/// and runs them, so a run agrees with a contributor that runs
/// `cargo test --doc` bare. The cargo that runs is the one that [mise]
/// installed for the project, at the version that the project pinned, and
/// the action installs nothing.
///
/// The action runs beside the action that runs the tests, because [nextest]
/// leaves the examples out. A run only reports, and it takes no argument. It
/// tests every workspace of the project that holds a library, because the
/// harness of a project is a package of its own, and cargo can test the
/// documentation of a library and of nothing else. It builds the versions
/// that the lockfile of a workspace holds, and it leaves the lockfile as it
/// is.
///
/// An example that failed becomes a finding at the line that documents it,
/// which carries the message of the panic for an example that ran, and what
/// the compiler wrote for an example that it refused. A build that does not
/// finish becomes findings from the diagnostics of the compiler. A library
/// without an example ran no example, and the run says so instead of
/// failing.
///
/// A project whose cargo discovers no workspace skips visibly, and so does a
/// project whose workspaces hold no library. A run stops with an error when
/// mise reports no cargo, when the workspaces of the project cannot be
/// discovered, when the lockfile of a workspace needs an update, and when a
/// run of cargo leaves the action without an answer.
///
/// A run declares a read of the project. It holds cargo to the lockfile of each
/// workspace, and cargo writes what it builds into the `target` directory,
/// which cargo locks against parallel use itself. The declaration covers the
/// action and cargo, and not the tests, build scripts, and procedural macros of
/// the project, which can write files of their own. A caller that orders runs
/// by their declarations runs it after the actions that write.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_test_rust_docs::TestRustDocs;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(TestRustDocs)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [nextest]: https://nexte.st
#[derive(Copy, Clone, Debug, Default)]
pub struct TestRustDocs;

impl Action for TestRustDocs {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("test-rust-docs")
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

/// What one workspace of a run reported
struct Examined {
    /// The problems that the workspace reported
    findings: Vec<Finding>,

    /// How many examples of the workspace ran
    ran: u64,
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves cargo, discovers the workspaces,
/// and runs the examples of every workspace that holds a library. An error
/// that this function returns stops the run, and the caller reports it in
/// the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, the discovery of the workspaces, a run of cargo, or the reading
/// of a report.
async fn drive(context: &Context) -> Result<Outcome, TestRustDocsError> {
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| TestRustDocsError::UnresolvedTool { source })?;

    let roots = cargo
        .roots()
        .await
        .map_err(|source| TestRustDocsError::UndiscoveredRoots { source })?;

    if roots.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_WORKSPACE),
        });
    }

    let mut findings: Vec<Finding> = Vec::new();
    let mut tested = 0;
    let mut ran = 0;

    for root in &roots {
        if root.documentation() == Documentation::Untestable {
            continue;
        }

        tested += 1;

        let examined = examine(&cargo, root, context).await?;
        findings.extend(examined.findings);
        ran += examined.ran;
    }

    if tested == 0 {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_LIBRARY),
        });
    }

    if findings.is_empty() {
        Ok(Outcome::Passed {
            summary: Some(summary(ran, tested)),
        })
    } else {
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Runs the examples of one workspace and returns what it reported
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run,
/// [`StaleLockfile`][stale] when cargo refused to build because the
/// lockfile needs an update, [`UnreadableDiagnostics`][diagnostics] and
/// [`UnreadableReport`][report] when the action cannot read what cargo or
/// the harness wrote, and [`UnrecognizedReport`][unrecognized] when the run
/// leaves the action without an answer.
///
/// [diagnostics]: TestRustDocsError::UnreadableDiagnostics
/// [report]: TestRustDocsError::UnreadableReport
/// [stale]: TestRustDocsError::StaleLockfile
/// [unavailable]: TestRustDocsError::CargoUnavailable
/// [unrecognized]: TestRustDocsError::UnrecognizedReport
async fn examine(
    cargo: &Cargo,
    root: &CargoRoot,
    context: &Context,
) -> Result<Examined, TestRustDocsError> {
    let execution = cargo
        .invocation(root)
        .args(TEST)
        .run()
        .await
        .map_err(|source| TestRustDocsError::CargoUnavailable { source })?;

    if lockfile::refused(&execution.stderr().to_string_lossy()) {
        return Err(TestRustDocsError::StaleLockfile {
            root: root.directory().clone(),
        });
    }

    let stdout = execution.stdout().to_string_lossy();
    let build =
        CargoReport::read(&stdout).map_err(|source| TestRustDocsError::UnreadableDiagnostics {
            root: root.directory().clone(),
            source,
        })?;
    let report =
        DoctestReport::read(&stdout).map_err(|source| TestRustDocsError::UnreadableReport {
            root: root.directory().clone(),
            source,
        })?;

    if !recognized(&report, &build, &execution) {
        return Err(TestRustDocsError::UnrecognizedReport {
            root: root.directory().clone(),
            stderr: execution.stderr().to_string_lossy().into_owned(),
        });
    }

    let mut findings: Vec<Finding> = build
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.finding(root, context.root()))
        .collect();

    findings.extend(
        report
            .failures()
            .iter()
            .map(|failure| failure.finding(root, context.root())),
    );

    Ok(Examined {
        findings,
        ran: report.ran(),
    })
}

/// Returns whether the action can answer from what a run reported
///
/// A run that ended with success and left no summary of the harness examined
/// nothing that the action can report. A run that ended without success and
/// named neither a failed example nor a diagnostic failed for a reason that
/// the action cannot read, such as a build script that ended badly. An
/// answer built on either would hide problems behind a green result.
fn recognized(report: &DoctestReport, build: &CargoReport, execution: &Execution) -> bool {
    if execution.status().success() {
        report.summaries() > 0
    } else {
        !report.failures().is_empty() || !build.diagnostics().is_empty()
    }
}

/// Returns the summary that tells how many examples ran in how many
/// workspaces
fn summary(ran: u64, roots: usize) -> Summary {
    let examples = match ran {
        1 => "1 example".to_owned(),
        count => format!("{count} examples"),
    };
    let workspaces = match roots {
        1 => "1 workspace".to_owned(),
        count => format!("{count} workspaces"),
    };

    Summary::new(format!("ran {examples} in {workspaces}"))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns a context whose root names no project, because a declaration
    /// of access reads no file
    fn context() -> Context {
        Context::builder().root("/tmp/project").build()
    }

    #[test]
    fn access_is_read() {
        let access = TestRustDocs.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn summary_of_many_examples_counts_them() {
        let summary = summary(42, 2);

        assert_eq!(summary.get(), "ran 42 examples in 2 workspaces");
    }

    #[test]
    fn summary_of_no_example_counts_zero() {
        let summary = summary(0, 2);

        assert_eq!(summary.get(), "ran 0 examples in 2 workspaces");
    }

    #[test]
    fn summary_of_one_example_in_one_workspace_says_so() {
        let summary = summary(1, 1);

        assert_eq!(summary.get(), "ran 1 example in 1 workspace");
    }
}
