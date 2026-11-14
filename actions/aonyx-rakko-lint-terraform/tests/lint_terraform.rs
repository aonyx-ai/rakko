//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. So no fixture with a deliberately broken module sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the TFLint that the action declares. Each project pins
//! another TFLint that nothing installs, so a test passes only when the run
//! ignores the pin of the project.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use aonyx_rakko_lint_terraform::{LintTerraform, LintTerraformError};
use rakko::{Action, Context, Finding, Location, Outcome, Position, ProjectRoot, Span};
use rakko_test_utils::{path_text, run_checking_access};
use tempfile::TempDir;

/// A module that TFLint finds nothing in
const CLEAN: &str = "terraform {\n  required_version = \">= 1.0\"\n}\n";

/// A module with a variable that has no type and that nothing reads
const UNTYPED: &str = "terraform {\n  required_version = \">= 1.0\"\n}\n\nvariable \"name\" {}\n";

/// A module that Terraform cannot parse
const INVALID: &str = "resource \"x\" {\n";

/// A configuration of TFLint that turns off the rules about variables
const QUIET: &str = "rule \"terraform_typed_variables\" {\n  enabled = false\n}\n\nrule \
                     \"terraform_unused_declarations\" {\n  enabled = false\n}\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project that pins a TFLint that nothing installed
    ///
    /// The pin holds a version that no machine installs. The run resolves the
    /// TFLint that the action declares, so the pin of the project must not
    /// change what runs. Mise ignores a configuration that nobody trusts, so
    /// the file is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\ntflint = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not
    /// depend on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        let root = ProjectRoot::canonical(self.directory.path())
            .expect("the test gives a directory that exists");

        Context::builder().root(root).build()
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        LintTerraform.run(&self.context(), &()).await
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
    /// The temporary directory of a test never comes back, so the record
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

/// Returns the messages of the findings of an outcome
fn messages(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.message().get())
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

#[test]
fn action_identifies_itself_as_lint_terraform() {
    let name = LintTerraform.name();

    assert_eq!(name.get(), "lint-terraform");
}

#[tokio::test]
async fn run_in_a_clean_project_passes() {
    let project = Project::new();
    project.write().path("main.tf").content(CLEAN).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_hidden_directory_leaves_the_module_alone() {
    let project = Project::new();
    project
        .write()
        .path(".terraform/modules/dns/main.tf")
        .content(UNTYPED)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_missing_directory_stops_without_a_tflint() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();

    let outcome = LintTerraform.run(&context, &()).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(LintTerraformError::UnresolvedTool { .. }))
        ),
        "expected the run to stop without a TFLint, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_without_terraform_passes() {
    let project = Project::new();
    project.write().path("notes.txt").content(UNTYPED).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_leaves_the_project_unchanged() {
    let project = Project::new();
    project.write().path("main.tf").content(UNTYPED).call();

    run_checking_access(&LintTerraform, &project.context(), &()).await;
}

#[tokio::test]
async fn run_whose_configuration_turns_off_the_rules_passes() {
    let project = Project::new();
    project.write().path(".tflint.hcl").content(QUIET).call();
    project.write().path("main.tf").content(UNTYPED).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_an_invalid_module_stops() {
    let project = Project::new();
    project.write().path("main.tf").content(INVALID).call();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(LintTerraformError::IncompleteInspection { .. }))
        ),
        "expected the run to stop, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_an_issue_carries_the_message_of_tflint() {
    let project = Project::new();
    project.write().path("main.tf").content(UNTYPED).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        messages(findings)
            .contains(&"[warning] `name` variable has no type (terraform_typed_variables)"),
        "expected the message of TFLint, got {findings:?}"
    );
}

#[tokio::test]
async fn run_with_an_issue_in_a_module_covers_its_range() {
    let project = Project::new();
    project
        .write()
        .path("modules/dns/main.tf")
        .content(UNTYPED)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    let location = findings.first().map(Finding::location);
    assert_eq!(
        location,
        Some(&Location::Span {
            path: path_text("modules/dns/main.tf")
                .parse()
                .expect("the test gives a path within the project"),
            span: Span::builder()
                .start(Position::builder().line(5).column(1).build())
                .end(Position::builder().line(5).column(16).build())
                .build(),
        })
    );
}
