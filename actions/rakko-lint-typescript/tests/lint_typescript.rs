//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately broken file sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the oxlint that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers is
//! the version that the repository installs, and a new pin reaches the tests
//! without a change to them.
//!
//! A project states no configuration of oxlint unless the test is about one.
//! Oxlint applies the rules that it enables by itself in that case, and it
//! reads no configuration of the user, so a project without one answers the
//! same way on the machine of every contributor.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Position, ProjectRoot};
use rakko_lint_typescript::LintTypeScript;
use rakko_test_utils::path_text;
use tempfile::TempDir;

/// TypeScript that breaks no rule that oxlint enables by itself
const VALID: &str = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";

/// TypeScript that breaks a rule which oxlint weighs as a warning
///
/// Every rule that oxlint enables by itself carries the warning severity, so a
/// project that states nothing weighs this one that way. Oxlint ends with
/// success for a run that finds only warnings, which is why a run over this
/// file is what holds the action to its own answer instead of to the status of
/// the process.
const DEBUGGER: &str = "export function bad(): void {\n  debugger;\n}\n";

/// A configuration that turns the rule of the broken file off
const PERMISSIVE_CONFIGURATION: &str = "{\n  \"rules\": {\n    \"no-debugger\": \"off\"\n  }\n}\n";

/// A configuration that weighs the rule of the broken file as an error
const STRICT_CONFIGURATION: &str = "{\n  \"rules\": {\n    \"no-debugger\": \"error\"\n  }\n}\n";

/// A configuration that ignores every file of the project
const IGNORE_EVERYTHING: &str = "{\n  \"ignorePatterns\": [\"**\"]\n}\n";

/// A configuration that oxlint refuses to read
const BROKEN_CONFIGURATION: &str = "{ this is not JSON\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without an oxlint to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the oxlint of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the oxlint that
    /// mise resolves for it is the oxlint that the repository pins and installs.
    /// Mise ignores a configuration that nobody trusts, so the copy is trusted
    /// right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project
    }

    /// Creates a project that pins an oxlint that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about oxlint.
    fn without_oxlint() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\noxlint = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend on
    /// the symbolic links of the temporary directory.
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
        LintTypeScript.run(&self.context(), &()).await
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

// linttypescript[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <<LintTypeScript as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

// linttypescript[verify name]
#[test]
fn action_identifies_itself_as_lint_typescript() {
    let name = LintTypeScript.name();

    assert_eq!(name.get(), "lint-typescript");
}

// linttypescript[verify check.passed]
// linttypescript[verify tool.oxlint]
#[tokio::test]
async fn run_in_a_valid_project_passes() {
    let project = Project::new();
    project.write("index.ts", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// linttypescript[verify check.passed]
#[tokio::test]
async fn run_in_a_valid_project_says_how_many_files_oxlint_examined() {
    let project = Project::new();
    project.write("index.ts", VALID);

    let outcome = project.run().await;

    let Outcome::Passed { summary: Some(it) } = &outcome else {
        panic!("expected a summary, got {outcome:?}");
    };
    assert_eq!(it.get(), "checked 1 file");
}

// linttypescript[verify check.read]
#[tokio::test]
async fn run_leaves_a_file_that_breaks_a_rule_unchanged() {
    let project = Project::new();
    project.write("index.ts", DEBUGGER);

    project.run().await;

    assert_eq!(project.read("index.ts"), DEBUGGER);
}

// linttypescript[verify run.project]
#[tokio::test]
async fn run_reaches_a_file_below_a_directory_of_the_project() {
    let project = Project::new();
    project.write("index.ts", VALID);
    project.write("deep/sub/broken.ts", DEBUGGER);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("deep/sub/broken.ts")]);
}

// linttypescript[verify check.configuration]
#[tokio::test]
async fn run_reads_the_configuration_of_the_project() {
    let project = Project::new();
    project.write(".oxlintrc.json", PERMISSIVE_CONFIGURATION);
    project.write("index.ts", DEBUGGER);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the rules of the project to answer, got {outcome:?}"
    );
}

// linttypescript[verify check.configuration]
// linttypescript[verify check.diagnostic]
#[tokio::test]
async fn run_with_a_broken_rule_at_the_error_severity_reports_it() {
    let project = Project::new();
    project.write(".oxlintrc.json", STRICT_CONFIGURATION);
    project.write("index.ts", DEBUGGER);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].message().get(),
        "[error] eslint(no-debugger): `debugger` statement is not allowed \
         help: Remove the debugger statement"
    );
}

// linttypescript[verify check.diagnostic]
// linttypescript[verify run.structured]
#[tokio::test]
async fn run_with_a_broken_rule_carries_the_message_of_oxlint() {
    let project = Project::new();
    project.write("index.ts", DEBUGGER);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].message().get(),
        "[warning] eslint(no-debugger): `debugger` statement is not allowed \
         help: Remove the debugger statement"
    );
}

// linttypescript[verify check.diagnostic]
#[tokio::test]
async fn run_with_a_broken_rule_names_the_file() {
    let project = Project::new();
    project.write("src/index.ts", DEBUGGER);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("src/index.ts")]);
}

// linttypescript[verify check.diagnostic]
#[tokio::test]
async fn run_with_a_broken_rule_reports_the_position_of_oxlint() {
    let project = Project::new();
    project.write("index.ts", DEBUGGER);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::Position {
            path: "index.ts".parse().expect("the test names a relative path"),
            position: Position::builder().line(2).column(3).build(),
        }
    );
}

// linttypescript[verify check.unreported]
#[tokio::test]
async fn run_with_a_refused_configuration_stops() {
    let project = Project::new();
    project.write(".oxlintrc.json", BROKEN_CONFIGURATION);
    project.write("index.ts", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// A rule that oxlint weighs as a warning leaves it ending with success, so a
// run that answered from the status of the process would pass this project.
// linttypescript[verify check.severity]
#[tokio::test]
async fn run_with_a_warning_fails() {
    let project = Project::new();
    project.write("index.ts", DEBUGGER);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// linttypescript[verify skip.unexamined]
#[tokio::test]
async fn run_whose_oxlint_examined_nothing_skips() {
    let project = Project::new();
    project.write(".oxlintrc.json", IGNORE_EVERYTHING);
    project.write("index.ts", DEBUGGER);

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("oxlint"),
        "expected the reason to name the tool, got {reason:?}"
    );
}

// A project that holds no TypeScript is the project that this action skips, and
// oxlint reports that it found nothing to lint rather than that it found no
// problem.
// linttypescript[verify skip.unexamined]
#[tokio::test]
async fn run_without_a_file_that_oxlint_lints_skips() {
    let project = Project::new();
    project.write("README.md", "# Notes\n");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// linttypescript[verify tool.missing]
#[tokio::test]
async fn run_without_an_oxlint_stops() {
    let project = Project::without_oxlint();
    project.write("index.ts", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}
