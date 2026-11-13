//! The action that runs the tests of a project
//!
//! This module holds the action and the error that stops a run. The action
//! looks at the project, resolves cargo, discovers the workspaces, and hands
//! each of them to the machinery that runs nextest. What that machinery
//! reports becomes the outcome of the run.

mod error;

use aonyx_rakko_cargo::Cargo;
use aonyx_rakko_nextest::{Lockfile, Nextest};
use rakko::{
    Access, Action, Context, Documented, Finding, Name, Outcome, SkipReason, Summary,
    ToolRequirement, action_name,
};

pub use self::error::TestRustError;

/// The reason of a run whose cargo discovered no workspace
const NO_WORKSPACE: &str = "cargo discovered no workspace in the project";

/// Runs the tests of a project
///
/// The action wraps [nextest]. Cargo builds every target of every package
/// with every feature. Nextest reads its own configuration and runs the
/// tests, so a run agrees with a contributor that runs nextest bare. The
/// nextest that runs is the version that the action declares, which [mise]
/// installed, whatever version the project pins. The cargo that builds is
/// the one of the stable toolchain that the project pins. The action
/// installs nothing.
///
/// A run only reports, and it takes no argument. It tests every workspace of
/// the project, because the harness of a project is a package of its own. It
/// builds the versions that the lockfile of a workspace holds, and it leaves
/// the lockfile as it is. A
/// test that failed becomes a finding that names the test and carries the
/// message of the panic, at the position where the test panicked. A build
/// that does not finish becomes findings from the diagnostics of the
/// compiler. A workspace without a test ran no test, and the run says so
/// instead of failing.
///
/// A project whose cargo discovers no workspace skips visibly, and says so. A
/// run stops with an error when the project pins no stable Rust toolchain that
/// the action accepts. It stops when mise reports no nextest at the declared
/// version. It stops when the workspaces of the project cannot be discovered or
/// when the lockfile of a workspace needs an update. It stops when nextest
/// writes a report that the run cannot answer from.
///
/// A run declares a read of the project. It holds cargo to the lockfile of
/// each workspace. Cargo writes what it builds into the `target` directory,
/// which cargo locks against parallel use itself. The declaration covers the
/// action and cargo. It does not cover the tests, build scripts, and
/// procedural macros of the project, which can write files of their own. A
/// caller that orders runs by their declarations runs it after the actions
/// that write.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use aonyx_rakko_test_rust::TestRust;
///
/// let actions = Bundle::default().with(TestRust);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [nextest]: https://nexte.st
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct TestRust;

impl Action for TestRust {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("test-rust")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    fn tools(&self) -> Vec<ToolRequirement> {
        vec![Cargo::requirement(), Nextest::requirement()]
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
/// The run examines the project, resolves cargo, discovers the workspaces,
/// and tests each of them. An error that this function returns stops the
/// run, and the caller reports it in the outcome.
///
/// # Errors
///
/// Returns the error of the step that could not finish. The step is the
/// resolution of a tool, the discovery of the workspaces, or a run of nextest
/// that left no answer.
async fn drive(context: &Context) -> Result<Outcome, TestRustError> {
    let cargo = Cargo::resolve(context)
        .await
        .map_err(|source| TestRustError::UnresolvedCargo { source })?;

    let roots = cargo
        .roots()
        .await
        .map_err(|source| TestRustError::UndiscoveredRoots { source })?;

    if roots.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_WORKSPACE),
        });
    }

    let nextest = Nextest::resolve()
        .cargo(cargo)
        .lockfile(Lockfile::Locked)
        .context(context)
        .call()
        .await
        .map_err(|source| TestRustError::UnresolvedNextest { source })?;
    let mut findings: Vec<Finding> = Vec::new();
    let mut ran = 0;

    for root in &roots {
        let observation = nextest.observe(root, context.root()).await?;

        findings.extend(observation.findings().iter().cloned());
        ran += observation.ran();
    }

    if findings.is_empty() {
        Ok(Outcome::Passed {
            summary: Some(summary(ran, roots.len())),
        })
    } else {
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Returns the summary that tells how many tests ran in how many workspaces
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
        let access = TestRust.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn summary_of_many_tests_counts_them() {
        let summary = summary(354, 2);

        assert_eq!(summary.get(), "ran 354 tests in 2 workspaces");
    }

    #[test]
    fn summary_of_no_test_counts_zero() {
        let summary = summary(0, 2);

        assert_eq!(summary.get(), "ran 0 tests in 2 workspaces");
    }

    #[test]
    fn summary_of_one_test_in_one_workspace_says_so() {
        let summary = summary(1, 1);

        assert_eq!(summary.get(), "ran 1 test in 1 workspace");
    }

    #[test]
    fn test_rust_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<TestRust>();
    }
}
