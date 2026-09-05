//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately broken Markdown file sits in
//! this repository, where the checks of the repository itself would fight it.
//!
//! The tests run the markdownlint that this repository pins. A project copies
//! the `mise.toml` of the repository and trusts it, so the version that
//! answers is the version that the repository installs, and a new pin reaches
//! the tests without a change to them.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Position};
use rakko_lint_markdown::LintMarkdown;
use tempfile::TempDir;

/// A Markdown document that markdownlint accepts
const VALID: &str = "# Notes\n\nA paragraph of prose.\n";

/// A Markdown document that breaks a rule which points at a column
///
/// The heading has no space after its hash, which MD018 reports at the
/// character that it wants the space in front of.
const UNSPACED_HEADING: &str = "#Notes\n\nA paragraph of prose.\n";

/// A Markdown document that breaks a rule which points at no column
///
/// The blank lines are more than one, which MD012 reports for the line and
/// nothing in it.
const BLANK_LINES: &str = "# Notes\n\n\n\nA paragraph of prose.\n";

/// A markdownlint configuration that turns the rule of the heading off
const CONFIGURATION: &str = "MD018: false\nMD041: false\n";

/// An ignore file that excludes the whole project
const IGNORE_EVERYTHING: &str = "*.md\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a markdownlint to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool
    /// runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the markdownlint of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the
    /// markdownlint that mise resolves for it is the markdownlint that the
    /// repository pins and installs, on the node that it pins next to it.
    /// Mise ignores a configuration that nobody trusts, so the copy is
    /// trusted right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project
    }

    /// Creates a project that pins a markdownlint that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about markdownlint.
    fn without_markdownlint() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\n\"npm:markdownlint-cli\" = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        let root = self
            .directory
            .path()
            .canonicalize()
            .expect("the test names a directory that exists");

        Context::builder().root(root.as_path()).build()
    }

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        LintMarkdown.run(&self.context(), &()).await
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

/// Returns the paths that the findings of an outcome name
fn locations(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::File { path } => path.to_string(),
            Location::Position { path, .. } => path.to_string(),
            other => panic!("expected a finding with a path, got {other:?}"),
        })
        .collect()
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
    /// pile up there forever. A project that never wrote a configuration has
    /// nothing to withdraw, and mise resolves a path that is not there to the
    /// directory above it, so the file has to exist first. A failure stays
    /// quiet, because the test already reported what it was about.
    fn drop(&mut self) {
        let pins = self.directory.path().join("mise.toml");

        if !pins.exists() {
            return;
        }

        let _ = Command::new("mise")
            .arg("trust")
            .arg("--quiet")
            .arg("--untrust")
            .arg(pins)
            .status();
    }
}

// lintmarkdown[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <<LintMarkdown as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

// lintmarkdown[verify name]
#[test]
fn action_identifies_itself_as_lint_markdown() {
    let name = LintMarkdown.name();

    assert_eq!(name.get(), "lint-markdown");
}

// lintmarkdown[verify check.read]
#[tokio::test]
async fn run_leaves_a_file_that_breaks_a_rule_unchanged() {
    let project = Project::new();
    project.write("notes.md", UNSPACED_HEADING);

    project.run().await;

    assert_eq!(project.read("notes.md"), UNSPACED_HEADING);
}

// lintmarkdown[verify check.passed]
// lintmarkdown[verify tool.markdownlint]
#[tokio::test]
async fn run_in_a_valid_project_passes() {
    let project = Project::new();
    project.write("notes.md", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// lintmarkdown[verify run.structured]
#[tokio::test]
async fn run_reads_the_configuration_of_the_project() {
    let project = Project::new();
    project.write(".markdownlint.yaml", CONFIGURATION);
    project.write("notes.md", UNSPACED_HEADING);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the rules of the project to answer, got {outcome:?}"
    );
}

// lintmarkdown[verify check.violation]
// lintmarkdown[verify run.structured]
#[tokio::test]
async fn run_with_a_broken_rule_carries_the_message_of_markdownlint() {
    let project = Project::new();
    project.write(".markdownlint.yaml", "MD041: false\n");
    project.write("notes.md", UNSPACED_HEADING);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].message().get(),
        "MD018/no-missing-space-atx No space after hash on atx style heading [Context: \"#Notes\"]"
    );
}

// lintmarkdown[verify check.violation]
#[tokio::test]
async fn run_with_a_broken_rule_names_the_file() {
    let project = Project::new();
    project.write(".markdownlint.yaml", "MD041: false\n");
    project.write("sub/notes.md", UNSPACED_HEADING);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["sub/notes.md"]);
}

// lintmarkdown[verify check.column]
#[tokio::test]
async fn run_with_a_rule_that_names_a_column_reports_it() {
    let project = Project::new();
    project.write(".markdownlint.yaml", "MD041: false\n");
    project.write("notes.md", UNSPACED_HEADING);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::Position {
            path: "notes.md".parse().expect("the test names a relative path"),
            position: Position::builder().line(1).column(1).build(),
        }
    );
}

// lintmarkdown[verify check.column]
#[tokio::test]
async fn run_with_a_rule_that_names_no_column_reports_the_line_alone() {
    let project = Project::new();
    project.write("notes.md", BLANK_LINES);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::Position {
            path: "notes.md".parse().expect("the test names a relative path"),
            position: Position::builder().line(3).build(),
        }
    );
}

// lintmarkdown[verify check.unreadable]
#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_stops() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write("secret.md", VALID);
    let path = project.directory.path().join("secret.md");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run().await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// lintmarkdown[verify skip.unexamined]
#[tokio::test]
async fn run_whose_markdownlint_examined_nothing_skips() {
    let project = Project::new();
    project.write(".markdownlintignore", IGNORE_EVERYTHING);
    project.write("notes.md", UNSPACED_HEADING);

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("markdownlint"),
        "expected the reason to name the tool, got {reason:?}"
    );
}

// lintmarkdown[verify skip.hidden]
#[tokio::test]
async fn run_with_markdown_only_under_a_hidden_directory_skips() {
    let project = Project::bare();
    project.write(".github/NOTES.md", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// lintmarkdown[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_markdown_only_behind_a_symbolic_link_skips() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("linked.md"), VALID)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// lintmarkdown[verify tool.missing]
#[tokio::test]
async fn run_without_a_markdownlint_stops() {
    let project = Project::without_markdownlint();
    project.write("notes.md", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// lintmarkdown[verify skip.missing]
#[tokio::test]
async fn run_without_markdown_files_names_what_it_looked_for() {
    let project = Project::bare();
    project.write("notes.txt", "Not Markdown.\n");

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains(".md"),
        "expected the reason to name the extension, got {reason:?}"
    );
}

// lintmarkdown[verify skip.missing]
#[tokio::test]
async fn run_without_markdown_files_skips() {
    let project = Project::bare();
    project.write("notes.txt", "Not Markdown.\n");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// lintmarkdown[verify run.project]
#[tokio::test]
async fn run_reaches_a_file_below_a_directory_of_the_project() {
    let project = Project::new();
    project.write(".markdownlint.yaml", "MD041: false\n");
    project.write("notes.md", VALID);
    project.write("deep/sub/notes.md", UNSPACED_HEADING);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["deep/sub/notes.md"]);
}
