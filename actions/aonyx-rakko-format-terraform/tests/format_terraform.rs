//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. So no fixture with a deliberately broken file sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the Terraform that this repository pins. Terraform is a
//! toolchain, so the project pins it. A project copies the `mise.toml` of the
//! repository and trusts it. So the version that answers is the version that
//! the repository installs, and a new pin reaches the tests without a change
//! to them.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::process::Command;

use aonyx_rakko_format_terraform::{
    FormatMode, FormatTerraform, FormatTerraformArgs, FormatTerraformError,
};
use rakko::{
    Action, Args, ArgsValues, ArgumentValue, Context, Finding, Location, Outcome, ProjectRoot,
    argument_name,
};
use rakko_test_utils::{path_text, run_checking_access};
use tempfile::TempDir;

/// A file that Terraform leaves alone
const FORMATTED: &str = "variable \"name\" {}\n";

/// A file that Terraform rewrites, because its equal signs do not line up
const UNFORMATTED: &str = "locals {\n  a = 1\n  bb = 2\n}\n";

/// The file above after Terraform formatted it
const REFORMATTED: &str = "locals {\n  a  = 1\n  bb = 2\n}\n";

/// A file that Terraform cannot parse
const INVALID: &str = "resource \"x\" {\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project with the Terraform of this repository
    ///
    /// The project copies the `mise.toml` of this repository. So the Terraform
    /// that mise resolves for it is the Terraform that the repository pins and
    /// installs. Mise ignores a configuration that nobody trusts, so the copy
    /// is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let copy = directory.path().join("mise.toml");
        std::fs::copy(repository().join("mise.toml"), &copy)
            .expect("the test copies the mise.toml of the repository");
        trust(&copy);

        Self { directory }
    }

    /// Creates a project that pins the given Terraform
    ///
    /// Mise ignores a configuration that nobody trusts, so the file is trusted
    /// right away.
    fn pinning(version: &str) -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        let pins = directory.path().join("mise.toml");
        std::fs::write(&pins, format!("[tools]\nterraform = \"{version}\"\n"))
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        Self { directory }
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

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self, format_mode: FormatMode) -> Outcome {
        let values = match format_mode {
            FormatMode::Rewrite => {
                ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
            }
            FormatMode::Report => ArgsValues::empty(),
        };
        let args = FormatTerraformArgs::from_values(&values)
            .expect("the test builds the arguments from values that the action reads");

        FormatTerraform.run(&self.context(), &args).await
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

/// Returns the paths of the findings of an outcome
fn locations(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::File { path } => path.to_string(),
            other => panic!("expected a finding with a file, got {other:?}"),
        })
        .collect()
}

/// Returns the root of this repository, two directories above the crate
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate lives two directories below the root of the repository")
        .to_path_buf()
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
fn action_identifies_itself_as_format_terraform() {
    let name = FormatTerraform.name();

    assert_eq!(name.get(), "format-terraform");
}

#[tokio::test]
async fn run_in_a_formatted_project_passes() {
    let project = Project::new();
    project.write().path("main.tf").content(FORMATTED).call();

    let outcome = project.run(FormatMode::Report).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_hidden_directory_leaves_the_file_alone() {
    let project = Project::new();
    project
        .write()
        .path(".terraform/modules/dns/main.tf")
        .content(UNFORMATTED)
        .call();

    let outcome = project.run(FormatMode::Report).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_without_terraform_passes() {
    let project = Project::new();
    project
        .write()
        .path("notes.txt")
        .content(UNFORMATTED)
        .call();

    let outcome = project.run(FormatMode::Report).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_terraform_that_nothing_installed_stops() {
    let project = Project::pinning("0.0.1");
    project.write().path("main.tf").content(FORMATTED).call();

    let outcome = project.run(FormatMode::Report).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(FormatTerraformError::UnresolvedTool { .. }))
        ),
        "expected the run to stop without a Terraform, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_an_invalid_file_stops() {
    let project = Project::new();
    project.write().path("main.tf").content(INVALID).call();

    let outcome = project.run(FormatMode::Report).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(FormatTerraformError::UnreadableConfiguration { .. }))
        ),
        "expected the run to stop, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_an_unformatted_file_gives_the_file() {
    let project = Project::new();
    project.write().path("main.tf").content(FORMATTED).call();
    project
        .write()
        .path("modules/dns/main.tf")
        .content(UNFORMATTED)
        .call();

    let outcome = project.run(FormatMode::Report).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("modules/dns/main.tf")]);
}

#[tokio::test]
async fn run_with_fix_holds_one_repair_for_each_file() {
    let project = Project::new();
    project.write().path("main.tf").content(UNFORMATTED).call();
    project
        .write()
        .path("prod.tfvars")
        .content("a = 1\nbb = 2\n")
        .call();

    let outcome = project.run(FormatMode::Rewrite).await;

    let Outcome::Changed { repairs } = &outcome else {
        panic!("expected the run to change the project, got {outcome:?}");
    };
    let mut repaired = locations(repairs);
    repaired.sort();
    assert_eq!(repaired, ["main.tf", "prod.tfvars"]);
}

#[tokio::test]
async fn run_with_fix_reports_the_change() {
    let project = Project::new();
    project.write().path("main.tf").content(UNFORMATTED).call();

    let outcome = project.run(FormatMode::Rewrite).await;

    assert!(
        matches!(outcome, Outcome::Changed { .. }),
        "expected the run to change the project, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_fix_rewrites_the_file() {
    let project = Project::new();
    project.write().path("main.tf").content(UNFORMATTED).call();

    project.run(FormatMode::Rewrite).await;

    assert_eq!(project.read("main.tf"), REFORMATTED);
}

#[tokio::test]
async fn run_without_fix_leaves_the_file_unchanged() {
    let project = Project::new();
    project.write().path("main.tf").content(UNFORMATTED).call();

    project.run(FormatMode::Report).await;

    assert_eq!(project.read("main.tf"), UNFORMATTED);
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_without_fix_leaves_the_project_unchanged() {
    let project = Project::new();
    project.write().path("main.tf").content(UNFORMATTED).call();

    run_checking_access(
        &FormatTerraform,
        &project.context(),
        &FormatTerraformArgs::default(),
    )
    .await;
}
