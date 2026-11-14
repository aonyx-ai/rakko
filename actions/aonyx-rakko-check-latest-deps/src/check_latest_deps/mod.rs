//! The action that checks a project against the newest versions of its
//! dependencies
//!
//! This module holds the action and the error that stops a run. The action
//! looks at the project, resolves cargo, discovers the workspaces, and stands
//! up a copy of the project to work in. In that copy it asks cargo for the
//! newest versions that the manifests allow, and then hands each workspace to
//! the machinery that runs nextest. What the two runs reported becomes the
//! outcome.

mod error;

use aonyx_rakko_cargo::{Cargo, CargoRoot};
use aonyx_rakko_nextest::{Lockfile, Nextest};
use aonyx_rakko_worktree::Worktree;
use rakko::{
    Access, Action, Context, Documented, Finding, Location, Name, Outcome, ProjectRoot, SkipReason,
    Summary, ToolOutput, ToolRequirement, action_name,
};
use rakko_tool::{Execution, Launch, ShownStreams};

pub use self::error::CheckLatestDepsError;

/// The reason of a run whose cargo discovered no workspace
const NO_WORKSPACE: &str = "cargo discovered no workspace in the project";

/// The argument that asks cargo for the newest versions that the manifests
/// allow
///
/// Cargo resolves the workspace that it runs in and writes the lockfile of
/// that workspace. The command names no package and no version, so every
/// dependency moves as far as its requirement allows.
const UPDATE: &str = "update";

/// The text that introduces what cargo wrote about an update it could not
/// finish
const NOT_UPDATED: &str = "failed to update the dependencies to their newest versions:";

/// The details of an update that ended without success and wrote nothing
const NO_DIAGNOSIS: &str = "cargo wrote nothing about it";

/// Checks a project against the newest versions of its dependencies
///
/// A manifest states a floor for each dependency, and whoever reads the floor
/// reads a promise: every later version of that dependency works as well. The
/// action resolves the newest versions that the manifests allow. It runs the
/// tests of the project against them, so the promise is confirmed instead of
/// assumed. The cargo that runs is the one of the stable toolchain that the
/// project pins, and [nextest] runs at the version that the action declares.
/// [Mise] installed both.
///
/// A run only reports, and it takes no argument. It covers every workspace of
/// the project, because the harness of a project is a package of its own. An
/// update that cargo could not finish becomes a finding at the manifest of its
/// workspace, and such a run runs no test. A test that failed becomes a
/// finding that names the test. A build that does not finish becomes findings
/// from the diagnostics of the compiler.
///
/// The whole run happens in a disposable copy of the project. So the checkout of
/// a contributor keeps its lockfiles and its build directory, whatever the run
/// finds. A tree with changes in it is copied as it is, so a contributor reads
/// the answer for the manifest that they are editing.
///
/// A project whose cargo discovers no workspace skips visibly, and says so. A
/// run stops with an error when the project pins no stable Rust toolchain that
/// the action accepts. It stops when mise reports no nextest at the declared
/// version, the project is no git repository, or the copy cannot be created. It
/// stops when the workspaces of the project cannot be discovered, or when
/// nextest writes a report that the run cannot answer from.
///
/// A run declares a read of the project, because it builds in a temporary
/// worktree outside the project. Git records that worktree only under `.git`,
/// and a change there does not count as a write. A caller that orders runs by
/// their declarations runs it after the actions that write.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use aonyx_rakko_check_latest_deps::CheckLatestDeps;
///
/// let actions = Bundle::default().with(CheckLatestDeps);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [nextest]: https://nexte.st
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct CheckLatestDeps;

impl Action for CheckLatestDeps {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("check-latest-deps")
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
/// and creates the copy that the work happens in. It updates every workspace
/// of the copy, and it tests them when every update succeeded. An error that
/// this function returns stops the run, and the caller reports it in the
/// outcome.
///
/// The copy lives until this function returns, and the drop takes it with it.
/// So a run that ends with an answer and a run that stops with an error clear it
/// alike.
///
/// # Errors
///
/// Returns the error of the step that could not finish. It may be from tool
/// resolution, workspace discovery, copy creation, a cargo run, or a nextest
/// run that left no answer.
async fn drive(context: &Context) -> Result<Outcome, CheckLatestDepsError> {
    let cargo = Cargo::resolve(context)
        .await
        .map_err(|source| CheckLatestDepsError::UnresolvedCargo { source })?;

    let roots = cargo
        .roots()
        .await
        .map_err(|source| CheckLatestDepsError::UndiscoveredRoots { source })?;

    if roots.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_WORKSPACE),
        });
    }

    let worktree = Worktree::create(context.root().clone())
        .await
        .map_err(|source| CheckLatestDepsError::CopyUnavailable { source })?;

    let copies = copies(&worktree, &roots)?;
    let mut findings = Vec::new();

    for root in &copies {
        findings.extend(update(&cargo, root, worktree.root(), context.tool_output()).await?);
    }

    if !findings.is_empty() {
        return Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        });
    }

    let nextest = Nextest::resolve()
        .cargo(cargo)
        .lockfile(Lockfile::Locked)
        .context(context)
        .call()
        .await
        .map_err(|source| CheckLatestDepsError::UnresolvedNextest { source })?;
    let mut ran = 0;

    for root in &copies {
        let observation = nextest.observe(root, worktree.root()).await?;

        findings.extend(observation.findings().iter().cloned());
        ran += observation.ran();
    }

    if findings.is_empty() {
        Ok(Outcome::Passed {
            summary: Some(summary(copies.len(), ran)),
        })
    } else {
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Returns the workspace roots as the copy holds them
///
/// The copy holds the tree of the project at its own root. So a root of the
/// project keeps the path that it has below the project root. The run works
/// on the roots of the copy from here on. It names the paths of its findings
/// against the root of the copy, which gives them the paths that the project
/// has.
///
/// # Errors
///
/// Returns [`ForeignRoot`][foreign] when the copy holds no directory for a
/// root of the project, which is a project that moved while the run was on.
///
/// [foreign]: CheckLatestDepsError::ForeignRoot
fn copies(
    worktree: &Worktree,
    roots: &[CargoRoot],
) -> Result<Vec<CargoRoot>, CheckLatestDepsError> {
    roots
        .iter()
        .map(|root| {
            worktree
                .path_of(root.directory())
                .map(|directory| {
                    CargoRoot::builder()
                        .directory(directory)
                        .documentation(root.documentation())
                        .packages(root.packages().clone())
                        .maybe_rust_version(root.rust_version().clone())
                        .build()
                })
                .ok_or_else(|| CheckLatestDepsError::ForeignRoot {
                    root: root.directory().clone(),
                })
        })
        .collect()
}

/// Returns what cargo wrote about an update that it could not finish
///
/// Cargo writes its diagnosis to the standard error stream. So whoever reads the
/// finding reads the answer of cargo instead of a sentence that Rakko wrote
/// about it.
fn details(execution: &Execution) -> String {
    let diagnosis = execution.stderr().to_string_lossy();
    let text = diagnosis.trim();

    if text.is_empty() {
        NO_DIAGNOSIS.to_owned()
    } else {
        text.to_owned()
    }
}

/// Returns the finding of an update that cargo could not finish
///
/// The message carries what cargo wrote, because cargo names the dependency
/// that it could not resolve and the reason. Nothing that Rakko could write
/// about it would
/// say more. The finding belongs to the manifest of the root, which is the
/// file that states the requirements that cargo read.
fn finding(root: &CargoRoot, project: &ProjectRoot, execution: &Execution) -> Finding {
    let message = format!("{NOT_UPDATED} {}", details(execution));

    match root.relative_path(&root.manifest(), project) {
        Some(path) => Finding::builder()
            .message(message)
            .location(Location::File { path })
            .build(),
        None => Finding::builder()
            .message(message)
            .location(Location::Project)
            .build(),
    }
}

/// Returns the summary that tells how many workspaces the run updated and how
/// many tests it ran
fn summary(roots: usize, ran: u64) -> Summary {
    let workspaces = match roots {
        1 => "1 workspace".to_owned(),
        count => format!("{count} workspaces"),
    };
    let tests = match ran {
        1 => "1 test".to_owned(),
        count => format!("{count} tests"),
    };

    Summary::new(format!("updated {workspaces} and ran {tests}"))
}

/// Resolves the newest versions that the manifests of one workspace allow
///
/// The update runs in the directory of the root, which lies in the copy. So the
/// lockfile that it rewrites is the lockfile of the copy. An update that ends
/// without success is an answer about the project, and it comes back as the
/// finding that reports it.
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run.
///
/// [unavailable]: CheckLatestDepsError::CargoUnavailable
async fn update(
    cargo: &Cargo,
    root: &CargoRoot,
    project: &ProjectRoot,
    output: &ToolOutput,
) -> Result<Option<Finding>, CheckLatestDepsError> {
    let execution = cargo
        .invocation(root)
        .arg(UPDATE)
        .launch_with(output, ShownStreams::Both)
        .await
        .map_err(|source| CheckLatestDepsError::CargoUnavailable { source })?;

    if execution.status().success() {
        return Ok(None);
    }

    Ok(Some(finding(root, project, &execution)))
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
        let access = CheckLatestDeps.access(&context(), &());

        assert_eq!(access, Access::Read);
    }

    #[test]
    fn check_latest_deps_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CheckLatestDeps>();
    }

    #[test]
    fn summary_of_many_tests_counts_them() {
        let summary = summary(2, 354);

        assert_eq!(summary.get(), "updated 2 workspaces and ran 354 tests");
    }

    #[test]
    fn summary_of_no_test_counts_zero() {
        let summary = summary(2, 0);

        assert_eq!(summary.get(), "updated 2 workspaces and ran 0 tests");
    }

    #[test]
    fn summary_of_one_test_in_one_workspace_says_so() {
        let summary = summary(1, 1);

        assert_eq!(summary.get(), "updated 1 workspace and ran 1 test");
    }
}
