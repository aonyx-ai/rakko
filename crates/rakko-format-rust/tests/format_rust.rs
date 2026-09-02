//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately broken Rust file sits in
//! this repository, where the formatting of the repository itself would
//! fight it.
//!
//! The tests run the cargo and the nightly toolchain that this repository
//! pins. A project copies the `mise.toml` and the `mise.lock` of the
//! repository and trusts them, so the versions that answer are the versions
//! that the repository installs, and a new pin reaches the tests without a
//! change to them.

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
use rakko_format_rust::{FormatRust, FormatRustArgs};
use tempfile::TempDir;

/// The manifest of a package that rustfmt formats
const PACKAGE: &str = "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// The manifest of a package that belongs to no workspace
///
/// The empty workspace table tells cargo that the package is a root of its
/// own, which cargo demands from a package below a workspace that does not
/// list it.
const STANDALONE: &str =
    "[workspace]\n\n[package]\nname = \"harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// A source file that rustfmt leaves alone
const FORMATTED: &str = "pub fn clean() {}\n";

/// A source file that rustfmt rewrites
const UNFORMATTED: &str = "pub fn messy( x:i32 )->i32{ x }\n";

/// The source file that rustfmt rewrites, after the rewrite
const REWRITTEN: &str = "pub fn messy(x: i32) -> i32 {\n    x\n}\n";

/// A binary that rustfmt rewrites
const UNFORMATTED_MAIN: &str = "fn main( ){ }\n";

/// A source file that rustfmt cannot parse
const INVALID: &str = "pub fn broken( {\n";

/// A configuration with an option that rustfmt does not know
///
/// Rustfmt warns and formats without the option instead of failing.
const UNKNOWN_OPTION: &str = "no_such_option = true\n";

/// A configuration that rustfmt cannot parse
const UNPARSABLE_CONFIGURATION: &str = "this = = 1\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a cargo to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool
    /// runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the toolchains of this repository
    ///
    /// The project copies the `mise.toml` and the `mise.lock` of this
    /// repository, so the cargo and the nightly that mise resolves for it
    /// are the ones that the repository pins and installs. Mise ignores a
    /// configuration that nobody trusts, so the copy is trusted right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        std::fs::copy(
            repository().join("mise.lock"),
            project.directory.path().join("mise.lock"),
        )
        .expect("the test copies the mise.lock of the repository");
        trust(&copy);

        project
    }

    /// Creates a project with one package whose library holds the given
    /// source
    fn with_library(source: &str) -> Self {
        let project = Self::new();
        project.write("Cargo.toml", PACKAGE);
        project.write("src/lib.rs", source);

        project
    }

    /// Creates a project that pins the given Rust toolchains and holds a
    /// package
    fn with_rust(pins: &str) -> Self {
        let project = Self::bare();

        let configuration = project.directory.path().join("mise.toml");
        std::fs::write(&configuration, format!("[tools]\nrust = {pins}\n"))
            .expect("the test writes the mise.toml of the project");
        trust(&configuration);
        project.write("Cargo.toml", PACKAGE);
        project.write("src/lib.rs", FORMATTED);

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
        let args = FormatRustArgs::from_values(&values)
            .expect("the test builds the arguments from values that the action reads");

        FormatRust.run(&self.context(), &args).await
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

/// Returns the default Rust toolchain that this repository pins, as mise
/// reports it
///
/// A test that pins no nightly still needs a cargo, and the default pin of
/// the repository is the one toolchain that every machine which runs the
/// tests has installed. Reading the pin from mise keeps the version out of
/// the tests.
fn pinned_rust() -> String {
    let output = Command::new("mise")
        .args(["ls", "--current", "--json", "rust"])
        .current_dir(repository())
        .output()
        .expect("the test starts mise to list the toolchains");
    let pins: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the test reads the report of mise");

    pins.as_array()
        .and_then(|pins| {
            pins.iter().find_map(|pin| {
                let requested = pin["requested_version"].as_str()?;

                (!requested.starts_with("nightly")).then(|| requested.to_owned())
            })
        })
        .expect("the repository pins a default Rust toolchain")
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

// formatrust[verify name]
#[test]
fn action_identifies_itself_as_format_rust() {
    let name = FormatRust.name();

    assert_eq!(name.get(), "format-rust");
}

// formatrust[verify check.invalid]
#[tokio::test]
async fn run_with_an_invalid_file_carries_the_message_of_rustfmt() {
    let project = Project::with_library(INVALID);

    let outcome = project.run(false).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0].message().get().contains("unclosed delimiter"),
        "expected the message of rustfmt, got {:?}",
        findings[0].message()
    );
}

// formatrust[verify check.invalid]
#[tokio::test]
async fn run_with_an_invalid_file_reports_the_position() {
    let project = Project::with_library(INVALID);

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
            "src/lib.rs".to_owned(),
            Position::builder().line(1).column(18).build()
        )
    );
}

// formatrust[verify check.unformatted]
#[tokio::test]
async fn run_with_an_unformatted_file_names_the_file() {
    let project = Project::with_library(UNFORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::File {
            path: "src/lib.rs"
                .parse()
                .expect("the test names a relative path"),
        }
    );
}

// formatrust[verify check.read]
#[tokio::test]
async fn run_without_fix_leaves_the_file_unchanged() {
    let project = Project::with_library(UNFORMATTED);

    project.run(false).await;

    assert_eq!(project.read("src/lib.rs"), UNFORMATTED);
}

// formatrust[verify check.configuration]
#[tokio::test]
async fn run_with_an_unknown_option_stops() {
    let project = Project::with_library(FORMATTED);
    project.write(".rustfmt.toml", UNKNOWN_OPTION);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formatrust[verify check.configuration]
#[tokio::test]
async fn run_with_an_unknown_option_reports_what_rustfmt_said() {
    let project = Project::with_library(FORMATTED);
    project.write(".rustfmt.toml", UNKNOWN_OPTION);

    let outcome = project.run(false).await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("Unknown configuration option"),
        "expected the warning of rustfmt, got {source}"
    );
}

// formatrust[verify check.unrecognized]
#[tokio::test]
async fn run_with_a_configuration_that_rustfmt_cannot_parse_stops() {
    let project = Project::with_library(FORMATTED);
    project.write(".rustfmt.toml", UNPARSABLE_CONFIGURATION);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formatrust[verify check.unrecognized]
#[tokio::test]
async fn run_with_a_configuration_that_rustfmt_cannot_parse_reports_what_it_wrote() {
    let project = Project::with_library(FORMATTED);
    project.write(".rustfmt.toml", UNPARSABLE_CONFIGURATION);

    let outcome = project.run(false).await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("Could not parse TOML"),
        "expected the diagnosis of rustfmt, got {source}"
    );
}

// formatrust[verify check.operation]
// formatrust[verify check.passed]
// formatrust[verify tool.cargo]
// formatrust[verify tool.toolchain]
#[tokio::test]
async fn run_in_a_formatted_project_passes() {
    let project = Project::with_library(FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// formatrust[verify check.passed]
#[tokio::test]
async fn run_in_a_formatted_project_counts_the_workspaces() {
    let project = Project::with_library(FORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("checked 1 workspace")
    );
}

// formatrust[verify roots.all]
#[tokio::test]
async fn run_with_an_unformatted_file_in_a_second_workspace_names_its_path() {
    let project = Project::with_library(FORMATTED);
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/lib.rs", UNFORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["tools/harness/src/lib.rs"]);
}

// formatrust[verify roots.all]
#[tokio::test]
async fn run_with_two_workspaces_counts_both() {
    let project = Project::with_library(FORMATTED);
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/lib.rs", FORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("checked 2 workspaces")
    );
}

// formatrust[verify roots.error]
#[tokio::test]
async fn run_with_a_manifest_that_cargo_cannot_read_stops() {
    let project = Project::new();
    project.write("Cargo.toml", "this is not a manifest\n");

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formatrust[verify fix.changed]
#[tokio::test]
async fn run_with_fix_holds_one_repair_for_each_file() {
    let project = Project::with_library(UNFORMATTED);
    project.write("src/main.rs", UNFORMATTED_MAIN);

    let outcome = project.run(true).await;

    let Outcome::Changed { repairs } = &outcome else {
        panic!("expected the run to change the project, got {outcome:?}");
    };
    let mut repaired = locations(repairs);
    repaired.sort();
    assert_eq!(repaired, ["src/lib.rs", "src/main.rs"]);
}

// formatrust[verify fix.changed]
#[tokio::test]
async fn run_with_fix_reports_the_change() {
    let project = Project::with_library(UNFORMATTED);

    let outcome = project.run(true).await;

    assert!(
        matches!(outcome, Outcome::Changed { .. }),
        "expected the run to change the project, got {outcome:?}"
    );
}

// formatrust[verify fix.write]
#[tokio::test]
async fn run_with_fix_rewrites_the_file() {
    let project = Project::with_library(UNFORMATTED);

    project.run(true).await;

    assert_eq!(project.read("src/lib.rs"), REWRITTEN);
}

// formatrust[verify fix.partial]
#[tokio::test]
async fn run_with_fix_and_an_invalid_file_elsewhere_fails() {
    let project = Project::with_library(UNFORMATTED);
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/lib.rs", INVALID);

    let outcome = project.run(true).await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// formatrust[verify fix.partial]
#[tokio::test]
async fn run_with_fix_and_an_invalid_file_elsewhere_holds_the_repairs() {
    let project = Project::with_library(UNFORMATTED);
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/lib.rs", INVALID);

    let outcome = project.run(true).await;

    let Outcome::Failed { repairs, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(repairs), ["src/lib.rs"]);
}

// formatrust[verify fix.partial]
#[tokio::test]
async fn run_with_fix_and_an_invalid_file_elsewhere_keeps_the_problem() {
    let project = Project::with_library(UNFORMATTED);
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/lib.rs", INVALID);

    let outcome = project.run(true).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["tools/harness/src/lib.rs"]);
}

// formatrust[verify fix.partial]
#[tokio::test]
async fn run_with_fix_and_an_invalid_module_keeps_the_package_as_it_is() {
    let project = Project::with_library("pub mod broken;\npub fn messy( ) {}\n");
    project.write("src/broken.rs", INVALID);

    let outcome = project.run(true).await;

    let Outcome::Failed { repairs, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        repairs.is_empty(),
        "expected no repair in a package that rustfmt cannot read, got {repairs:?}"
    );
}

// formatrust[verify tool.missing]
#[tokio::test]
async fn run_without_a_cargo_stops() {
    let project = Project::with_rust("\"0.0.1\"");

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formatrust[verify tool.unpinned]
#[tokio::test]
async fn run_without_a_nightly_toolchain_names_the_channel() {
    let project = Project::with_rust(&format!("\"{}\"", pinned_rust()));

    let outcome = project.run(false).await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    let chain = std::iter::successors(Some(source.as_ref() as &dyn std::error::Error), |error| {
        error.source()
    })
    .map(ToString::to_string)
    .collect::<Vec<String>>()
    .join(": ");
    assert!(
        chain.contains("nightly"),
        "expected the error to name the channel, got {chain}"
    );
}

// formatrust[verify tool.unpinned]
#[tokio::test]
async fn run_without_a_nightly_toolchain_stops() {
    let project = Project::with_rust(&format!("\"{}\"", pinned_rust()));

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formatrust[verify skip.git]
#[tokio::test]
async fn run_with_a_manifest_only_under_the_git_directory_skips() {
    let project = Project::bare();
    project.write(".git/Cargo.toml", PACKAGE);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formatrust[verify skip.target]
#[tokio::test]
async fn run_with_a_manifest_only_under_the_target_directory_skips() {
    let project = Project::bare();
    project.write("target/debug/build/dep/Cargo.toml", PACKAGE);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formatrust[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_a_manifest_only_behind_a_symbolic_link_skips() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("Cargo.toml"), PACKAGE)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formatrust[verify skip.missing]
#[tokio::test]
async fn run_without_a_manifest_names_what_it_looked_for() {
    let project = Project::bare();
    project.write("README.md", "# Project\n");

    let outcome = project.run(false).await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("Cargo.toml"),
        "expected the reason to name the manifest, got {reason:?}"
    );
}

// formatrust[verify skip.missing]
#[tokio::test]
async fn run_without_a_manifest_skips() {
    let project = Project::bare();
    project.write("README.md", "# Project\n");

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}
