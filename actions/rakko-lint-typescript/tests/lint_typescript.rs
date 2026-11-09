//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. So no fixture with a deliberately broken file sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the oxlint that the action declares. Each project pins
//! another oxlint that nothing installs, so a test passes only when the run
//! ignores the pin of the project.
//!
//! A project states no configuration of oxlint unless the test is about one.
//! Oxlint applies the rules that it enables by itself in that case. It reads no
//! configuration of the user. So a project without one answers the same way on
//! the machine of every contributor.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko::{Action, Args, Context, Finding, Location, Outcome, Position, ProjectRoot};
use rakko_lint_typescript::{LintTypeScript, LintTypeScriptError};
use rakko_test_utils::{path_text, run_checking_access};
use rakko_tool::ResolveRequirementError;
use tempfile::TempDir;

/// TypeScript that breaks no rule that oxlint enables by itself
const VALID: &str = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";

/// TypeScript that breaks a rule which oxlint weighs as a warning
///
/// Every rule that oxlint enables by itself carries the warning severity, so a
/// project that states nothing weighs this one that way. Oxlint ends with
/// success for a run that finds only warnings. This is why a run over this file
/// is what holds the action to its own answer instead of to the status of the
/// process.
const DEBUGGER: &str = "export function bad(): void {\n  debugger;\n}\n";

/// TypeScript that breaks the rule of the broken file twice
const TWO_DEBUGGERS: &str = "export function bad(): void {\n  debugger;\n  debugger;\n}\n";

/// A suppressions file that accepts one break of the rule of the broken file
///
/// Oxlint reads the file in the root of the project, and it compares the
/// count with the code. A suppression covers only a rule that the project
/// weighs as an error, so a project with this file uses the strict
/// configuration as well.
const SUPPRESSIONS: &str =
    "{\n  \"index.ts\": {\n    \"no-debugger\": {\n      \"count\": 1\n    }\n  }\n}\n";

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

#[bon::bon]
impl Project {
    /// Creates a project that pins an oxlint that nothing installed
    ///
    /// The pin holds a version that no machine installs. The run resolves the
    /// oxlint that the action declares, so the pin of the project must not
    /// change what runs. Mise ignores a configuration that nobody trusts, so
    /// the file is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

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
    let schema = <<LintTypeScript as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

#[test]
fn action_identifies_itself_as_lint_typescript() {
    let name = LintTypeScript.name();

    assert_eq!(name.get(), "lint-typescript");
}

#[tokio::test]
async fn run_in_a_missing_directory_stops_without_an_oxlint() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();

    let outcome = LintTypeScript.run(&context, &()).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(LintTypeScriptError::UnresolvedTool {
                        source: ResolveRequirementError::MiseUnavailable { tool, .. },
                    }) if tool.get() == "oxlint"
                )
        ),
        "expected the run to stop without an oxlint, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_valid_project_passes() {
    let project = Project::new();
    project.write().path("index.ts").content(VALID).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_valid_project_says_how_many_files_oxlint_examined() {
    let project = Project::new();
    project.write().path("index.ts").content(VALID).call();

    let outcome = project.run().await;

    let Outcome::Passed { summary: Some(it) } = &outcome else {
        panic!("expected a summary, got {outcome:?}");
    };
    assert_eq!(it.get(), "checked 1 file");
}

#[tokio::test]
async fn run_leaves_a_file_that_breaks_a_rule_unchanged() {
    let project = Project::new();
    project.write().path("index.ts").content(DEBUGGER).call();

    project.run().await;

    assert_eq!(project.read("index.ts"), DEBUGGER);
}

#[tokio::test]
async fn run_reaches_a_file_below_a_directory_of_the_project() {
    let project = Project::new();
    project.write().path("index.ts").content(VALID).call();
    project
        .write()
        .path("deep/sub/broken.ts")
        .content(DEBUGGER)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("deep/sub/broken.ts")]);
}

#[tokio::test]
async fn run_reads_the_configuration_of_the_project() {
    let project = Project::new();
    project
        .write()
        .path(".oxlintrc.json")
        .content(PERMISSIVE_CONFIGURATION)
        .call();
    project.write().path("index.ts").content(DEBUGGER).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the rules of the project to answer, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_whose_oxlint_examined_nothing_skips() {
    let project = Project::new();
    project
        .write()
        .path(".oxlintrc.json")
        .content(IGNORE_EVERYTHING)
        .call();
    project.write().path("index.ts").content(DEBUGGER).call();

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("oxlint"),
        "expected the reason to name the tool, got {reason:?}"
    );
}

#[tokio::test]
async fn run_with_a_broken_rule_at_the_error_severity_reports_it() {
    let project = Project::new();
    project
        .write()
        .path(".oxlintrc.json")
        .content(STRICT_CONFIGURATION)
        .call();
    project.write().path("index.ts").content(DEBUGGER).call();

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

#[tokio::test]
async fn run_with_a_broken_rule_carries_the_message_of_oxlint() {
    let project = Project::new();
    project.write().path("index.ts").content(DEBUGGER).call();

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

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_with_a_broken_rule_leaves_the_project_unchanged() {
    let project = Project::new();
    project.write().path("index.ts").content(DEBUGGER).call();

    run_checking_access(&LintTypeScript, &project.context(), &()).await;
}

#[tokio::test]
async fn run_with_a_broken_rule_names_the_file() {
    let project = Project::new();
    project
        .write()
        .path("src/index.ts")
        .content(DEBUGGER)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("src/index.ts")]);
}

#[tokio::test]
async fn run_with_a_broken_rule_reports_the_position_of_oxlint() {
    let project = Project::new();
    project.write().path("index.ts").content(DEBUGGER).call();

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

#[tokio::test]
async fn run_with_a_refused_configuration_stops() {
    let project = Project::new();
    project
        .write()
        .path(".oxlintrc.json")
        .content(BROKEN_CONFIGURATION)
        .call();
    project.write().path("index.ts").content(VALID).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

/// Shows that stale suppressions are reported
///
/// Oxlint reports a suppression that the code no longer needs as a problem of
/// the project, with no rule and no file.
#[tokio::test]
async fn run_with_a_stale_suppression_reports_it_for_the_project() {
    let project = Project::new();
    project
        .write()
        .path(".oxlintrc.json")
        .content(STRICT_CONFIGURATION)
        .call();
    project
        .write()
        .path("oxlint-suppressions.json")
        .content(SUPPRESSIONS)
        .call();
    project.write().path("index.ts").content(VALID).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings.iter().map(Finding::location).collect::<Vec<_>>(),
        [&Location::Project]
    );
}

/// Shows that warnings cause runs to fail
///
/// A rule that oxlint weighs as a warning leaves it ending with success. So a
/// run that answered from the status of the process would pass this project.
#[tokio::test]
async fn run_with_a_warning_fails() {
    let project = Project::new();
    project.write().path("index.ts").content(DEBUGGER).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

/// Shows that uncovered violations are kept as findings
///
/// Oxlint reports every break of a rule whose count the suppressions file no
/// longer covers, and then a problem of the project. The breaks of the file stay
/// findings beside it.
#[tokio::test]
async fn run_with_an_uncovered_violation_keeps_the_findings_of_the_file() {
    let project = Project::new();
    project
        .write()
        .path(".oxlintrc.json")
        .content(STRICT_CONFIGURATION)
        .call();
    project
        .write()
        .path("oxlint-suppressions.json")
        .content(SUPPRESSIONS)
        .call();
    project
        .write()
        .path("index.ts")
        .content(TWO_DEBUGGERS)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings.iter().map(Finding::location).collect::<Vec<_>>(),
        [
            &Location::Position {
                path: "index.ts".parse().expect("the test names a relative path"),
                position: Position::builder().line(2).column(3).build(),
            },
            &Location::Position {
                path: "index.ts".parse().expect("the test names a relative path"),
                position: Position::builder().line(3).column(3).build(),
            },
            &Location::Project,
        ]
    );
}

/// Shows that projects without TypeScript files are skipped
///
/// A project that holds no TypeScript is the project that this action skips.
/// Oxlint reports that it found nothing to lint rather than that it found no
/// problem.
#[tokio::test]
async fn run_without_a_file_that_oxlint_lints_skips() {
    let project = Project::new();
    project
        .write()
        .path("README.md")
        .content("# Notes\n")
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}
