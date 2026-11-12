//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. So no fixture with a deliberately broken Markdown file sits in
//! this repository, where the checks of the repository itself would fight it.
//!
//! The tests run the markdownlint that the action declares. A project copies the
//! `mise.toml` of the repository and trusts it. So the Node that answers is
//! the one that the repository installs. The copy pins another markdownlint that
//! nothing installs. So a test passes only when the run ignores the pin of
//! the project.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko::{Action, Args, Context, Finding, Location, Outcome, Position, ProjectRoot};
use rakko_lint_markdown::{LintMarkdown, LintMarkdownError};
use rakko_test_utils::{path_text, run_checking_access};
use rakko_tool::ResolveRequirementError;
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

/// A markdownlint configuration that no reader of markdownlint can parse
///
/// The list of the value has no closing bracket. A markdownlint that finds
/// the file on its own treats it as a file that is not there.
const UNPARSABLE_CONFIGURATION: &str = "MD013:\n  line_length: [60\n";

/// A JSON configuration that no reader of markdownlint can parse
///
/// The object has no closing brace.
const UNPARSABLE_JSON_CONFIGURATION: &str = "{ \"MD018\": false\n";

/// A JSONC configuration that turns the rule of the heading off, with
/// comments and trailing commas
const TRAILING_COMMA_CONFIGURATION: &str = "{\n  // The headings of the tests have no space.\n  \"MD018\": false,\n  \"MD041\": false,\n}\n";

/// A markdownlint configuration that extends another file of the project
const EXTENDING_CONFIGURATION: &str = "extends: base.yaml\nMD041: false\n";

/// A markdownlint configuration that turns the rule of the heading off
///
/// A test writes it to the file that another configuration extends.
const BASE_CONFIGURATION: &str = "MD018: false\n";

/// A markdownlint configuration of the user that turns the rule of the
/// heading off
const RC_CONFIGURATION: &str = "{ \"MD018\": false }\n";

/// An ignore file that excludes the whole project
const IGNORE_EVERYTHING: &str = "*.md\n";

/// An ignore file that excludes a directory whose name starts with a dot
const IGNORE_DOT_DIRECTORY: &str = ".claude/\n";

/// The key that pins markdownlint in the mise configuration of a project
const TOOL_PIN: &str = "\"npm:markdownlint-cli\"";

/// The header of the table of tools in a mise configuration
const TOOLS_TABLE: &str = "[tools]\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project with the Node of this repository and a markdownlint that
    /// nothing installed
    ///
    /// The project copies the `mise.toml` of this repository, so the Node that
    /// mise resolves for it is the Node that the repository pins and installs.
    /// The copy pins markdownlint at a version that no machine installs. The
    /// run resolves the markdownlint that the action declares, so the pin of
    /// the project must not change what runs. Mise ignores a configuration that
    /// nobody trusts, so the copy is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        let pins = std::fs::read_to_string(repository().join("mise.toml"))
            .expect("the test reads the mise.toml of the repository");
        let (head, tail) = pins
            .split_once(TOOLS_TABLE)
            .expect("the mise.toml of the repository holds a table of tools");
        let copy = project.directory.path().join("mise.toml");
        std::fs::write(
            &copy,
            format!("{head}{TOOLS_TABLE}{TOOL_PIN} = \"0.0.1\"\n{tail}"),
        )
        .expect("the test writes the mise.toml of the project");
        trust(&copy);

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
    #[builder]
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
    /// Mise records the trust of a configuration in the state of the user. The
    /// temporary directory of a test never comes back, so the record would pile
    /// up there forever. A project that never wrote a configuration has nothing
    /// to withdraw. Mise resolves a path that is not there to the directory
    /// above it, so the file has to exist first. A failure stays quiet, because
    /// the test already reported what it was about.
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

#[test]
fn action_declares_no_argument() {
    let schema = <<LintMarkdown as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

#[test]
fn action_identifies_itself_as_lint_markdown() {
    let name = LintMarkdown.name();

    assert_eq!(name.get(), "lint-markdown");
}

#[tokio::test]
async fn run_in_a_missing_directory_stops_without_a_markdownlint() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();

    let outcome = LintMarkdown.run(&context, &()).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(LintMarkdownError::UnresolvedTool { .. }))
        ),
        "expected the run to stop without a markdownlint, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_without_node_stops_on_node() {
    let directory = tempfile::tempdir().expect("the test creates a temporary directory");
    let context = Context::builder().root(directory.path()).build();

    let outcome = LintMarkdown.run(&context, &()).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(LintMarkdownError::UnresolvedTool {
                        source: ResolveRequirementError::UnresolvedToolchain { tool, .. },
                    }) if tool.get() == "node"
                )
        ),
        "expected the run to stop without a Node, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_valid_project_passes() {
    let project = Project::new();
    project.write().path("notes.md").content(VALID).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_leaves_a_file_that_breaks_a_rule_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    project.run().await;

    assert_eq!(project.read("notes.md"), UNSPACED_HEADING);
}

#[tokio::test]
async fn run_leaves_out_a_directory_that_the_ignore_file_names() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content("MD041: false\n")
        .call();
    project
        .write()
        .path(".markdownlintignore")
        .content(IGNORE_DOT_DIRECTORY)
        .call();
    project.write().path("notes.md").content(VALID).call();
    project
        .write()
        .path(".claude/notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the ignore file to leave the directory out, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_leaves_out_the_directories_of_git() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content("MD041: false\n")
        .call();
    project.write().path("notes.md").content(VALID).call();
    project
        .write()
        .path(".git/notes.md")
        .content(UNSPACED_HEADING)
        .call();
    project
        .write()
        .path("vendor/.git/notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to leave the directories of git out, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_reaches_a_file_below_a_directory_of_the_project() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content("MD041: false\n")
        .call();
    project.write().path("notes.md").content(VALID).call();
    project
        .write()
        .path("deep/sub/notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("deep/sub/notes.md")]);
}

#[tokio::test]
async fn run_reaches_a_file_in_a_directory_with_a_dot() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content("MD041: false\n")
        .call();
    project.write().path("notes.md").content(VALID).call();
    project
        .write()
        .path(".changeset/notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text(".changeset/notes.md")]);
}

/// Shows that the run applies a configuration file that the configuration of
/// the project extends
#[tokio::test]
async fn run_reads_a_configuration_that_the_configuration_of_the_project_extends() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content(EXTENDING_CONFIGURATION)
        .call();
    project
        .write()
        .path("base.yaml")
        .content(BASE_CONFIGURATION)
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the rules of the extended file to answer, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_reads_the_configuration_of_the_project() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content(CONFIGURATION)
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the rules of the project to answer, got {outcome:?}"
    );
}

/// Shows that the run applies a `.markdownlintrc` beside the configuration
/// file of the project
///
/// Markdownlint reads the `.markdownlintrc` first and adds the configuration
/// file of the project on top of it.
#[tokio::test]
async fn run_reads_the_markdownlintrc_beside_the_configuration_of_the_project() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlintrc")
        .content(RC_CONFIGURATION)
        .call();
    project
        .write()
        .path(".markdownlint.yaml")
        .content("MD041: false\n")
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the rules of both files to answer, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_whose_markdownlint_examined_nothing_skips() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlintignore")
        .content(IGNORE_EVERYTHING)
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("markdownlint"),
        "expected the reason to name the tool, got {reason:?}"
    );
}

/// Shows that the run stops on a file that the configuration of the project
/// extends, when markdownlint cannot parse that file
#[tokio::test]
async fn run_with_a_broken_configuration_that_the_project_extends_stops() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content(EXTENDING_CONFIGURATION)
        .call();
    project
        .write()
        .path("base.yaml")
        .content(UNPARSABLE_CONFIGURATION)
        .call();
    project.write().path("notes.md").content(VALID).call();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(LintMarkdownError::RejectedConfiguration { path, .. })
                        if path == Path::new(".markdownlint.yaml")
                )
        ),
        "expected the run to stop on the extended file, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_broken_rule_carries_the_message_of_markdownlint() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content("MD041: false\n")
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].message().get(),
        "MD018/no-missing-space-atx No space after hash on atx style heading [Context: \"#Notes\"]"
    );
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_with_a_broken_rule_leaves_the_project_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    run_checking_access(&LintMarkdown, &project.context(), &()).await;
}

/// Shows that run names a nested file with the separator of the platform
///
/// Markdownlint writes a path with a forward slash on every platform, and the
/// action rebuilds it with the separator of the platform.
#[tokio::test]
async fn run_with_a_broken_rule_names_the_file() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content("MD041: false\n")
        .call();
    project
        .write()
        .path("sub/notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("sub/notes.md")]);
}

/// Shows that a later configuration file does not hide one that markdownlint
/// cannot parse
///
/// Markdownlint takes the next of its file names when it cannot parse the
/// first one that exists. The run stops on the first one, because the project
/// wrote it to be read.
#[tokio::test]
async fn run_with_a_configuration_that_markdownlint_cannot_parse_before_a_valid_one_stops() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.json")
        .content(UNPARSABLE_JSON_CONFIGURATION)
        .call();
    project
        .write()
        .path(".markdownlint.yaml")
        .content(CONFIGURATION)
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(LintMarkdownError::RejectedConfiguration { path, .. })
                        if path == Path::new(".markdownlint.json")
                )
        ),
        "expected the run to stop on the first configuration, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_configuration_that_markdownlint_cannot_parse_reports_its_message() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content(UNPARSABLE_CONFIGURATION)
        .call();
    project.write().path("notes.md").content(VALID).call();

    let outcome = project.run().await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("Unable to parse")
            && source.to_string().contains(".markdownlint.yaml"),
        "expected the message of markdownlint about the file, got {source}"
    );
}

#[tokio::test]
async fn run_with_a_configuration_that_markdownlint_cannot_parse_stops() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content(UNPARSABLE_CONFIGURATION)
        .call();
    project.write().path("notes.md").content(VALID).call();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(LintMarkdownError::RejectedConfiguration { path, .. })
                        if path == Path::new(".markdownlint.yaml")
                )
        ),
        "expected the run to stop on the configuration, got {outcome:?}"
    );
}

/// Shows that a configuration that markdownlint accepts on its own still
/// applies
///
/// Markdownlint parses a JSONC file with a trailing comma. A file that its
/// `--config` option gives directly is first parsed as JSON with comments,
/// and that parser stops on the comma.
#[tokio::test]
async fn run_with_a_jsonc_configuration_with_a_trailing_comma_applies_it() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.jsonc")
        .content(TRAILING_COMMA_CONFIGURATION)
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the rules of the project to answer, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_rule_that_names_a_column_reports_it() {
    let project = Project::new();
    project
        .write()
        .path(".markdownlint.yaml")
        .content("MD041: false\n")
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNSPACED_HEADING)
        .call();

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

#[tokio::test]
async fn run_with_a_rule_that_names_no_column_reports_the_line_alone() {
    let project = Project::new();
    project.write().path("notes.md").content(BLANK_LINES).call();

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

#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_stops() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write().path("secret.md").content(VALID).call();
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

#[tokio::test]
async fn run_without_markdown_files_skips() {
    let project = Project::new();
    project
        .write()
        .path("notes.txt")
        .content("Not Markdown.\n")
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}
