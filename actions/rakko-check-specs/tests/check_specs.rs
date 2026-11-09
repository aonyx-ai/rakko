//! Tests of the action that need no installation of tracey
//!
//! The tests cover what the action declares and the run that stops when the
//! project pins a tracey that nothing installed. A test that needs files
//! builds a project in a temporary directory, so no fixture sits in this
//! repository.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko::{Action, Args, Context, Outcome, ProjectRoot};
use rakko_check_specs::CheckSpecs;
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

/// Code that implements the requirement of the specification
const IMPLEMENTATION: &str = "// probe[impl alpha]\npub fn alpha() {}\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
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

        project
            .write()
            .path(".config/tracey/config.styx")
            .content(CONFIGURATION)
            .call();
        project
            .write()
            .path("SPECIFICATION.md")
            .content(SPECIFICATION)
            .call();
        project
            .write()
            .path("src/lib.rs")
            .content(IMPLEMENTATION)
            .call();

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
    #[builder]
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
    /// Mise records the trust of a configuration in the state of the user.
    /// The temporary directory of a test never comes back, so the record would
    /// pile up there forever.
    ///
    /// A project that never pinned a tool trusted nothing, and mise reads a
    /// path that is not there as its parent directory. The method skips such a
    /// project.
    fn drop(&mut self) {
        let configuration = self.directory.path().join("mise.toml");

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

#[tokio::test]
async fn run_without_a_tracey_leaves_the_project_unchanged() {
    let project = Project::without_tracey();

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
