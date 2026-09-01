//! The action that runs the tests of a project
//!
//! This module holds the action, the error that stops a run, and the reading
//! of what nextest reports. The action wraps cargo as a subprocess: cargo
//! builds the targets, nextest reads its own configuration and runs the
//! tests, and the action translates what both reported into an outcome.

/// The error that stops a run of the action
mod error;
/// What nextest reported about a run
mod report;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, Position, SkipReason, Summary, action_name,
};
use rakko_cargo::{Cargo, CargoReport, CargoRoot};
use rakko_tool::Execution;

pub use self::error::TestRustError;
pub use self::report::{NextestReport, Panic, TestFailure};

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

/// The arguments that ask nextest to run every test with every feature
///
/// The reports of nextest and of cargo arrive as JSON, because the action
/// reads them as data. The formats select the presentation of the reports
/// and not the behavior of the tools: how the tests run comes from the
/// configuration of the project alone.
const NEXTEST: [&str; 8] = [
    "nextest",
    "run",
    "--all-targets",
    "--all-features",
    "--message-format",
    "libtest-json-plus",
    "--cargo-message-format",
    "json",
];

/// The variable that gives consent to the experimental report of nextest
const CONSENT_VARIABLE: &str = "NEXTEST_EXPERIMENTAL_LIBTEST_JSON";

/// The value that gives consent
const CONSENT_VALUE: &str = "1";

/// The exit status of a nextest run that found no test to run
///
/// Nextest documents the status, and it is the one signal for a workspace
/// without tests, which is not a failure of the project.
const NO_TESTS: i32 = 4;

/// The action that runs the tests of a project
///
/// The action wraps [nextest]: cargo builds every target of every package
/// with every feature, and nextest reads its own configuration and runs the
/// tests, so a run agrees with a contributor that runs nextest bare. The
/// cargo that runs is the one that [mise] installed for the project, at the
/// version that the project pinned, and the action installs nothing.
///
/// A run only reports, and it takes no argument. It tests every workspace of
/// the project, because the harness of a project is a package of its own. A
/// test that failed becomes a finding that names the test and carries the
/// message of the panic, at the position where the test panicked. A build
/// that does not finish becomes findings from the diagnostics of the
/// compiler. A workspace without a test ran no test, and the run says so
/// instead of failing.
///
/// The action applies to a project that holds a manifest of cargo, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// cargo, when the workspaces of the project cannot be discovered, and when
/// nextest writes a report that the action does not recognize.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_test_rust::TestRust;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(TestRust)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [nextest]: https://nexte.st
#[derive(Copy, Clone, Debug, Default)]
pub struct TestRust;

impl Action for TestRust {
    // testrust[impl args.none]
    type Args = ();

    // testrust[impl name]
    fn name(&self) -> Name {
        action_name!("test-rust")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // testrust[impl roots.error]
            // testrust[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// What one run of nextest at one root reported
///
/// The two reports come from one output stream, and the count travels with
/// them, so a caller sums the tests over the roots.
struct Observation {
    /// The findings of the run: the tests that failed, or the diagnostics of
    /// a build that did not finish
    findings: Vec<Finding>,

    /// How many tests ran
    ran: u64,
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves cargo, discovers the workspaces,
/// and tests each of them. An error that this function returns stops the
/// run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of
/// the tool, the discovery of the workspaces, a nextest run, or the reading
/// of a report.
async fn drive(context: &Context) -> Result<Outcome, TestRustError> {
    // testrust[impl skip.git]
    // testrust[impl skip.links]
    // testrust[impl skip.missing]
    // testrust[impl skip.target]
    if !Cargo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MANIFEST),
        });
    }

    // testrust[impl tool.cargo]
    // testrust[impl tool.missing]
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| TestRustError::UnresolvedTool { source })?;

    // testrust[impl roots.error]
    let roots = cargo
        .roots()
        .await
        .map_err(|source| TestRustError::UndiscoveredRoots { source })?;

    let mut findings = Vec::new();
    let mut ran = 0;

    // testrust[impl roots.all]
    for root in &roots {
        let observation = test(&cargo, root, context).await?;
        findings.extend(observation.findings);
        ran += observation.ran;
    }

    if findings.is_empty() {
        // testrust[impl run.none]
        // testrust[impl run.passed]
        Ok(Outcome::Passed {
            summary: Some(summary(ran, roots.len())),
        })
    } else {
        // testrust[impl run.build]
        // testrust[impl run.failed]
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Returns the finding that reports one test that failed
///
/// The finding names the test and carries the message of the panic, at the
/// position where the test panicked. A test whose output names no panic,
/// and a test that panicked in a file outside the project, get a finding at
/// the level of the project.
// testrust[impl run.failed]
// testrust[impl run.position]
fn finding(failure: &TestFailure, root: &CargoRoot, context: &Context) -> Finding {
    let Some(panic) = failure.panic() else {
        return Finding::builder()
            .message(format!("test `{}` failed", failure.name()))
            .location(Location::Project)
            .build();
    };

    let message = match panic.message() {
        Some(message) => format!("test `{}` failed: {message}", failure.name()),
        None => format!("test `{}` panicked", failure.name()),
    };

    let location = match root.relative_path(panic.path(), context.root()) {
        Some(path) => Location::Position {
            path,
            position: Position::builder()
                .line(panic.line())
                .column(panic.column())
                .build(),
        },
        None => Location::Project,
    };

    Finding::builder()
        .message(message)
        .location(location)
        .build()
}

/// Returns whether the action can answer from the reports of a run
///
/// A run that ended with success answered. A run that ended without success
/// answered when it named a test that failed, a diagnostic of the build, or
/// the absence of tests. Everything else is a report that the action cannot
/// read, and an answer built on it would hide failures behind a green
/// result.
// testrust[impl run.none]
// testrust[impl run.unrecognized]
fn recognized(nextest: &NextestReport, cargo: &CargoReport, execution: &Execution) -> bool {
    execution.status().success()
        || !nextest.failures().is_empty()
        || !cargo.diagnostics().is_empty()
        || execution.status().code() == Some(NO_TESTS)
}

/// Returns the summary that tells how many tests ran in how many workspaces
// testrust[impl run.passed]
fn summary(ran: u64, roots: usize) -> Summary {
    let tests = match ran {
        1 => "1 test".to_owned(),
        count => format!("{count} tests"),
    };
    let workspaces = match roots {
        1 => "1 workspace".to_owned(),
        count => format!("{count} workspaces"),
    };

    Summary::new(format!("ran {tests} in {workspaces}"))
}

/// Tests one workspace of the project and returns what the run reported
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run, and
/// [`UnrecognizedReport`][unrecognized] when nextest writes a report that
/// the action cannot answer from.
///
/// [unavailable]: TestRustError::CargoUnavailable
/// [unrecognized]: TestRustError::UnrecognizedReport
// testrust[impl run.build]
// testrust[impl run.consent]
// testrust[impl run.operation]
// testrust[impl run.read]
async fn test(
    cargo: &Cargo,
    root: &CargoRoot,
    context: &Context,
) -> Result<Observation, TestRustError> {
    let execution = cargo
        .invocation(root)
        .env(CONSENT_VARIABLE, CONSENT_VALUE)
        .args(NEXTEST)
        .run()
        .await
        .map_err(|source| TestRustError::CargoUnavailable { source })?;

    let stdout = execution.stdout().to_string_lossy();
    let nextest = NextestReport::read(&stdout);
    let diagnostics = CargoReport::read(&stdout);

    // testrust[impl run.unrecognized]
    if !recognized(&nextest, &diagnostics, &execution) {
        return Err(TestRustError::UnrecognizedReport {
            root: root.directory().clone(),
            stderr: execution.stderr().to_string_lossy().into_owned(),
        });
    }

    let mut findings: Vec<Finding> = diagnostics
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.finding(root, context.root()))
        .collect();
    findings.extend(
        nextest
            .failures()
            .iter()
            .map(|failure| finding(failure, root, context)),
    );

    Ok(Observation {
        findings,
        ran: nextest.ran(),
    })
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // testrust[verify run.passed]
    #[test]
    fn summary_of_one_test_in_one_workspace_says_so() {
        let summary = summary(1, 1);

        assert_eq!(summary.get(), "ran 1 test in 1 workspace");
    }

    // testrust[verify run.none]
    #[test]
    fn summary_of_no_test_counts_zero() {
        let summary = summary(0, 2);

        assert_eq!(summary.get(), "ran 0 tests in 2 workspaces");
    }

    // testrust[verify run.passed]
    #[test]
    fn summary_of_many_tests_counts_them() {
        let summary = summary(354, 2);

        assert_eq!(summary.get(), "ran 354 tests in 2 workspaces");
    }
}
