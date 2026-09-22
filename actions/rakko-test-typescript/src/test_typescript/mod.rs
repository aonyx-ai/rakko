//! The action that runs the tests of a Node project
//!
//! This module holds the action and the error that stops a run. The action
//! wraps the test runner of Node as a subprocess: Node finds the test files,
//! runs them, and reports what every test did, and the action translates that
//! into an outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Name, Outcome, ProjectRoot, SkipReason, Summary, action_name,
};

pub use self::error::TestTypeScriptError;
use crate::node::Node;
use crate::node::report::Report;

/// The reason of a run whose node found no test
const NO_TEST: &str = "node found no test in the project";

/// The action that runs the tests of a Node project
///
/// The action wraps the [test runner] that Node carries: Node walks the
/// project for its test files, runs each of them, and reports what every test
/// did. The node that runs is the one that [mise] installed for the project,
/// at the version that the project pinned, and the action installs nothing.
///
/// A run only reports, and it takes no argument. A test that failed becomes a
/// finding that names the test and carries what Node said, at the place where
/// the project declares the test. A test that holds tests reports no finding
/// of its own when one of those failed, because that one reports it already.
///
/// The action applies to a project that holds a test, and a run whose Node
/// found none skips visibly. A run stops with an error when mise reports no
/// node, when node reports no failure and ends without success, and when node
/// writes a report that the action cannot read.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_test_typescript::TestTypeScript;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(TestTypeScript)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [test runner]: https://nodejs.org/api/test.html
#[derive(Copy, Clone, Debug, Default)]
pub struct TestTypeScript;

impl Action for TestTypeScript {
    // testtypescript[impl args.none]
    type Args = ();

    // testtypescript[impl name]
    fn name(&self) -> Name {
        action_name!("test-typescript")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // testtypescript[impl tool.missing]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run resolves node, tests the project with it, and turns the report into
/// an outcome. An error that this function returns stops the run, and the
/// caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool, or the run of node that left no answer.
async fn drive(context: &Context) -> Result<Outcome, TestTypeScriptError> {
    // testtypescript[impl tool.missing]
    // testtypescript[impl tool.node]
    let node = Node::resolve(context.root().clone())
        .await
        .map_err(|source| TestTypeScriptError::UnresolvedTool { source })?;

    // testtypescript[impl run.read]
    let report = node.observe().await?;

    Ok(outcome(&report, context.root()))
}

/// Returns what a run reports about the given report of node
///
/// A run that examined no test at all is an action that does not apply, and
/// not a project whose tests all passed, so it comes before everything else.
// testtypescript[impl result.failed]
// testtypescript[impl skip.untested]
fn outcome(report: &Report, root: &ProjectRoot) -> Outcome {
    if report.tests() == 0 {
        return Outcome::Skipped {
            reason: SkipReason::new(NO_TEST),
        };
    }

    let findings: Vec<Finding> = report
        .failures()
        .iter()
        .map(|failure| failure.finding(root))
        .collect();

    if findings.is_empty() {
        // testtypescript[impl result.passed]
        return Outcome::Passed {
            summary: Some(summary(report.tests())),
        };
    }

    Outcome::Failed {
        findings,
        repairs: Vec::new(),
    }
}

/// Returns the summary that tells how many tests a run ran
// testtypescript[impl result.passed]
fn summary(tests: u64) -> Summary {
    let counted = match tests {
        1 => "1 test".to_owned(),
        count => format!("{count} tests"),
    };

    Summary::new(format!("ran {counted}"))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_test_utils::path;

    use super::*;
    use crate::failure::{Failure, Origin};

    /// Returns a failure of a test that the project declares in a file
    fn failure(name: &str) -> Failure {
        Failure::builder()
            .name(name)
            .origin(
                Origin::builder()
                    .path(path("/home/otter/project/src/add.test.ts"))
                    .line(5)
                    .column(1)
                    .build(),
            )
            .reason("it broke")
            .build()
    }

    /// Returns the report of a run that ran the given tests and had the given
    /// failures
    fn report(tests: u64, failures: Vec<Failure>) -> Report {
        Report::builder().failures(failures).tests(tests).build()
    }

    /// The root that the failures of a test belong to
    fn root() -> ProjectRoot {
        ProjectRoot::new(path("/home/otter/project"))
    }

    // testtypescript[verify result.failed]
    #[test]
    fn outcome_of_a_run_with_a_failure_fails() {
        let outcome = outcome(&report(3, vec![failure("counts")]), &root());

        let Outcome::Failed { findings, .. } = &outcome else {
            panic!("expected the run to fail, got {outcome:?}");
        };
        assert_eq!(findings.len(), 1);
    }

    // testtypescript[verify result.passed]
    #[test]
    fn outcome_of_a_run_without_a_failure_passes() {
        let outcome = outcome(&report(3, Vec::new()), &root());

        assert!(
            matches!(outcome, Outcome::Passed { .. }),
            "expected the run to pass, got {outcome:?}"
        );
    }

    // testtypescript[verify result.passed]
    #[test]
    fn outcome_of_a_run_without_a_failure_says_how_many_tests_ran() {
        let outcome = outcome(&report(3, Vec::new()), &root());

        let Outcome::Passed { summary } = &outcome else {
            panic!("expected the run to pass, got {outcome:?}");
        };
        assert_eq!(summary.as_ref().map(Summary::get), Some("ran 3 tests"));
    }

    // A summary that reads "ran 1 tests" tells a reader that nobody read it.
    #[test]
    fn summary_of_a_run_that_ran_one_test_names_it_in_the_singular() {
        let summary = summary(1);

        assert_eq!(summary.get(), "ran 1 test");
    }

    // testtypescript[verify skip.untested]
    #[test]
    fn outcome_of_a_run_that_ran_no_test_skips() {
        let outcome = outcome(&report(0, Vec::new()), &root());

        let Outcome::Skipped { reason } = &outcome else {
            panic!("expected the run to skip, got {outcome:?}");
        };
        assert_eq!(reason.get(), "node found no test in the project");
    }
}
