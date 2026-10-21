//! Tests of the action that need no installation of tracey
//!
//! The tests cover what the action declares, the run that stops when the
//! project pins a tracey that nothing installed, and the comparison of a pull
//! request with its base, which only runs git. A test that needs files builds
//! a project in a temporary directory, so no fixture with a deliberately
//! changed specification sits in this repository.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Outcome, ProjectRoot};
use rakko_check_specs::{CheckSpecs, Comparison};
use rakko_test_utils::run_checking_access;
use tempfile::TempDir;

/// The configuration that points tracey at the files of a project
const CONFIGURATION: &str = r"@schema {id crate:tracey-config@1, cli tracey}

specs (
    {
        name probe
        source_url https://example.com/probe
        include (SPECIFICATION.md)
        impls (
            {
                name rust
                include (src/**/*.rs)
                test_include (tests/**/*.rs)
            }
        )
    }
)
";

/// A specification with one requirement
const SPECIFICATION: &str = "# Probe\n\nprobe[alpha]\nThe thing MUST do alpha.\n";

/// A specification whose requirement says something else, at the same version
const CHANGED: &str = "# Probe\n\nprobe[alpha]\nThe thing MUST do alpha and gamma.\n";

/// Code that implements the requirement of the specification
const IMPLEMENTATION: &str = "// probe[impl alpha]\npub fn alpha() {}\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates an empty project without a tracey to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project that pins a tracey that nothing installed
    fn without_tracey() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(
            &pins,
            "[tools]\n\"cargo:https://github.com/bearcove/tracey\" = { version = \"rev:0000000000000000000000000000000000000000\", crate = \"tracey\" }\n",
        )
        .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project.write(".config/tracey/config.styx", CONFIGURATION);
        project.write("SPECIFICATION.md", SPECIFICATION);
        project.write("src/lib.rs", IMPLEMENTATION);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        Context::builder().root(self.root()).build()
    }

    /// Returns the canonical root of the project
    fn root(&self) -> ProjectRoot {
        ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists")
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        CheckSpecs.run(&self.context(), &()).await
    }

    /// Writes a file of the project, with the directories that lead to it
    fn write(&self, path: &str, content: &str) {
        let path = self.directory.path().join(path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the test creates the directories of a file");
        }

        std::fs::write(&path, content).expect("the test writes a file of the project");
    }

    /// Runs one command of git in the project
    fn git(&self, arguments: &[&str]) -> String {
        git(self.directory.path(), arguments)
    }
}

/// Runs one command of git in the directory
fn git(directory: &Path, arguments: &[&str]) -> String {
    // The commits of a fixture are thrown away with its directory, and
    // the signing key of a contributor is not the test's to ask for.
    let mut command = Command::new("git");
    command
        .arg("-c")
        .arg("commit.gpgsign=false")
        .args(arguments)
        .current_dir(directory);

    // A test that runs inside a git hook inherits the variables that name
    // the repository of the hook, and those beat the working directory, so
    // the fixture would act on the checkout of the contributor. The
    // identity below is set afterwards, because it carries the prefix too.
    for (name, _) in std::env::vars_os() {
        if name.as_encoded_bytes().starts_with(b"GIT_") {
            command.env_remove(name);
        }
    }

    let output = command
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .expect("the test starts git");

    assert!(
        output.status.success(),
        "expected git to run {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

impl Drop for Project {
    /// Withdraws the trust that the project received
    ///
    /// Mise records the trust of a configuration in the state of the user, and
    /// the temporary directory of a test never comes back, so the record would
    /// pile up there forever.
    fn drop(&mut self) {
        let configuration = self.directory.path().join("mise.toml");

        // A project that never pinned a tool trusted nothing, and mise reads a
        // path that is not there as its parent directory.
        if !configuration.is_file() {
            return;
        }

        let _ = Command::new("mise")
            .arg("trust")
            .arg("--quiet")
            .arg("--untrust")
            .arg(configuration)
            .status();
    }
}

/// Trusts a mise configuration, so that mise reads it
fn trust(configuration: &Path) {
    let trusted = Command::new("mise")
        .arg("trust")
        .arg("--quiet")
        .arg(configuration)
        .status()
        .expect("the test starts mise to trust a configuration");

    assert!(
        trusted.success(),
        "expected mise to trust the configuration"
    );
}

#[test]
fn action_declares_no_argument() {
    let schema = <<CheckSpecs as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

#[test]
fn action_identifies_itself_as_check_specs() {
    let name = CheckSpecs.name();

    assert_eq!(name.get(), "check-specs");
}

// A code host checks out a merge commit of the branch and its base, and stages
// nothing, so a comparison of that index with that HEAD finds no change at all.
#[tokio::test]
async fn comparison_of_a_merge_commit_stages_the_pull_request() {
    let project = merged();

    let comparison = Comparison::prepare(&project.root(), true)
        .await
        .expect("the test compares the pull request with its base")
        .expect("the test checks out a merge commit");

    let staged = git(
        &comparison.directory().join("base"),
        &["diff", "--cached", "--name-only"],
    );
    assert_eq!(staged, "SPECIFICATION.md");
}

#[tokio::test]
async fn comparison_leaves_the_commit_of_the_checkout_alone() {
    let project = merged();
    let before = project.git(&["rev-parse", "HEAD"]);

    let comparison = Comparison::prepare(&project.root(), true)
        .await
        .expect("the test compares the pull request with its base");
    drop(comparison);

    assert_eq!(project.git(&["rev-parse", "HEAD"]), before);
}

#[tokio::test]
async fn comparison_leaves_the_index_of_the_checkout_alone() {
    let project = merged();

    let comparison = Comparison::prepare(&project.root(), true)
        .await
        .expect("the test compares the pull request with its base");
    drop(comparison);

    assert_eq!(project.git(&["diff", "--cached", "--name-only"]), "");
}

// A run that no environment calls a pull request compares the checkout itself.
#[tokio::test]
async fn comparison_that_nothing_asked_for_is_none() {
    let project = merged();

    let comparison = Comparison::prepare(&project.root(), false)
        .await
        .expect("the test asks for no comparison");

    assert!(comparison.is_none(), "expected no comparison");
}

#[tokio::test]
async fn run_without_a_tracey_leaves_the_project_unchanged() {
    let project = Project::without_tracey();

    // The helper asserts that the run left every file of the project as it
    // was.
    run_checking_access(&CheckSpecs, &project.context(), &()).await;
}

#[tokio::test]
async fn run_without_a_tracey_stops() {
    let project = Project::without_tracey();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

/// Returns a project whose checkout is a merge commit of a pull request
///
/// The branch changes the text of a requirement and bumps no version, which is
/// what the comparison of a pull request has to find. The comparison only
/// runs git, so the project holds the specification and nothing else.
fn merged() -> Project {
    let project = Project::bare();
    project.write("SPECIFICATION.md", SPECIFICATION);

    project.git(&["init", "--quiet", "--initial-branch", "main", "."]);
    project.git(&["add", "-A"]);
    project.git(&["commit", "--quiet", "-m", "base"]);
    project.git(&["checkout", "--quiet", "-b", "feature"]);
    project.write("SPECIFICATION.md", CHANGED);
    project.git(&["add", "-A"]);
    project.git(&["commit", "--quiet", "-m", "change the requirement"]);
    project.git(&["checkout", "--quiet", "main"]);
    project.git(&["merge", "--quiet", "--no-ff", "feature", "-m", "Merge"]);
    let merge = project.git(&["rev-parse", "HEAD"]);
    project.git(&["checkout", "--quiet", "--detach", &merge]);

    project
}
