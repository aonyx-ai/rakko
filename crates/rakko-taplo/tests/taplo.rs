//! Tests that drive the crate against real projects
//!
//! Each test builds a project in a temporary directory, so no fixture with a
//! deliberately broken TOML file sits in this repository, where the
//! formatting of the repository itself would fight it.
//!
//! The tests run the taplo that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers
//! is the version that the repository installs, and a new pin reaches the
//! tests without a change to them.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::ProjectRoot;
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

impl Project {
    /// Creates a project without a taplo to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when it only looks at the project.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the taplo of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the taplo
    /// that mise resolves for it is the taplo that the repository pins and
    /// installs. Mise ignores a configuration that nobody trusts, so the
    /// copy is trusted right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project
    }

    /// Creates a project that pins a taplo that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine
    /// says about taplo.
    fn without_taplo() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\ntaplo = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns whether taplo has anything to do in this project
    async fn applies(&self) -> bool {
        Taplo::applies(&self.root()).await
    }

    /// Runs one operation of taplo against this project
    async fn observe(&self, operation: Operation) -> Observation {
        let taplo = Taplo::resolve(self.root())
            .await
            .expect("the test resolves the taplo that the repository pins");

        taplo
            .observe(operation)
            .await
            .expect("the test reads a complete report of taplo")
    }

    /// Returns the root of this project
    ///
    /// The root is canonical, so the paths that a run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn root(&self) -> ProjectRoot {
        let path = self
            .directory
            .path()
            .canonicalize()
            .expect("the test names a directory that exists");

        ProjectRoot::new(path)
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

/// Returns the root of the repository that the tests run in
fn repository() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    manifest
        .parent()
        .and_then(Path::parent)
        .expect("the crate lives two directories below the root of the repository")
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

// taplo[verify look.git]
#[tokio::test]
async fn applies_with_toml_only_under_the_git_directory_is_false() {
    let project = Project::bare();
    project.write(".git/config.toml", FORMATTED);

    let applies = project.applies().await;

    assert!(!applies);
}

// taplo[verify look.links]
#[cfg(unix)]
#[tokio::test]
async fn applies_with_toml_only_behind_a_symbolic_link_is_false() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("linked.toml"), FORMATTED)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let applies = project.applies().await;

    assert!(!applies);
}

// taplo[verify look.unreadable]
#[cfg(unix)]
#[tokio::test]
async fn applies_with_an_unreadable_directory_is_true() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::bare();
    project.write("closed/README.md", "# Project\n");
    let closed = project.directory.path().join("closed");
    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a directory");

    let applies = project.applies().await;

    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o755))
        .expect("the test restores the permissions of a directory");
    assert!(applies);
}

// taplo[verify look.toml]
#[tokio::test]
async fn applies_with_a_toml_file_is_true() {
    let project = Project::bare();
    project.write("sub/clean.toml", FORMATTED);

    let applies = project.applies().await;

    assert!(applies);
}

// taplo[verify look.toml]
#[tokio::test]
async fn applies_without_a_toml_file_is_false() {
    let project = Project::bare();
    project.write("README.md", "# Project\n");

    let applies = project.applies().await;

    assert!(!applies);
}

// taplo[verify run.operation]
#[tokio::test]
async fn observe_a_format_check_reports_a_file_that_is_not_formatted() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);

    let observation = project.observe(Operation::CheckFormat).await;

    assert_eq!(
        observation
            .problems()
            .first()
            .map(|problem| problem.detail()),
        Some(&ProblemDetail::Unformatted)
    );
}

// taplo[verify run.operation]
#[tokio::test]
async fn observe_a_format_rewrites_the_project() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);

    project.observe(Operation::Format).await;

    let content = std::fs::read_to_string(project.directory.path().join("messy.toml"))
        .expect("the test reads a file that it wrote");
    assert_eq!(content, FORMATTED);
}

// taplo[verify run.operation]
#[tokio::test]
async fn observe_a_lint_ignores_a_file_that_is_not_formatted() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);

    let observation = project.observe(Operation::Lint).await;

    assert!(
        observation.problems().is_empty(),
        "expected no problem, got {:?}",
        observation.problems()
    );
}

// taplo[verify run.operation]
#[tokio::test]
async fn observe_a_lint_reports_a_file_that_taplo_cannot_parse() {
    let project = Project::new();
    project.write("broken.toml", INVALID);

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

// taplo[verify run.plain]
#[tokio::test]
async fn observe_reads_a_report_without_color_codes() {
    let project = Project::new();
    project.write("broken.toml", INVALID);

    let observation = project.observe(Operation::Lint).await;

    assert!(
        !observation.stderr().contains('\u{1b}'),
        "expected a report without an escape code, got {:?}",
        observation.stderr()
    );
}

// taplo[verify tool.resolve]
#[tokio::test]
async fn resolve_in_a_project_that_pins_taplo_finds_the_program() {
    let project = Project::new();

    let taplo = Taplo::resolve(project.root()).await;

    assert!(taplo.is_ok(), "expected a taplo, got {taplo:?}");
}

// taplo[verify tool.missing]
#[tokio::test]
async fn resolve_without_a_taplo_reports_the_tool() {
    let project = Project::without_taplo();

    let taplo = Taplo::resolve(project.root()).await;

    let Err(error) = taplo else {
        panic!("expected the resolution to fail, got {taplo:?}");
    };
    assert!(
        error.to_string().contains("taplo"),
        "expected the error to name the tool, got {error}"
    );
}
