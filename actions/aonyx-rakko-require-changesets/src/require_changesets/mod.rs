//! The action that requires a changeset for each changed release unit
//!
//! This module holds the action and the error that stops a run. The action
//! reads the pull request from the event file of GitHub Actions. It asks Otto
//! for the release units, and Git for the files that the pull request
//! changes. The judgment itself lives in the verdict.

mod error;

use rakko::{
    Access, Action, Context, Documented, Name, Outcome, SkipReason, Summary, Tools, action_name,
};

pub use self::error::RequireChangesetsError;
use crate::git;
use crate::opt_out::OptOuts;
use crate::otto::Otto;
use crate::pull_request::{Environment, Origin, PullRequest};
use crate::verdict;

/// The reason of a run outside a pull request
const NO_PULL_REQUEST: &str = "the run has no GitHub pull request";

/// The reason of a run in a project where Otto lists no release unit
const NO_UNITS: &str = "otto lists no release unit in the project";

/// Requires a changeset for each release unit that a pull request changes
///
/// [Otto] builds the changelog of a release from changesets: one file for each
/// change in the `.changeset` directory of a release unit. A change without a
/// changeset is missing from the changelog and from the release notes. The
/// action fails a pull request that changes a unit and adds no changeset to
/// it. The body of the pull request can opt the unit out with a reason.
///
/// Otto finds the units. The action runs `otto list-units` in the root of the
/// project, at the version of Otto that it declares, which [mise] installed. A
/// unit counts as changed when the pull request changes a file that the unit
/// owns, by the rule that Otto gives for its list. The manifest of a unit
/// belongs to that unit. Any other file belongs to the nearest directory above
/// it among the directories of the crates and the directories that Otto lists
/// as unowned. A file that no unit owns, such as `Cargo.lock` at the top of a
/// repository, changes no unit. The action compares the head
/// of the pull request with its base, so a fixup commit after a changeset
/// does not hide that changeset. A changeset is a file with the extension
/// `md` that the pull request adds directly to the `.changeset` directory of
/// the root of the unit. The action looks for the presence of a changeset and
/// not for its content.
///
/// A line in the body opts a unit out:
///
/// ```text
/// No changeset for <unit>: <reason>
/// ```
///
/// One line opts out every unit of the pull request:
///
/// ```text
/// No changeset: <reason>
/// ```
///
/// A line that gives no reason fails the run. A line for a unit that Otto does
/// not list fails the run as well. So a typo cannot skip a unit without a sign. The
/// summary of a passing run gives each unit that it accepted without a
/// changeset, with its reason. A pull request from the branch `release/<unit>`
/// of a listed unit, in the repository of the pull request, is a release pull
/// request of Otto. It passes, because a release writes the changelog itself.
/// A branch of a fork with such a name gets the normal check.
///
/// The action runs only for the event of a pull request in GitHub Actions. It
/// reads the pull request from the event file that GitHub Actions writes, with
/// no call over the network and no token. Any other run, such as one on the
/// machine of a contributor, skips with a reason. A run in a pull request
/// stops with an error when it cannot read the event file. It also stops when
/// Otto gives no list of the units. It stops when Git cannot compare the base
/// with the head, for example in a checkout without the history of both. A
/// project where Otto lists no unit has nothing to check, so the run skips. The event of an
/// edit of the body starts a new run only when the workflow listens for it.
///
/// A run declares a read of the project. A caller that orders runs by their
/// declarations runs it after the actions that write.
///
/// # Examples
///
/// A harness adds the action as it is to a list:
///
/// ```
/// use rakko::Bundle;
/// use aonyx_rakko_require_changesets::RequireChangesets;
///
/// let actions = Bundle::default().with(RequireChangesets);
/// ```
///
/// [mise]: https://mise.jdx.dev
/// [otto]: https://github.com/aonyx-ai/otto
#[derive(Copy, Clone, Debug, Default, Documented)]
pub struct RequireChangesets;

impl Action for RequireChangesets {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("require-changesets")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    fn bundled_tools(&self) -> Tools {
        rakko::bundled_tools!()
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        match drive(context, self, &Environment::of_process()).await {
            Ok(outcome) => outcome,
            Err(error) => Outcome::Errored {
                source: Box::new(error),
            },
        }
    }
}

/// Runs the action against the project of the context, in the given
/// environment
///
/// The run reads the pull request first, so that a run without one skips
/// before it starts a program. Then it lists the units, and it skips when
/// there are none. It passes a release pull request of Otto before it
/// compares the commits.
///
/// # Errors
///
/// Returns the error of the step that could not finish. The steps are the
/// reading of the pull request, the resolution of Otto, the list of the
/// units, and the comparison of the commits.
async fn drive(
    context: &Context,
    action: &RequireChangesets,
    environment: &Environment,
) -> Result<Outcome, RequireChangesetsError> {
    let Some(pull_request) = PullRequest::read(environment)? else {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_PULL_REQUEST),
        });
    };

    let otto = Otto::resolve()
        .context(context)
        .action(action)
        .call()
        .await
        .map_err(|source| RequireChangesetsError::UnresolvedTool { source })?;

    let listing = otto.list_units().await?;
    let units = listing.units();

    if units.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::new(NO_UNITS),
        });
    }

    let released = pull_request
        .branch()
        .released_unit()
        .filter(|_| pull_request.origin() == Origin::BaseRepository)
        .filter(|name| units.iter().any(|unit| unit.name().get() == *name));

    if let Some(name) = released {
        return Ok(Outcome::Passed {
            summary: Some(Summary::new(format!(
                "a release pull request of Otto for {name}"
            ))),
        });
    }

    let comparison = git::compare(context.root(), pull_request.base(), pull_request.head()).await?;
    let opt_outs = OptOuts::read(pull_request.body().get());

    Ok(verdict::judge(&listing, &comparison, &opt_outs))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "An assertion in a test panics by design, and the helpers of this module exist only for tests."
    )]
    #![allow(clippy::missing_panics_doc)]

    use std::collections::BTreeSet;
    use std::path::Path;
    use std::process::Command;

    use rakko::{OperatingSystem, Platform, ProjectRoot};
    use tempfile::TempDir;

    use super::*;

    /// The manifest of the workspace of the repositories of the tests
    const WORKSPACE: &str = "[workspace]\nmembers = [\"crates/alpha\"]\nresolver = \"3\"\n";

    /// The number that GitHub gives the repository of the pull requests
    #[cfg(not(windows))]
    const BASE_REPOSITORY: u64 = 81;

    /// The manifest of the crate `alpha`, which is an independent unit
    const ALPHA: &str = "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

    /// A repository with a base commit and a branch of a pull request
    ///
    /// The base commit holds a workspace with the crate `alpha`. The test adds
    /// files on the branch and commits them. The event file lies outside the
    /// repository, as GitHub Actions writes it outside the checkout.
    struct Repository {
        /// The directory of the repository
        directory: TempDir,

        /// The directory of the event file
        #[cfg_attr(
            windows,
            expect(
                dead_code,
                reason = "Only the tests that run Otto write an event file, and the pin of Otto leaves Windows out."
            )
        )]
        events: TempDir,

        /// The hash of the base commit
        base: String,
    }

    #[bon::bon]
    impl Repository {
        /// Creates the repository with its base commit
        fn new() -> Self {
            let directory = tempfile::tempdir().expect("the test creates a temporary directory");
            let events = tempfile::tempdir().expect("the test creates a temporary directory");
            let mut repository = Self {
                directory,
                events,
                base: String::new(),
            };

            repository.git(&["init", "--quiet", "--initial-branch=main"]);
            repository
                .write()
                .path("Cargo.toml")
                .content(WORKSPACE)
                .call();
            repository
                .write()
                .path("crates/alpha/Cargo.toml")
                .content(ALPHA)
                .call();
            repository
                .write()
                .path("crates/alpha/src/lib.rs")
                .content("//! Alpha\n")
                .call();
            repository.base = repository.commit("Add alpha");
            repository.git(&["switch", "--quiet", "--create", "change"]);

            repository
        }

        /// Commits every file of the working tree and returns the hash
        fn commit(&self, message: &str) -> String {
            self.git(&["add", "--all"]);
            self.git(&[
                "-c",
                "user.name=Otter",
                "-c",
                "user.email=otter@example.com",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "--message",
                message,
            ]);

            self.git(&["rev-parse", "HEAD"])
        }

        /// Returns the context of a run in the repository
        fn context(&self) -> Context {
            let root = ProjectRoot::canonical(self.directory.path())
                .expect("the test names a directory that exists");

            Context::builder().root(root).build()
        }

        /// Returns the environment of a job for a pull request from the branch
        /// to the base, with the body
        ///
        /// The base is the base commit of the repository, unless the test
        /// gives another one.
        #[cfg(not(windows))]
        #[builder]
        fn environment(
            &self,
            base: Option<&str>,
            branch: &str,
            head: &str,
            #[builder(default)] body: &str,
            #[builder(default = BASE_REPOSITORY)] head_repository: u64,
        ) -> Environment {
            let quoted =
                |text: &str| serde_json::to_string(text).expect("the test writes a string as JSON");
            let event = format!(
                r#"{{"pull_request":{{"body":{},"base":{{"ref":"main","sha":{},"repo":{{"id":{BASE_REPOSITORY}}}}},"head":{{"ref":{},"sha":{},"repo":{{"id":{head_repository}}}}}}}}}"#,
                quoted(body),
                quoted(base.unwrap_or(&self.base)),
                quoted(branch),
                quoted(head),
            );
            let path = self.events.path().join("event.json");
            std::fs::write(&path, event).expect("the test writes the event file");

            Environment::builder()
                .event_name(std::ffi::OsString::from("pull_request"))
                .event_path(path)
                .build()
        }

        /// Runs Git in the repository and returns what it wrote
        fn git(&self, arguments: &[&str]) -> String {
            let mut command = Command::new("git");
            for (name, _) in std::env::vars_os() {
                if name.as_encoded_bytes().starts_with(b"GIT_") {
                    command.env_remove(name);
                }
            }

            let output = command
                .args(arguments)
                .current_dir(self.directory.path())
                .output()
                .expect("the test runs git");
            assert!(
                output.status.success(),
                "git {arguments:?} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );

            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }

        /// Runs the action in the environment
        async fn run(&self, environment: &Environment) -> Outcome {
            match drive(&self.context(), &RequireChangesets, environment).await {
                Ok(outcome) => outcome,
                Err(error) => Outcome::Errored {
                    source: Box::new(error),
                },
            }
        }

        /// Writes a file of the repository, with the directories that lead to
        /// it
        #[builder]
        fn write(&self, path: &str, content: &str) {
            let path = self.directory.path().join(Path::new(path));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("the test creates a directory");
            }

            std::fs::write(path, content).expect("the test writes a file");
        }
    }

    #[test]
    fn access_is_read() {
        let context = Context::builder().root("/tmp/project").build();

        let access = RequireChangesets.access(&context, &());

        assert_eq!(access, Access::Read);
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_change_to_a_fixture_below_a_crate_passes() {
        let repository = Repository::new();
        repository
            .write()
            .path("Cargo.toml")
            .content(&format!(
                "{WORKSPACE}exclude = [\"crates/alpha/tests/fixture\"]\n"
            ))
            .call();
        repository
            .write()
            .path("crates/alpha/tests/fixture/Cargo.toml")
            .content("[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
            .call();
        repository
            .write()
            .path("crates/alpha/tests/fixture/src/lib.rs")
            .content("//! Fixture\n")
            .call();
        repository.git(&["switch", "--quiet", "main"]);
        let base = repository.commit("Add a fixture");
        repository.git(&["switch", "--quiet", "--force-create", "change"]);
        repository
            .write()
            .path("crates/alpha/tests/fixture/src/lib.rs")
            .content("//! Fixture, changed\n")
            .call();
        let head = repository.commit("Change the fixture");

        let outcome = repository
            .run(
                &repository
                    .environment()
                    .base(&base)
                    .branch("change")
                    .head(&head)
                    .call(),
            )
            .await;

        assert!(
            matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "the pull request changes no release unit"),
            "expected a pass that changes no unit, got {outcome:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_change_with_a_changeset_after_a_fixup_passes() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/alpha/src/lib.rs")
            .content("//! Alpha, changed\n")
            .call();
        repository
            .write()
            .path("crates/alpha/.changeset/change-alpha.md")
            .content("+++\ncategory = \"Changed\"\ntitle = \"Change alpha\"\n+++\n")
            .call();
        repository.commit("Change alpha");
        repository
            .write()
            .path("crates/alpha/src/lib.rs")
            .content("//! Alpha, fixed\n")
            .call();
        let head = repository.commit("Fix alpha");

        let outcome = repository
            .run(&repository.environment().branch("change").head(&head).call())
            .await;

        assert!(
            matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "1 changed release unit adds a changeset"),
            "expected a pass, got {outcome:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_change_with_an_opt_out_shows_the_reason() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/alpha/src/lib.rs")
            .content("//! Alpha, changed\n")
            .call();
        let head = repository.commit("Change alpha");

        let outcome = repository
            .run(
                &repository
                    .environment()
                    .branch("change")
                    .head(&head)
                    .body("Refactor alpha.\r\n\r\nNo changeset for alpha: only a refactor\r\n")
                    .call(),
            )
            .await;

        assert!(
            matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "accepted without a changeset: alpha (only a refactor)"),
            "expected a pass with the reason, got {outcome:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_change_without_a_changeset_fails() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/alpha/src/lib.rs")
            .content("//! Alpha, changed\n")
            .call();
        let head = repository.commit("Change alpha");

        let outcome = repository
            .run(&repository.environment().branch("change").head(&head).call())
            .await;

        assert!(
            matches!(&outcome, Outcome::Failed { findings, .. } if findings.len() == 1),
            "expected one finding, got {outcome:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_change_without_history_of_the_base_stops() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/alpha/src/lib.rs")
            .content("//! Alpha, changed\n")
            .call();
        let head = repository.commit("Change alpha");
        let environment = repository
            .environment()
            .base("0123456789012345678901234567890123456789")
            .branch("change")
            .head(&head)
            .call();

        let result = drive(&repository.context(), &RequireChangesets, &environment).await;

        assert!(
            matches!(
                result,
                Err(RequireChangesetsError::IncomparableCommits { .. })
            ),
            "expected commits that git cannot compare, got {result:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_project_without_release_units_skips() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/alpha/Cargo.toml")
            .content(&format!("{ALPHA}publish = false\n"))
            .call();
        let head = repository.commit("Keep alpha out of the releases");

        let outcome = repository
            .run(&repository.environment().branch("change").head(&head).call())
            .await;

        assert!(
            matches!(&outcome, Outcome::Skipped { reason } if reason.get() == NO_UNITS),
            "expected a skip, got {outcome:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_release_branch_of_a_fork_judges_the_change() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/alpha/src/lib.rs")
            .content("//! Alpha, changed\n")
            .call();
        let head = repository.commit("Change alpha");

        let outcome = repository
            .run(
                &repository
                    .environment()
                    .branch("release/alpha")
                    .head(&head)
                    .head_repository(92)
                    .call(),
            )
            .await;

        assert!(
            matches!(&outcome, Outcome::Failed { .. }),
            "expected a failure, got {outcome:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_release_branch_of_an_unknown_unit_judges_the_change() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/alpha/src/lib.rs")
            .content("//! Alpha, changed\n")
            .call();
        let head = repository.commit("Change alpha");

        let outcome = repository
            .run(
                &repository
                    .environment()
                    .branch("release/omega")
                    .head(&head)
                    .call(),
            )
            .await;

        assert!(
            matches!(&outcome, Outcome::Failed { .. }),
            "expected a failure, got {outcome:?}"
        );
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn drive_of_a_release_pull_request_passes() {
        let repository = Repository::new();
        repository
            .write()
            .path("crates/alpha/CHANGELOG.md")
            .content("# Changelog\n")
            .call();
        let head = repository.commit("Release alpha 0.2.0");

        let outcome = repository
            .run(
                &repository
                    .environment()
                    .branch("release/alpha")
                    .head(&head)
                    .call(),
            )
            .await;

        assert!(
            matches!(&outcome, Outcome::Passed { summary: Some(summary) } if summary.get() == "a release pull request of Otto for alpha"),
            "expected a pass for the release, got {outcome:?}"
        );
    }

    #[tokio::test]
    async fn drive_outside_a_pull_request_skips() {
        let repository = Repository::new();

        let outcome = repository.run(&Environment::default()).await;

        assert!(
            matches!(&outcome, Outcome::Skipped { reason } if reason.get() == NO_PULL_REQUEST),
            "expected a skip, got {outcome:?}"
        );
    }

    #[test]
    fn require_changesets_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<RequireChangesets>();
    }

    /// Shows that the machinery skips the action on Windows
    ///
    /// The pin of Otto leaves Windows out, so the test runs only there. A run
    /// through [`rakko::ErasedAction`] is the path that the command line, a
    /// group, and a written command take.
    #[cfg(windows)]
    #[tokio::test]
    async fn run_through_the_machinery_on_windows_skips_for_the_missing_build() {
        let repository = Repository::new();

        let outcome = rakko::ErasedAction::run(
            &RequireChangesets,
            &repository.context(),
            &rakko::ArgsValues::default(),
        )
        .await;

        assert!(
            matches!(
                &outcome,
                Outcome::Skipped { reason }
                    if reason.get().starts_with("the tool cargo:otto has no build for windows")
            ),
            "expected the run to skip for the missing build of Otto, got {outcome:?}"
        );
    }

    #[test]
    fn tools_declare_otto_alone() {
        let tools = RequireChangesets.tools();

        let names: Vec<&str> = tools.iter().map(|tool| tool.tool().get()).collect();

        assert_eq!(names, ["cargo:otto"]);
    }

    #[test]
    fn tools_declare_the_version_of_the_mise_file() {
        let tools = RequireChangesets.tools();

        let version = tools[0].version();

        assert!(
            include_str!("../../mise.rakko.toml")
                .contains(&format!("\"cargo:otto\" = {{ version = \"{version}\"")),
            "expected the version of the mise file of the crate, got {version}"
        );
    }

    #[test]
    fn tools_leave_out_windows() {
        let tools = RequireChangesets.tools();

        assert_eq!(
            tools[0].platforms(),
            Some(&BTreeSet::from([
                Platform::new(OperatingSystem::Linux),
                Platform::new(OperatingSystem::Macos),
            ]))
        );
    }
}
