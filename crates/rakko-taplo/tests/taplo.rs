//! Tests that drive the crate against real projects
//!
//! Each test builds a project in a temporary directory, so no fixture with a
//! deliberately broken TOML file sits in this repository, where the
//! formatting of the repository itself would fight it.
//!
//! The tests run the taplo that the crate declares. Each project pins
//! another taplo that nothing installs, so a test passes only when the run
//! ignores the pin of the project.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use bon::bon;
use rakko::{Context, ProjectRoot};
use rakko_taplo::{Observation, Operation, ProblemDetail, Taplo};
use tempfile::TempDir;

/// A TOML document that taplo leaves alone
const FORMATTED: &str = "x = 1\n";

/// A TOML document that taplo rewrites
const UNFORMATTED: &str = "x   =    1\n";

/// A TOML document that taplo cannot parse
const INVALID: &str = "broken = [1,\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon]
impl Project {
    /// Creates a project that pins a taplo that nothing installed
    ///
    /// The pin holds a version that no machine installs. The run resolves the
    /// taplo that the crate declares, so the pin of the project must not
    /// change what runs. Mise ignores a configuration that nobody trusts, so
    /// the file is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\ntaplo = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Runs one operation of taplo against this project
    async fn observe(&self, operation: Operation) -> Observation {
        let taplo = Taplo::resolve(&self.context())
            .await
            .expect("the test resolves the taplo that the crate declares");

        taplo
            .observe(operation)
            .await
            .expect("the test reads a complete report of taplo")
    }

    /// Returns the context of a run against this project
    fn context(&self) -> Context {
        Context::builder().root(self.root()).build()
    }

    /// Returns the root of this project
    ///
    /// The root is canonical, so the paths that a run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn root(&self) -> ProjectRoot {
        ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists")
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

impl Drop for Project {
    /// Withdraws the trust that the project received
    ///
    /// Mise records the trust of a configuration in the state of the user,
    /// and the temporary directory of a test never comes back, so the record
    /// would pile up there forever. A failure stays quiet, because the test
    /// already reported what it was about.
    fn drop(&mut self) {
        let _ = Command::new("mise")
            .arg("trust")
            .arg("--quiet")
            .arg("--untrust")
            .arg(self.directory.path().join("mise.toml"))
            .status();
    }
}

#[tokio::test]
async fn observe_a_format_check_reports_a_file_that_is_not_formatted() {
    let project = Project::new();
    project
        .write()
        .path("messy.toml")
        .content(UNFORMATTED)
        .call();

    let observation = project.observe(Operation::CheckFormat).await;

    assert_eq!(
        observation
            .problems()
            .first()
            .map(|problem| problem.detail()),
        Some(&ProblemDetail::Unformatted)
    );
}

#[tokio::test]
async fn observe_a_format_rewrites_the_project() {
    let project = Project::new();
    project
        .write()
        .path("messy.toml")
        .content(UNFORMATTED)
        .call();

    project.observe(Operation::Format).await;

    let content = std::fs::read_to_string(project.directory.path().join("messy.toml"))
        .expect("the test reads a file that it wrote");
    assert_eq!(content, FORMATTED);
}

#[tokio::test]
async fn observe_a_lint_ignores_a_file_that_is_not_formatted() {
    let project = Project::new();
    project
        .write()
        .path("messy.toml")
        .content(UNFORMATTED)
        .call();

    let observation = project.observe(Operation::Lint).await;

    assert!(
        observation.problems().is_empty(),
        "expected no problem, got {:?}",
        observation.problems()
    );
}

#[tokio::test]
async fn observe_a_lint_reports_a_file_that_taplo_cannot_parse() {
    let project = Project::new();
    project.write().path("broken.toml").content(INVALID).call();

    let observation = project.observe(Operation::Lint).await;

    assert!(
        matches!(
            observation
                .problems()
                .first()
                .map(|problem| problem.detail()),
            Some(&ProblemDetail::Diagnostic { .. })
        ),
        "expected a diagnostic, got {:?}",
        observation.problems()
    );
}

#[tokio::test]
async fn observe_reads_a_report_without_color_codes() {
    let project = Project::new();
    project.write().path("broken.toml").content(INVALID).call();

    let taplo = Taplo::resolve(&project.context())
        .await
        .expect("the test resolves the taplo that the crate declares");

    let observation = taplo.observe(Operation::Lint).await;

    assert!(
        observation.as_ref().is_ok_and(|observation| {
            observation.problems().iter().any(|problem| {
                matches!(
                    problem.detail(),
                    ProblemDetail::Diagnostic { message, .. } if !message.contains('\u{1b}')
                )
            })
        }),
        "expected a diagnostic without an escape code, got {observation:?}"
    );
}

#[tokio::test]
async fn resolve_in_a_missing_directory_reports_an_error() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();

    let taplo = Taplo::resolve(&context).await;

    assert!(
        taplo.is_err(),
        "expected the resolution to fail, got {taplo:?}"
    );
}

#[tokio::test]
async fn resolve_in_a_project_that_pins_another_taplo_finds_the_declared_one() {
    let project = Project::new();

    let taplo = Taplo::resolve(&project.context()).await;

    assert!(taplo.is_ok(), "expected the declared taplo, got {taplo:?}");
}
