//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately faulty crate sits in this
//! repository, where the lints of the repository itself would fight it.
//!
//! The tests run the cargo that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers
//! is the version that the repository installs, and a new pin reaches the
//! tests without a change to them. A project that must reach a toolchain
//! declares the Rust version of this repository, which the copied
//! configuration pins as well.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Summary};
use rakko_check_msrv::CheckMsrv;
use tempfile::TempDir;

/// The manifest of a package that declares no Rust version
const UNDECLARED: &str = "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// A source file that the compiler accepts without a word
const CLEAN: &str = "pub fn clean() {}\n";

/// A source file that the compiler warns about
const WARNED: &str = "pub fn warned() {\n    let unused = 1;\n}\n";

/// A source file that the compiler refuses
const BROKEN: &str = "pub fn broken() -> i32 {\n    \"no\"\n}\n";

/// A source file that only a toolchain newer than the declared one compiles
///
/// `Path::file_prefix` is stable on the default toolchain of this repository
/// and unstable on the oldest one that the repository promises to compile
/// on. A run that reports the unstable feature therefore ran on the declared
/// toolchain and not on the default. The day this repository declares a
/// version that has the method, the test that uses this source fails and
/// asks for another one.
const UNSTABLE: &str =
    "pub fn unstable() {\n    let _ = std::path::Path::new(\"a.tar.gz\").file_prefix();\n}\n";

/// A build script that fails
///
/// Cargo reports the failure on its standard error stream and not as a
/// diagnostic of the compiler, so the report holds nothing that the action
/// recognizes.
const FAILING_SCRIPT: &str = "fn main() {\n    std::process::exit(1);\n}\n";

/// A version that no toolchain of this repository answers to
///
/// Cargo refuses a manifest whose `rust-version` is older than the version
/// that its edition needs, and edition 2024 needs 1.85.0, so the version
/// that stands for an unpinned one is the oldest that cargo accepts here.
const UNPINNED: &str = "1.85.0";

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

    /// Creates a project with the cargo of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the cargo
    /// that mise resolves for it is the cargo that the repository pins and
    /// installs, and the toolchains that it pins are the toolchains that a
    /// declaration of the project can reach. Mise ignores a configuration
    /// that nobody trusts, so the copy is trusted right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project
    }

    /// Creates a project with one package that declares the Rust version of
    /// this repository and holds the given source in its library
    fn with_library(source: &str) -> Self {
        let project = Self::new();
        project.write("Cargo.toml", &package("probe", &declared_rust()));
        project.write("src/lib.rs", source);

        project
    }

    /// Creates a project that pins a Rust toolchain that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for cargo, whatever the global configuration of the machine
    /// says about Rust.
    fn without_cargo() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\nrust = \"0.0.1\"\n")
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
    async fn run(&self) -> Outcome {
        CheckMsrv.run(&self.context(), &()).await
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

/// Returns the Rust version that this repository declares in its manifest
///
/// The repository pins a toolchain for that version, and a project of a test
/// copies those pins, so a project that declares this version reaches a
/// toolchain that the machine has installed. Reading the declaration from
/// the manifest keeps the version out of the tests.
fn declared_rust() -> String {
    let manifest = std::fs::read_to_string(repository().join("Cargo.toml"))
        .expect("the test reads the manifest of the repository");

    manifest
        .lines()
        .find_map(|line| line.strip_prefix("rust-version = "))
        .map(|version| version.trim_matches('"').to_owned())
        .expect("the repository declares a rust-version")
}

/// Returns the paths that the findings of an outcome name
fn locations(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::Span { path, .. } => path.to_string(),
            other => panic!("expected a finding with a range, got {other:?}"),
        })
        .collect()
}

/// Returns the manifest of a package that declares the given Rust version
fn package(name: &str, version: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"{version}\"\n"
    )
}

/// Returns the root of the repository that the tests run in
fn repository() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    manifest
        .parent()
        .and_then(Path::parent)
        .expect("the crate lives two directories below the root of the repository")
}

/// Returns the manifest of a package that belongs to no workspace
///
/// The empty workspace table tells cargo that the package is a root of its
/// own, which cargo demands from a package below a workspace that does not
/// list it.
fn standalone(name: &str, version: &str) -> String {
    format!("[workspace]\n\n{}", package(name, version))
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

// checkmsrv[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <CheckMsrv as Action>::Args::schema();

    assert!(schema.arguments().is_empty());
}

// checkmsrv[verify name]
#[test]
fn action_identifies_itself_as_check_msrv() {
    let name = CheckMsrv.name();

    assert_eq!(name.get(), "check-msrv");
}

// checkmsrv[verify check.operation]
// checkmsrv[verify check.passed]
// checkmsrv[verify tool.cargo]
// checkmsrv[verify tool.toolchain]
#[tokio::test]
async fn run_in_a_clean_project_passes() {
    let project = Project::with_library(CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// checkmsrv[verify check.passed]
#[tokio::test]
async fn run_in_a_clean_project_counts_the_workspaces() {
    let project = Project::with_library(CLEAN);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("checked 1 workspace")
    );
}

// checkmsrv[verify check.operation]
#[tokio::test]
async fn run_examines_the_tests_of_a_package() {
    let project = Project::with_library(CLEAN);
    project.write("tests/probe.rs", WARNED);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// checkmsrv[verify check.read]
#[tokio::test]
async fn run_leaves_the_project_unchanged() {
    let project = Project::with_library(WARNED);

    project.run().await;

    assert_eq!(project.read("src/lib.rs"), WARNED);
}

// checkmsrv[verify tool.toolchain]
#[tokio::test]
async fn run_reports_what_the_declared_toolchain_refuses() {
    let project = Project::with_library(UNSTABLE);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings
            .iter()
            .any(|finding| finding.message().get().contains("unstable")),
        "expected the diagnostic of the declared toolchain, got {findings:?}"
    );
}

// checkmsrv[verify check.diagnostic]
#[tokio::test]
async fn run_with_a_warning_carries_the_code_in_the_message() {
    let project = Project::with_library(WARNED);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings
            .iter()
            .any(|finding| finding.message().get().ends_with("[unused_variables]")),
        "expected the code of the lint, got {findings:?}"
    );
}

// checkmsrv[verify check.diagnostic]
#[tokio::test]
async fn run_with_a_warning_reports_the_range() {
    let project = Project::with_library(WARNED);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    let Location::Span { span, .. } = findings[0].location() else {
        panic!("expected a finding with a range, got {:?}", findings[0]);
    };
    assert_eq!(
        (
            span.start().line().get(),
            span.start().column().map(|column| column.get())
        ),
        (2, Some(9))
    );
}

// checkmsrv[verify check.failed]
#[tokio::test]
async fn run_with_a_warning_fails() {
    let project = Project::with_library(WARNED);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// checkmsrv[verify check.failed]
#[tokio::test]
async fn run_with_a_compiler_error_fails_with_a_finding() {
    let project = Project::with_library(BROKEN);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings
            .iter()
            .any(|finding| finding.message().get().contains("mismatched types")),
        "expected the error of the compiler, got {findings:?}"
    );
}

// checkmsrv[verify roots.declared]
#[tokio::test]
async fn run_with_a_warning_in_a_second_workspace_names_its_path() {
    let project = Project::with_library(CLEAN);
    project.write(
        "tools/harness/Cargo.toml",
        &standalone("harness", &declared_rust()),
    );
    project.write("tools/harness/src/lib.rs", WARNED);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings)[0], "tools/harness/src/lib.rs");
}

// checkmsrv[verify roots.declared]
#[tokio::test]
async fn run_with_two_declaring_workspaces_counts_both() {
    let project = Project::with_library(CLEAN);
    project.write(
        "tools/harness/Cargo.toml",
        &standalone("harness", &declared_rust()),
    );
    project.write("tools/harness/src/lib.rs", CLEAN);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("checked 2 workspaces")
    );
}

// checkmsrv[verify roots.declared]
#[tokio::test]
async fn run_with_a_workspace_that_declares_nothing_passes_over_it() {
    let project = Project::with_library(CLEAN);
    project.write(
        "tools/harness/Cargo.toml",
        &format!("[workspace]\n\n{UNDECLARED}"),
    );
    project.write("tools/harness/src/lib.rs", BROKEN);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("checked 1 workspace")
    );
}

// checkmsrv[verify roots.error]
#[tokio::test]
async fn run_with_a_manifest_that_cargo_cannot_read_stops() {
    let project = Project::new();
    project.write("Cargo.toml", "this is not a manifest\n");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// checkmsrv[verify check.unrecognized]
#[tokio::test]
async fn run_with_a_build_script_that_fails_stops() {
    let project = Project::new();
    project.write(
        "Cargo.toml",
        &format!(
            "{}build = \"build.rs\"\n",
            package("probe", &declared_rust())
        ),
    );
    project.write("build.rs", FAILING_SCRIPT);
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// checkmsrv[verify check.unrecognized]
#[tokio::test]
async fn run_with_a_build_script_that_fails_reports_what_cargo_said() {
    let project = Project::new();
    project.write(
        "Cargo.toml",
        &format!(
            "{}build = \"build.rs\"\n",
            package("probe", &declared_rust())
        ),
    );
    project.write("build.rs", FAILING_SCRIPT);
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("custom build command"),
        "expected the diagnosis of cargo, got {source}"
    );
}

// checkmsrv[verify tool.missing]
#[tokio::test]
async fn run_without_a_cargo_stops() {
    let project = Project::without_cargo();
    project.write("Cargo.toml", &package("probe", &declared_rust()));
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// checkmsrv[verify tool.unpinned]
#[tokio::test]
async fn run_without_a_pinned_toolchain_stops() {
    let project = Project::new();
    project.write("Cargo.toml", &package("probe", UNPINNED));
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// checkmsrv[verify tool.unpinned]
#[tokio::test]
async fn run_without_a_pinned_toolchain_names_the_version() {
    let project = Project::new();
    project.write("Cargo.toml", &package("probe", UNPINNED));
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains(UNPINNED),
        "expected the error to name the version, got {source}"
    );
}

// checkmsrv[verify skip.undeclared]
#[tokio::test]
async fn run_without_a_declaration_skips() {
    let project = Project::new();
    project.write("Cargo.toml", UNDECLARED);
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkmsrv[verify skip.undeclared]
#[tokio::test]
async fn run_without_a_declaration_names_what_it_looked_for() {
    let project = Project::new();
    project.write("Cargo.toml", UNDECLARED);
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("rust-version"),
        "expected the reason to name the declaration, got {reason:?}"
    );
}

// checkmsrv[verify skip.git]
#[tokio::test]
async fn run_with_a_manifest_only_under_the_git_directory_skips() {
    let project = Project::bare();
    project.write(".git/Cargo.toml", UNDECLARED);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkmsrv[verify skip.target]
#[tokio::test]
async fn run_with_a_manifest_only_under_the_target_directory_skips() {
    let project = Project::bare();
    project.write("target/debug/build/dep/Cargo.toml", UNDECLARED);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkmsrv[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_a_manifest_only_behind_a_symbolic_link_skips() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("Cargo.toml"), UNDECLARED)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkmsrv[verify skip.missing]
#[tokio::test]
async fn run_without_a_manifest_skips() {
    let project = Project::bare();
    project.write("README.md", "# Project\n");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkmsrv[verify skip.missing]
#[tokio::test]
async fn run_without_a_manifest_names_what_it_looked_for() {
    let project = Project::bare();
    project.write("README.md", "# Project\n");

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("Cargo.toml"),
        "expected the reason to name the manifest, got {reason:?}"
    );
}
