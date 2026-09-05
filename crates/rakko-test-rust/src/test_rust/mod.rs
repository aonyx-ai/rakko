//! The action that runs the tests of a project
//!
//! This module holds the action and the error that stops a run. The action
//! looks at the project, resolves cargo, discovers the workspaces, and hands
//! each of them to the machinery that runs nextest. What that machinery
//! reports becomes the outcome of the run.

/// The error that stops a run of the action
mod error;

use rakko_action::{Action, Context, Finding, Name, Outcome, SkipReason, Summary, action_name};
use rakko_cargo::Cargo;
use rakko_nextest::Nextest;

pub use self::error::TestRustError;

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

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
/// nextest writes a report that the run cannot answer from.
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
            // testrust[impl run.error]
            // testrust[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
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
/// the tool, the discovery of the workspaces, or a run of nextest that left
/// no answer.
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

    let nextest = Nextest::new(cargo);
    let mut findings: Vec<Finding> = Vec::new();
    let mut ran = 0;

    // testrust[impl roots.all]
    for root in &roots {
        // testrust[impl run.error]
        // testrust[impl run.read]
        let observation = nextest.observe(root, context.root()).await?;

        findings.extend(observation.findings().iter().cloned());
        ran += observation.ran();
    }

    if findings.is_empty() {
        // testrust[impl run.none]
        // testrust[impl run.passed]
        Ok(Outcome::Passed {
            summary: Some(summary(ran, roots.len())),
        })
    } else {
        // testrust[impl run.build+2]
        // testrust[impl run.failed+2]
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
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

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // testrust[verify run.passed]
    #[test]
    fn summary_of_many_tests_counts_them() {
        let summary = summary(354, 2);

        assert_eq!(summary.get(), "ran 354 tests in 2 workspaces");
    }

    // testrust[verify run.none]
    #[test]
    fn summary_of_no_test_counts_zero() {
        let summary = summary(0, 2);

        assert_eq!(summary.get(), "ran 0 tests in 2 workspaces");
    }

    // testrust[verify run.passed]
    #[test]
    fn summary_of_one_test_in_one_workspace_says_so() {
        let summary = summary(1, 1);

        assert_eq!(summary.get(), "ran 1 test in 1 workspace");
    }
}
