//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately unsafe workflow sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the zizmor that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers is
//! the version that the repository installs, and a new pin reaches the tests
//! without a change to them.
//!
//! A project writes no configuration of zizmor. Zizmor needs none, and the
//! audits that it applies without one are the audits that these tests are
//! about. The tests that are about a configuration write their own.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Position, Span};
use rakko_lint_github_actions::LintGitHubActions;
use tempfile::TempDir;

/// A workflow that no audit of the pedantic persona reports
const CLEAN: &str = "\
---
name: Clean
on:
  push:
permissions: {}
concurrency:
  group: clean
  cancel-in-progress: true
jobs:
  build:
    name: Build the project
    runs-on: ubuntu-latest
    permissions: {}
    steps:
      - name: Say hello
        run: echo hi
";

/// A workflow that expands attacker-controlled text into a shell
///
/// The audit that reports it belongs to the regular persona, so the finding
/// arrives whatever persona a run asks for.
const INJECTION: &str = "\
---
name: Injection
on:
  push:
permissions: {}
concurrency:
  group: injection
  cancel-in-progress: true
jobs:
  build:
    name: Build the project
    runs-on: ubuntu-latest
    permissions: {}
    steps:
      - name: Echo the message
        run: echo \"${{ github.event.head_commit.message }}\"
";

/// A workflow that only an audit of the pedantic persona reports
///
/// The workflow is [`CLEAN`] without its concurrency block, which the regular
/// persona says nothing about.
const PEDANTIC: &str = "\
---
name: Pedantic
on:
  push:
permissions: {}
jobs:
  build:
    name: Build the project
    runs-on: ubuntu-latest
    permissions: {}
    steps:
      - name: Say hello
        run: echo hi
";

/// A workflow that zizmor cannot read
const UNREADABLE: &str = "---\nname: Broken\non: [push\njobs: : :\n";

/// A configuration that turns the audit of the unsafe workflow off
const PERMISSIVE_CONFIGURATION: &str = "---\nrules:\n  template-injection:\n    disable: true\n";

/// A configuration that zizmor refuses to read
const BROKEN_CONFIGURATION: &str = "---\nno-such-key: true\n";

/// A rule of the version control system that hides the workflows of a project
const IGNORE_WORKFLOWS: &str = ".github/\n";

/// The place that GitHub reads the workflows of a project from
const WORKFLOWS: &str = ".github/workflows";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a zizmor to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool
    /// runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the zizmor of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the zizmor
    /// that mise resolves for it is the zizmor that the repository pins and
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

    /// Creates a project that pins a zizmor that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about zizmor.
    fn without_zizmor() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\nzizmor = \"0.0.1\"\n")
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
        LintGitHubActions.run(&self.context(), &()).await
    }

    /// Writes a file of the project, with the directories that lead to it
    fn write(&self, path: &str, content: &str) {
        let path = self.directory.path().join(path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the test creates the directories of a file");
        }

        std::fs::write(&path, content).expect("the test writes a file of the project");
    }

    /// Writes a workflow of the project
    fn workflow(&self, name: &str, content: &str) {
        self.write(&format!("{WORKFLOWS}/{name}"), content);
    }
}

/// Returns the messages of the given findings
fn messages(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.message().get())
        .collect()
}

/// Returns the paths that the findings of an outcome name
fn paths(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::Span { path, .. } => path.to_string(),
            other => panic!("expected a finding that covers a range, got {other:?}"),
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

// lintgithubactions[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <<LintGitHubActions as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

// lintgithubactions[verify name]
#[test]
fn action_identifies_itself_as_lint_github_actions() {
    let name = LintGitHubActions.name();

    assert_eq!(name.get(), "lint-github-actions");
}

// lintgithubactions[verify check.passed]
// lintgithubactions[verify tool.zizmor]
#[tokio::test]
async fn run_in_a_clean_project_passes() {
    let project = Project::new();
    project.workflow("clean.yml", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// lintgithubactions[verify skip.missing]
#[tokio::test]
async fn run_in_a_project_without_a_workflow_directory_skips() {
    let project = Project::without_zizmor();
    project.write("README.md", "# Notes\n");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// lintgithubactions[verify skip.missing]
#[tokio::test]
async fn run_in_a_project_without_a_workflow_names_what_it_looked_for() {
    let project = Project::without_zizmor();
    project.write(&format!("{WORKFLOWS}/notes.txt"), "not a workflow\n");

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert_eq!(
        reason.get(),
        "the .github/workflows directory of the project holds no .yaml or .yml file"
    );
}

// lintgithubactions[verify check.read]
#[tokio::test]
async fn run_leaves_a_workflow_with_a_finding_unchanged() {
    let project = Project::new();
    project.workflow("injection.yml", INJECTION);

    project.run().await;

    assert_eq!(project.read(".github/workflows/injection.yml"), INJECTION);
}

// lintgithubactions[verify run.project]
#[tokio::test]
async fn run_names_the_project_and_reports_the_path_below_it() {
    let project = Project::new();
    project.workflow("injection.yaml", INJECTION);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        paths(findings)
            .iter()
            .all(|path| path == ".github/workflows/injection.yaml"),
        "expected the path below the project, got {:?}",
        paths(findings)
    );
}

// lintgithubactions[verify run.persona]
#[tokio::test]
async fn run_reports_a_finding_of_the_pedantic_persona() {
    let project = Project::new();
    project.workflow("pedantic.yml", PEDANTIC);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        messages(findings)
            .iter()
            .any(|message| message.contains("concurrency-limits")),
        "expected an audit of the pedantic persona, got {:?}",
        messages(findings)
    );
}

// lintgithubactions[verify run.structured]
#[tokio::test]
async fn run_reads_the_configuration_of_the_project() {
    let project = Project::new();
    project.write("zizmor.yml", PERMISSIVE_CONFIGURATION);
    project.workflow("injection.yml", INJECTION);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the audits of the project to answer, got {outcome:?}"
    );
}

// lintgithubactions[verify check.finding]
// lintgithubactions[verify check.severity]
#[tokio::test]
async fn run_with_a_finding_carries_the_message_of_zizmor() {
    let project = Project::new();
    project.workflow("injection.yml", INJECTION);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        messages(findings).contains(
            &"[high] template-injection: code injection via template expansion \
              (may expand into attacker-controllable code)"
        ),
        "expected the message of zizmor, got {:?}",
        messages(findings)
    );
}

// lintgithubactions[verify check.finding]
#[tokio::test]
async fn run_with_a_finding_covers_the_range_of_zizmor() {
    let project = Project::new();
    project.workflow("injection.yml", INJECTION);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings.iter().any(|finding| finding.location()
            == &Location::Span {
                path: ".github/workflows/injection.yml"
                    .parse()
                    .expect("the test names a relative path"),
                span: Span::builder()
                    .start(Position::builder().line(16).column(24).build())
                    .end(Position::builder().line(16).column(56).build())
                    .build(),
            }),
        "expected the range of zizmor, got {:?}",
        findings.iter().map(Finding::location).collect::<Vec<_>>()
    );
}

// lintgithubactions[verify check.finding]
#[tokio::test]
async fn run_with_a_finding_reports_every_place_that_zizmor_named() {
    let project = Project::new();
    project.workflow("pedantic.yml", PEDANTIC);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        messages(findings),
        [
            "[low] concurrency-limits: insufficient job-level concurrency limits \
             (workflow is missing concurrency setting)",
            "[low] concurrency-limits: insufficient job-level concurrency limits \
             (job affected by missing workflow concurrency)",
        ]
    );
}

// lintgithubactions[verify check.configuration]
#[tokio::test]
async fn run_with_a_refused_configuration_stops() {
    let project = Project::new();
    project.write("zizmor.yml", BROKEN_CONFIGURATION);
    project.workflow("clean.yml", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// lintgithubactions[verify check.incomplete]
// lintgithubactions[verify run.strict]
#[tokio::test]
async fn run_with_a_workflow_that_zizmor_cannot_read_stops() {
    let project = Project::new();
    project.workflow("clean.yml", CLEAN);
    project.workflow("unreadable.yml", UNREADABLE);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// lintgithubactions[verify skip.uncollected]
#[tokio::test]
async fn run_whose_zizmor_collects_nothing_skips() {
    let project = Project::new();
    project.workflow("clean.yml", CLEAN);
    project.write(".gitignore", IGNORE_WORKFLOWS);

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert_eq!(reason.get(), "zizmor found no file to audit");
}

// lintgithubactions[verify tool.missing]
#[tokio::test]
async fn run_without_a_zizmor_stops() {
    let project = Project::without_zizmor();
    project.workflow("clean.yml", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// lintgithubactions[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_a_workflow_behind_a_symbolic_link_skips() {
    let project = Project::without_zizmor();
    project.write("elsewhere/clean.yml", CLEAN);
    project.write(&format!("{WORKFLOWS}/.keep"), "");
    std::os::unix::fs::symlink(
        project.directory.path().join("elsewhere/clean.yml"),
        project.directory.path().join(WORKFLOWS).join("clean.yml"),
    )
    .expect("the test links a workflow into the workflow directory");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}
