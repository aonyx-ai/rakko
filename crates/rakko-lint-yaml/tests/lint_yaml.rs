//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately broken YAML file sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the yamllint that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers is
//! the version that the repository installs, and a new pin reaches the tests
//! without a change to them.
//!
//! A project also writes its own configuration. Yamllint looks for one in
//! every directory above the run, and it falls back to the configuration of
//! the user, so a project without one would answer differently on the machine
//! of every contributor. The configuration file is a YAML file that yamllint
//! examines like any other, which is why the counts below are one higher than
//! the number of files that a test wrote itself.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Position};
use rakko_lint_yaml::LintYaml;
use tempfile::TempDir;

/// A YAML document that yamllint accepts
const VALID: &str = "---\nkey: value\n";

/// A YAML document that breaks a rule of the error level
const TRAILING_SPACES: &str = "---\nkey: value   \n";

/// A YAML document that breaks a rule of the warning level
const TRUTHY: &str = "---\nkey: yes\n";

/// The configuration that a project uses when a test states nothing else
const DEFAULT_CONFIGURATION: &str = "---\nextends: default\n";

/// A configuration that turns the rules of the broken documents off
const PERMISSIVE_CONFIGURATION: &str =
    "---\nextends: default\nrules:\n  trailing-spaces: disable\n  truthy: disable\n";

/// A configuration that excludes every file of the project
const IGNORE_EVERYTHING: &str = "---\nextends: default\nignore: |\n  *\n";

/// A configuration that yamllint refuses to read
const BROKEN_CONFIGURATION: &str = "---\nrules:\n  no-such-rule: enable\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a yamllint to resolve
    ///
    /// The project holds no mise configuration and no configuration of
    /// yamllint, so nothing in it reaches a tool. A test uses this shape when
    /// the run must end before the tool runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the yamllint of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the yamllint
    /// that mise resolves for it is the yamllint that the repository pins and
    /// installs. Mise ignores a configuration that nobody trusts, so the copy
    /// is trusted right away. The project also states the rules that yamllint
    /// applies, so that no configuration of the machine reaches the run.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project.write(".yamllint", DEFAULT_CONFIGURATION);

        project
    }

    /// Creates a project that pins a yamllint that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about yamllint.
    fn without_yamllint() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\nyamllint = \"0.0.1\"\n")
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
        LintYaml.run(&self.context(), &()).await
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

// lintyaml[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <<LintYaml as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

// lintyaml[verify name]
#[test]
fn action_identifies_itself_as_lint_yaml() {
    let name = LintYaml.name();

    assert_eq!(name.get(), "lint-yaml");
}

// lintyaml[verify check.passed]
// lintyaml[verify tool.yamllint]
#[tokio::test]
async fn run_in_a_valid_project_passes() {
    let project = Project::new();
    project.write("notes.yaml", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// lintyaml[verify check.passed]
// lintyaml[verify run.listing]
#[tokio::test]
async fn run_in_a_valid_project_says_how_many_files_yamllint_examined() {
    let project = Project::new();
    project.write("notes.yaml", VALID);

    let outcome = project.run().await;

    let Outcome::Passed { summary: Some(it) } = &outcome else {
        panic!("expected a summary, got {outcome:?}");
    };
    assert_eq!(it.get(), "checked 2 files");
}

// lintyaml[verify check.read]
#[tokio::test]
async fn run_leaves_a_file_that_breaks_a_rule_unchanged() {
    let project = Project::new();
    project.write("notes.yaml", TRAILING_SPACES);

    project.run().await;

    assert_eq!(project.read("notes.yaml"), TRAILING_SPACES);
}

// lintyaml[verify run.project]
#[tokio::test]
async fn run_reaches_a_file_below_a_directory_of_the_project() {
    let project = Project::new();
    project.write("notes.yaml", VALID);
    project.write("deep/sub/notes.yaml", TRAILING_SPACES);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["deep/sub/notes.yaml"]);
}

// lintyaml[verify skip.hidden]
#[tokio::test]
async fn run_reaches_a_file_under_a_hidden_directory() {
    let project = Project::bare();
    project.write(".github/workflows/ci.yml", VALID);

    let outcome = project.run().await;

    assert!(
        !matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to examine the project, got {outcome:?}"
    );
}

// lintyaml[verify run.structured]
#[tokio::test]
async fn run_reads_the_configuration_of_the_project() {
    let project = Project::new();
    project.write(".yamllint", PERMISSIVE_CONFIGURATION);
    project.write("notes.yaml", TRAILING_SPACES);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the rules of the project to answer, got {outcome:?}"
    );
}

// lintyaml[verify check.problem]
// lintyaml[verify run.structured]
#[tokio::test]
async fn run_with_a_broken_rule_carries_the_message_of_yamllint() {
    let project = Project::new();
    project.write("notes.yaml", TRAILING_SPACES);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].message().get(),
        "[error] trailing spaces (trailing-spaces)"
    );
}

// lintyaml[verify check.problem]
#[tokio::test]
async fn run_with_a_broken_rule_names_the_file() {
    let project = Project::new();
    project.write("sub/notes.yaml", TRAILING_SPACES);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["sub/notes.yaml"]);
}

// lintyaml[verify check.problem]
#[tokio::test]
async fn run_with_a_broken_rule_reports_the_position_of_yamllint() {
    let project = Project::new();
    project.write("notes.yaml", TRAILING_SPACES);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::Position {
            path: "notes.yaml"
                .parse()
                .expect("the test names a relative path"),
            position: Position::builder().line(2).column(11).build(),
        }
    );
}

// lintyaml[verify check.configuration]
#[tokio::test]
async fn run_with_a_refused_configuration_stops() {
    let project = Project::new();
    project.write(".yamllint", BROKEN_CONFIGURATION);
    project.write("notes.yaml", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// lintyaml[verify check.level]
#[tokio::test]
async fn run_with_a_warning_fails() {
    let project = Project::new();
    project.write("notes.yaml", TRUTHY);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].message().get(),
        "[warning] truthy value should be one of [false, true] (truthy)"
    );
}

// lintyaml[verify check.incomplete]
#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_stops() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write("secret.yaml", VALID);
    let path = project.directory.path().join("secret.yaml");
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

// lintyaml[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_yaml_only_behind_a_symbolic_link_skips() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("linked.yaml"), VALID)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// lintyaml[verify skip.unexamined]
#[tokio::test]
async fn run_whose_yamllint_examined_nothing_skips() {
    let project = Project::new();
    project.write(".yamllint", IGNORE_EVERYTHING);
    project.write("notes.yaml", TRAILING_SPACES);

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("yamllint"),
        "expected the reason to name the tool, got {reason:?}"
    );
}

// lintyaml[verify tool.missing]
#[tokio::test]
async fn run_without_a_yamllint_stops() {
    let project = Project::without_yamllint();
    project.write("notes.yaml", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// lintyaml[verify skip.missing]
#[tokio::test]
async fn run_without_yaml_files_names_what_it_looked_for() {
    let project = Project::bare();
    project.write("notes.txt", "Not YAML.\n");

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains(".yaml"),
        "expected the reason to name the extension, got {reason:?}"
    );
}

// lintyaml[verify skip.missing]
#[tokio::test]
async fn run_without_yaml_files_skips() {
    let project = Project::bare();
    project.write("notes.txt", "Not YAML.\n");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}
