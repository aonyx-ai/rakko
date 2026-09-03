//! Tests that drive the crate against real projects
//!
//! Each test builds a project in a temporary directory, so no fixture with a
//! deliberately broken file sits in this repository, where the formatting of
//! the repository itself would fight it.
//!
//! The tests run the prettier that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers is
//! the version that the repository installs, and a new pin reaches the tests
//! without a change to them.
//!
//! Prettier runs on Node, and the program that mise reports starts Node from
//! the environment of the process. The tests therefore run inside the
//! environment of mise, which every Just recipe of this repository enters.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::ProjectRoot;
use rakko_prettier::{FileExtension, Filter, Observation, Operation, Prettier, ProblemDetail};
use tempfile::TempDir;

/// A Markdown document that prettier leaves alone
const FORMATTED: &str = "# Project\n";

/// A Markdown document that prettier rewrites
const UNFORMATTED: &str = "#  Project\n";

/// A file of a language that prettier does not know
const FOREIGN: &str = "fn main() {}\n";

/// A prettier configuration that names an option prettier does not know
///
/// Prettier reports the option and then formats without it, which is the
/// shortest way to make a run write a marker line of its own.
const IGNORED_CONFIGURATION: &str = "{ \"notAnOption\": 5 }\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a prettier to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when it only looks at the project.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the prettier of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the prettier
    /// that mise resolves for it is the prettier that the repository pins and
    /// installs. Mise ignores a configuration that nobody trusts, so the copy
    /// is trusted right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project
    }

    /// Creates a project that pins a prettier that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about prettier.
    fn without_prettier() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\n\"npm:prettier\" = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns whether prettier has anything to do in this project
    async fn applies(&self, filter: &Filter) -> bool {
        Prettier::applies(&self.root(), filter).await
    }

    /// Returns what a file of this project holds
    fn content(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs one operation of prettier against this project
    async fn observe(&self, operation: Operation, filter: &Filter) -> Observation {
        let prettier = Prettier::resolve(self.root())
            .await
            .expect("the test resolves the prettier that the repository pins");

        prettier
            .observe(operation, filter)
            .await
            .expect("the test reads a report of prettier")
    }

    /// Returns the root of this project
    ///
    /// The root is canonical, so the paths that a run reports do not depend on
    /// the symbolic links of the temporary directory.
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

/// Returns the filter that selects the Markdown files of a project
fn markdown() -> Filter {
    Filter::new([FileExtension::new("md")])
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
    /// Mise records the trust of a configuration in the state of the user, and
    /// the temporary directory of a test never comes back, so the record would
    /// pile up there forever. A failure stays quiet, because the test already
    /// reported what it was about.
    fn drop(&mut self) {
        let _ = Command::new("mise")
            .arg("trust")
            .arg("--quiet")
            .arg("--untrust")
            .arg(self.directory.path().join("mise.toml"))
            .status();
    }
}

// prettier[verify look.files]
#[tokio::test]
async fn applies_with_a_file_of_the_filter_is_true() {
    let project = Project::bare();
    project.write("sub/clean.md", FORMATTED);

    let applies = project.applies(&markdown()).await;

    assert!(applies);
}

// prettier[verify look.links]
#[cfg(unix)]
#[tokio::test]
async fn applies_with_a_file_only_behind_a_symbolic_link_is_false() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("linked.md"), FORMATTED)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let applies = project.applies(&markdown()).await;

    assert!(!applies);
}

// prettier[verify look.dependencies]
#[tokio::test]
async fn applies_with_a_file_only_under_the_dependencies_is_false() {
    let project = Project::bare();
    project.write("node_modules/package/README.md", FORMATTED);

    let applies = project.applies(&markdown()).await;

    assert!(!applies);
}

// prettier[verify look.git]
#[tokio::test]
async fn applies_with_a_file_only_under_the_git_directory_is_false() {
    let project = Project::bare();
    project.write(".git/description.md", FORMATTED);

    let applies = project.applies(&markdown()).await;

    assert!(!applies);
}

// prettier[verify look.unreadable]
#[cfg(unix)]
#[tokio::test]
async fn applies_with_an_unreadable_directory_is_true() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::bare();
    project.write("closed/notes.txt", FORMATTED);
    let closed = project.directory.path().join("closed");
    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a directory");

    let applies = project.applies(&markdown()).await;

    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o755))
        .expect("the test restores the permissions of a directory");
    assert!(applies);
}

// prettier[verify look.files]
#[tokio::test]
async fn applies_without_a_file_of_the_filter_is_false() {
    let project = Project::bare();
    project.write("main.rs", FOREIGN);

    let applies = project.applies(&markdown()).await;

    assert!(!applies);
}

// prettier[verify run.operation]
#[tokio::test]
async fn observe_a_report_changes_nothing() {
    let project = Project::new();
    project.write("messy.md", UNFORMATTED);

    project.observe(Operation::Report, &markdown()).await;

    assert_eq!(project.content("messy.md"), UNFORMATTED);
}

// prettier[verify run.operation]
#[tokio::test]
async fn observe_a_report_names_a_file_that_is_not_formatted() {
    let project = Project::new();
    project.write("messy.md", UNFORMATTED);

    let observation = project.observe(Operation::Report, &markdown()).await;

    assert_eq!(
        observation
            .problems()
            .first()
            .map(|problem| problem.detail()),
        Some(&ProblemDetail::Unformatted)
    );
}

// prettier[verify run.operation]
#[tokio::test]
async fn observe_a_rewrite_formats_the_project() {
    let project = Project::new();
    project.write("messy.md", UNFORMATTED);

    project.observe(Operation::Rewrite, &markdown()).await;

    assert_eq!(project.content("messy.md"), FORMATTED);
}

// prettier[verify run.plain]
#[tokio::test]
async fn observe_reads_a_report_without_color_codes() {
    let project = Project::new();
    project.write(".prettierrc.json", IGNORED_CONFIGURATION);
    project.write("clean.md", FORMATTED);

    let observation = project.observe(Operation::Report, &markdown()).await;

    assert!(
        !observation.stderr().contains('\u{1b}'),
        "expected a report without an escape code, got {:?}",
        observation.stderr()
    );
}

// prettier[verify run.plain]
#[tokio::test]
async fn observe_reads_the_marker_of_a_line_that_a_color_would_hide() {
    let project = Project::new();
    project.write(".prettierrc.json", IGNORED_CONFIGURATION);
    project.write("clean.md", FORMATTED);

    let observation = project.observe(Operation::Report, &markdown()).await;

    assert!(
        observation.rejected_configuration().is_some(),
        "expected the ignored option, got {:?}",
        observation.stderr()
    );
}

// prettier[verify select.unknown]
#[tokio::test]
async fn observe_with_a_filter_for_every_extension_skips_an_unknown_language() {
    let project = Project::new();
    project.write("main.rs", FOREIGN);

    let observation = project.observe(Operation::Report, &Filter::any()).await;

    assert!(
        observation.succeeded(),
        "expected a run that ended with success, got {:?}",
        observation.stderr()
    );
}

// prettier[verify tool.resolve]
#[tokio::test]
async fn resolve_in_a_project_that_pins_prettier_finds_the_program() {
    let project = Project::new();

    let prettier = Prettier::resolve(project.root()).await;

    assert!(prettier.is_ok(), "expected a prettier, got {prettier:?}");
}

// prettier[verify tool.missing]
#[tokio::test]
async fn resolve_without_a_prettier_reports_the_tool() {
    let project = Project::without_prettier();

    let prettier = Prettier::resolve(project.root()).await;

    let Err(error) = prettier else {
        panic!("expected the resolution to fail, got {prettier:?}");
    };
    assert!(
        error.to_string().contains("prettier"),
        "expected the error to name the tool, got {error}"
    );
}
