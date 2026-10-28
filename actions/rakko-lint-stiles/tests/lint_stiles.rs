//! Tests of the action that need no installation of stiles
//!
//! The tests cover what the action declares, and the run that stops when the
//! project pins a stiles that nothing installed. This repository pins no
//! stiles, because it holds no stiles rule to apply, so the tests that read
//! what stiles reported live beside the code that reads it, and they read
//! reports in the shape that stiles writes.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Outcome, ProjectRoot};
use rakko_lint_stiles::LintStiles;
use rakko_test_utils::run_checking_access;
use tempfile::TempDir;

/// A configuration of stiles that names a lint of the project
const CONFIGURATION: &str = "[[lints]]\npath = \"lints/todo_comment\"\n";

/// Code that a stiles rule could report
const SOURCE: &str = "// TODO: finish this\npub fn alpha() {}\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project that pins a stiles that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about stiles.
    fn without_stiles() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\n\"github:aonyx-ai/stiles\" = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project.write(".config/stiles.toml", CONFIGURATION);
        project.write("src/lib.rs", SOURCE);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        let root = ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists");

        Context::builder().root(root).build()
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        LintStiles.run(&self.context(), &()).await
    }

    /// Writes a file of the project, with the directories that lead to it
    fn write(&self, path: &str, content: &str) {
        let path = self.directory.path().join(path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the test creates the directories of a file");
        }

        std::fs::write(&path, content).expect("the test writes a file of the project");
    }
}

impl Drop for Project {
    /// Withdraws the trust that the project received
    ///
    /// Mise records the trust of a configuration in the state of the user, and
    /// the temporary directory of a test never comes back, so the record would
    /// pile up there forever. A failure stays quiet, because the test already
    /// reported what it was about.
    fn drop(&mut self) {
        let pins = self.directory.path().join("mise.toml");

        let _ = Command::new("mise")
            .arg("trust")
            .arg("--quiet")
            .arg("--untrust")
            .arg(pins)
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
    let schema = <<LintStiles as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

#[test]
fn action_identifies_itself_as_lint_stiles() {
    let name = LintStiles.name();

    assert_eq!(name.get(), "lint-stiles");
}

#[tokio::test]
async fn run_without_a_stiles_leaves_the_project_unchanged() {
    let project = Project::without_stiles();

    // The helper asserts that the run left every file of the project as it
    // was.
    run_checking_access(&LintStiles, &project.context(), &()).await;
}

#[tokio::test]
async fn run_without_a_stiles_stops() {
    let project = Project::without_stiles();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}
