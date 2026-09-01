//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately broken TOML file sits in
//! this repository, where the formatting of the repository itself would
//! fight it.
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

use rakko_action::{
    Action, Args, ArgsValues, ArgumentValue, Context, Finding, Location, Outcome, Position,
    Summary, argument_name,
};
use rakko_format_toml::{FormatToml, FormatTomlArgs};
use tempfile::TempDir;

/// A TOML document that taplo leaves alone
const FORMATTED: &str = "x = 1\n";

/// A TOML document that taplo rewrites
const UNFORMATTED: &str = "x   =    1\n";

/// A TOML document that taplo cannot parse
const INVALID: &str = "broken = [1,\n";

/// A taplo configuration that taplo rejects
///
/// The file is valid TOML, and the value of the option has the wrong type,
/// so taplo warns and falls back to its defaults instead of failing.
const REJECTED_CONFIGURATION: &str = "[formatting]\nalign_entries = \"banana\"\n";

/// A taplo configuration that excludes every file of the project
const EXCLUDE_EVERYTHING: &str = "exclude = [\"**/*\"]\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a taplo to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool
    /// runs.
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

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not
    /// depend on the symbolic links of the temporary directory.
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
    async fn run(&self, fix: bool) -> Outcome {
        let values = if fix {
            ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
        } else {
            ArgsValues::empty()
        };
        let args = FormatTomlArgs::from_values(&values)
            .expect("the test builds the arguments from values that the action reads");

        FormatToml.run(&self.context(), &args).await
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

// formattoml[verify name]
#[test]
fn action_identifies_itself_as_format_toml() {
    let name = FormatToml.name();

    assert_eq!(name.get(), "format-toml");
}

// formattoml[verify check.invalid]
#[tokio::test]
async fn run_with_an_invalid_file_carries_the_message_of_taplo() {
    let project = Project::new();
    project.write("broken.toml", INVALID);

    let outcome = project.run(false).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0].message().get().contains("invalid TOML"),
        "expected the message of taplo, got {:?}",
        findings[0].message()
    );
}

// formattoml[verify check.invalid]
#[tokio::test]
async fn run_with_an_invalid_file_reports_the_position() {
    let project = Project::new();
    project.write("broken.toml", INVALID);

    let outcome = project.run(false).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    let Location::Position { path, position } = findings[0].location() else {
        panic!("expected a finding at a position, got {:?}", findings[0]);
    };
    assert_eq!(
        (path.to_string(), *position),
        (
            "broken.toml".to_owned(),
            Position::builder().line(2).column(1).build()
        )
    );
}

// formattoml[verify check.unformatted]
#[tokio::test]
async fn run_with_an_unformatted_file_names_the_file() {
    let project = Project::new();
    project.write("sub/messy.toml", UNFORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::File {
            path: "sub/messy.toml"
                .parse()
                .expect("the test names a relative path"),
        }
    );
}

// formattoml[verify check.read]
#[tokio::test]
async fn run_without_fix_leaves_the_file_unchanged() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);

    project.run(false).await;

    assert_eq!(project.read("messy.toml"), UNFORMATTED);
}

// formattoml[verify check.configuration]
#[tokio::test]
async fn run_with_a_rejected_configuration_stops() {
    let project = Project::new();
    project.write(".taplo.toml", REJECTED_CONFIGURATION);
    project.write("clean.toml", FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formattoml[verify check.configuration]
#[tokio::test]
async fn run_with_a_rejected_configuration_reports_what_taplo_said() {
    let project = Project::new();
    project.write(".taplo.toml", REJECTED_CONFIGURATION);
    project.write("clean.toml", FORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("TOML parse error"),
        "expected the diagnosis of taplo, got {source}"
    );
}

// formattoml[verify check.unrecognized]
#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_stops() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write("secret.toml", FORMATTED);
    let path = project.directory.path().join("secret.toml");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(false).await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formattoml[verify check.passed]
// formattoml[verify tool.taplo]
#[tokio::test]
async fn run_in_a_formatted_project_passes() {
    let project = Project::new();
    project.write("clean.toml", FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// formattoml[verify check.passed]
#[tokio::test]
async fn run_in_a_formatted_project_counts_the_files() {
    let project = Project::new();
    project.write("clean.toml", FORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("checked 2 files"),
        "the project holds the clean file and its mise.toml"
    );
}

// formattoml[verify check.passed]
#[tokio::test]
async fn run_whose_configuration_excludes_every_file_counts_zero() {
    let project = Project::new();
    project.write(".taplo.toml", EXCLUDE_EVERYTHING);
    project.write("messy.toml", UNFORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(summary.as_ref().map(Summary::get), Some("checked 0 files"));
}

// formattoml[verify fix.changed]
#[tokio::test]
async fn run_with_fix_holds_one_repair_for_each_file() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);
    project.write("sub/other.toml", UNFORMATTED);

    let outcome = project.run(true).await;

    let Outcome::Changed { repairs } = &outcome else {
        panic!("expected the run to change the project, got {outcome:?}");
    };
    let mut repaired = locations(repairs);
    repaired.sort();
    assert_eq!(repaired, ["messy.toml", "sub/other.toml"]);
}

// formattoml[verify fix.changed]
#[tokio::test]
async fn run_with_fix_reports_the_change() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);

    let outcome = project.run(true).await;

    assert!(
        matches!(outcome, Outcome::Changed { .. }),
        "expected the run to change the project, got {outcome:?}"
    );
}

// formattoml[verify fix.write]
#[tokio::test]
async fn run_with_fix_rewrites_the_file() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);

    project.run(true).await;

    assert_eq!(project.read("messy.toml"), FORMATTED);
}

// formattoml[verify fix.partial]
#[tokio::test]
async fn run_with_fix_and_an_invalid_file_fails() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);
    project.write("broken.toml", INVALID);

    let outcome = project.run(true).await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// formattoml[verify fix.partial]
#[tokio::test]
async fn run_with_fix_and_an_invalid_file_holds_the_repairs() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);
    project.write("broken.toml", INVALID);

    let outcome = project.run(true).await;

    let Outcome::Failed { repairs, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(repairs), ["messy.toml"]);
}

// formattoml[verify fix.partial]
#[tokio::test]
async fn run_with_fix_and_an_invalid_file_keeps_the_problem() {
    let project = Project::new();
    project.write("messy.toml", UNFORMATTED);
    project.write("broken.toml", INVALID);

    let outcome = project.run(true).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["broken.toml"]);
}

// formattoml[verify tool.missing]
#[tokio::test]
async fn run_without_a_taplo_stops() {
    let project = Project::without_taplo();
    project.write("clean.toml", FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formattoml[verify skip.git]
#[tokio::test]
async fn run_with_toml_only_under_the_git_directory_skips() {
    let project = Project::bare();
    project.write(".git/config.toml", FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formattoml[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_toml_only_behind_a_symbolic_link_skips() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("linked.toml"), FORMATTED)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formattoml[verify skip.missing]
#[tokio::test]
async fn run_without_toml_files_names_what_it_looked_for() {
    let project = Project::bare();
    project.write("README.md", "# Project\n");

    let outcome = project.run(false).await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains(".toml"),
        "expected the reason to name the extension, got {reason:?}"
    );
}

// formattoml[verify skip.missing]
#[tokio::test]
async fn run_without_toml_files_skips() {
    let project = Project::bare();
    project.write("README.md", "# Project\n");

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}
