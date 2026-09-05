//! The action that checks a project against the lowest versions of its
//! dependencies
//!
//! This module holds the action and the error that stops a run. The action
//! looks at the project, resolves cargo and the nightly toolchain, discovers
//! the workspaces, and stands up a copy of the project to work in. In that
//! copy it asks cargo for the lowest versions that the manifests allow, and
//! then hands each workspace to the machinery that runs nextest. What the two
//! runs reported becomes the outcome.

/// The error that stops a run of the action
mod error;

use rakko_action::{
    Action, Context, Finding, Location, Name, Outcome, ProjectRoot, SkipReason, Summary,
    action_name,
};
use rakko_cargo::{Cargo, CargoRoot, Channel, Toolchain};
use rakko_nextest::{Lockfile, Nextest};
use rakko_tool::Execution;
use rakko_worktree::Worktree;

pub use self::error::CheckMinimalDepsError;

/// The reason of a run that found no manifest
const NO_MANIFEST: &str = "the project holds no file named Cargo.toml";

/// The channel that the resolution of the floors needs
///
/// The option below is unstable, and a stable cargo refuses it and resolves
/// nothing at all.
const NIGHTLY: &str = "nightly";

/// The arguments that ask cargo for the lowest versions that the manifests
/// allow
///
/// Cargo resolves the workspace that it runs in and writes the lockfile of
/// that workspace. The command names no package and no version, so every
/// direct dependency moves down to its floor. The option reaches the
/// dependencies that the manifests name and not the dependencies of those,
/// because a floor of another project is a promise that its author makes.
const UPDATE: [&str; 3] = ["update", "-Z", "direct-minimal-versions"];

/// The text that introduces what cargo wrote about an update it could not
/// finish
const NOT_UPDATED: &str = "failed to update the dependencies to their lowest versions:";

/// The details of an update that ended without success and wrote nothing
const NO_DIAGNOSIS: &str = "cargo wrote nothing about it";

/// The action that checks a project against the lowest versions of its
/// dependencies
///
/// A manifest states a floor for each dependency, and whoever reads the floor
/// reads a promise: that version of the dependency works. Nothing else in the
/// life of a project ever asks, because every ordinary build reaches for the
/// newest version that the floor allows. The action resolves the floors and
/// runs the tests of the project against them, so the promise is confirmed
/// instead of assumed. The cargo that runs is the one that [mise] installed
/// for the project, and [nextest] is the plugin that it carries.
///
/// The resolution runs on the nightly toolchain that the project pins,
/// because the option that asks cargo for the floors is unstable. The tests
/// run on the toolchain that the project builds with: they answer for the
/// versions of the resolution and not for the compiler that resolved them.
///
/// A run only reports, and it takes no argument. It covers every workspace of
/// the project, because the harness of a project is a package of its own. An
/// update that cargo could not finish becomes a finding at the manifest of its
/// workspace, and such a run runs no test. A test that failed becomes a
/// finding that names the test, and a build that does not finish becomes
/// findings from the diagnostics of the compiler.
///
/// The whole run happens in a disposable copy of the project, so the checkout
/// of a contributor keeps its lockfiles and its build directory, whatever the
/// run finds. A tree with changes in it is copied as it is, so a contributor
/// reads the answer for the manifest that they are editing.
///
/// The action applies to a project that holds a manifest of cargo, and it
/// skips visibly otherwise. A run stops with an error when mise reports no
/// cargo and when it reports no nightly toolchain, when the project is no git
/// repository, when the copy cannot be created, when the workspaces of the
/// project cannot be discovered, and when nextest writes a report that the run
/// cannot answer from.
///
/// # Examples
///
/// A harness mounts the action:
///
/// ```
/// use rakko_action::ErasedAction;
/// use rakko_check_minimal_deps::CheckMinimalDeps;
///
/// let actions: Vec<Box<dyn ErasedAction>> = vec![Box::new(CheckMinimalDeps)];
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [nextest]: https://nexte.st
#[derive(Copy, Clone, Debug, Default)]
pub struct CheckMinimalDeps;

impl Action for CheckMinimalDeps {
    // checkminimaldeps[impl args.none]
    type Args = ();

    // checkminimaldeps[impl name]
    fn name(&self) -> Name {
        action_name!("check-minimal-deps")
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context).await {
            Ok(outcome) => outcome,
            // checkminimaldeps[impl copy.unavailable]
            // checkminimaldeps[impl roots.error]
            // checkminimaldeps[impl tests.error]
            // checkminimaldeps[impl tool.missing]
            // checkminimaldeps[impl tool.unpinned]
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context
///
/// The run examines the project, resolves cargo and the nightly toolchain,
/// discovers the workspaces, and creates the copy that the work happens in.
/// It updates every workspace of the copy, and it tests them when every
/// update succeeded. An error that this function returns stops the run, and
/// the caller reports it in the outcome.
///
/// The copy lives until this function returns, and the drop takes it with it,
/// so a run that ends with an answer and a run that stops with an error clear
/// it alike.
///
/// # Errors
///
/// Returns the error of the step that could not finish: the resolution of the
/// tool or of the toolchain, the discovery of the workspaces, the creation of
/// the copy, a run of cargo, or a run of nextest that left no answer.
async fn drive(context: &Context) -> Result<Outcome, CheckMinimalDepsError> {
    // checkminimaldeps[impl skip.git]
    // checkminimaldeps[impl skip.links]
    // checkminimaldeps[impl skip.missing]
    // checkminimaldeps[impl skip.target]
    if !Cargo::applies(context.root()).await {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_MANIFEST),
        });
    }

    // checkminimaldeps[impl tool.cargo]
    // checkminimaldeps[impl tool.missing]
    let cargo = Cargo::resolve(context.root().clone())
        .await
        .map_err(|source| CheckMinimalDepsError::UnresolvedTool { source })?;

    // checkminimaldeps[impl tool.toolchain]
    // checkminimaldeps[impl tool.unpinned]
    let toolchain = Toolchain::resolve(Channel::new(NIGHTLY), context.root())
        .await
        .map_err(|source| CheckMinimalDepsError::UnresolvedToolchain { source })?;

    // checkminimaldeps[impl roots.error]
    let roots = cargo
        .roots()
        .await
        .map_err(|source| CheckMinimalDepsError::UndiscoveredRoots { source })?;

    // checkminimaldeps[impl copy.disposable]
    // checkminimaldeps[impl copy.unavailable]
    let worktree = Worktree::create(context.root().clone())
        .await
        .map_err(|source| CheckMinimalDepsError::CopyUnavailable { source })?;

    let copies = copies(&worktree, &roots)?;
    let mut findings = Vec::new();

    // checkminimaldeps[impl roots.all]
    for root in &copies {
        findings.extend(update(&cargo, root, &toolchain, worktree.root()).await?);
    }

    // checkminimaldeps[impl update.failed]
    if !findings.is_empty() {
        return Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        });
    }

    // checkminimaldeps[impl tests.locked]
    let nextest = Nextest::new(cargo, Lockfile::Locked);
    let mut ran = 0;

    // checkminimaldeps[impl roots.all]
    for root in &copies {
        // checkminimaldeps[impl copy.paths]
        // checkminimaldeps[impl tests.error]
        // checkminimaldeps[impl tests.toolchain]
        let observation = nextest.observe(root, worktree.root()).await?;

        findings.extend(observation.findings().iter().cloned());
        ran += observation.ran();
    }

    if findings.is_empty() {
        // checkminimaldeps[impl tests.none]
        // checkminimaldeps[impl tests.passed]
        Ok(Outcome::Passed {
            summary: Some(summary(copies.len(), ran)),
        })
    } else {
        // checkminimaldeps[impl tests.failed]
        Ok(Outcome::Failed {
            findings,
            repairs: Vec::new(),
        })
    }
}

/// Returns the workspace roots as the copy holds them
///
/// The copy holds the tree of the project at its own root, so a root of the
/// project keeps the path that it has below the project root. The run works
/// on the roots of the copy from here on, and it names the paths of its
/// findings against the root of the copy, which gives them the paths that the
/// project has.
///
/// # Errors
///
/// Returns [`ForeignRoot`][foreign] when the copy holds no directory for a
/// root of the project, which is a project that moved while the run was on.
///
/// [foreign]: CheckMinimalDepsError::ForeignRoot
// checkminimaldeps[impl copy.paths]
// checkminimaldeps[impl roots.error]
fn copies(
    worktree: &Worktree,
    roots: &[CargoRoot],
) -> Result<Vec<CargoRoot>, CheckMinimalDepsError> {
    roots
        .iter()
        .map(|root| {
            worktree
                .path_of(root.directory())
                .map(CargoRoot::new)
                .ok_or_else(|| CheckMinimalDepsError::ForeignRoot {
                    root: root.directory().clone(),
                })
        })
        .collect()
}

/// Returns what cargo wrote about an update that it could not finish
///
/// Cargo writes its diagnosis to the standard error stream, so whoever reads
/// the finding reads the answer of cargo instead of a sentence that Rakko
/// wrote about it.
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
/// whose floor it could not resolve and the reason, and nothing that Rakko
/// could write about it would say more. The finding belongs to the manifest of
/// the root, which is the file that states the floors that cargo read.
// checkminimaldeps[impl copy.paths]
// checkminimaldeps[impl update.failed]
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
// checkminimaldeps[impl tests.passed]
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

/// Resolves the lowest versions that the manifests of one workspace allow
///
/// The update runs in the directory of the root, which lies in the copy, so
/// the lockfile that it rewrites is the lockfile of the copy. It runs on the
/// nightly toolchain, which is the only one that accepts the option, and the
/// selection reaches this run of cargo alone. An update that ends without
/// success is an answer about the project, and it comes back as the finding
/// that reports it.
///
/// # Errors
///
/// Returns [`CargoUnavailable`][unavailable] when cargo does not run.
///
/// [unavailable]: CheckMinimalDepsError::CargoUnavailable
// checkminimaldeps[impl copy.disposable]
// checkminimaldeps[impl update.operation]
async fn update(
    cargo: &Cargo,
    root: &CargoRoot,
    toolchain: &Toolchain,
    project: &ProjectRoot,
) -> Result<Option<Finding>, CheckMinimalDepsError> {
    let execution = cargo
        .invocation_with_toolchain(root, toolchain)
        .args(UPDATE)
        .run()
        .await
        .map_err(|source| CheckMinimalDepsError::CargoUnavailable { source })?;

    if execution.status().success() {
        return Ok(None);
    }

    // checkminimaldeps[impl update.failed]
    Ok(Some(finding(root, project, &execution)))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // checkminimaldeps[verify tests.passed]
    #[test]
    fn summary_of_many_tests_counts_them() {
        let summary = summary(2, 354);

        assert_eq!(summary.get(), "updated 2 workspaces and ran 354 tests");
    }

    // checkminimaldeps[verify tests.none]
    #[test]
    fn summary_of_no_test_counts_zero() {
        let summary = summary(2, 0);

        assert_eq!(summary.get(), "updated 2 workspaces and ran 0 tests");
    }

    // checkminimaldeps[verify tests.passed]
    #[test]
    fn summary_of_one_test_in_one_workspace_says_so() {
        let summary = summary(1, 1);

        assert_eq!(summary.get(), "updated 1 workspace and ran 1 test");
    }
}
