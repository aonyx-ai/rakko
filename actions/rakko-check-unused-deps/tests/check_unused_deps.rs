//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately unused dependency sits in
//! this repository, where the checks of the repository itself would fight
//! it.
//!
//! A project depends on a package that it carries, and never on a package of
//! a registry. The dependency that a test declares is therefore the one that
//! the action reports, the build reaches no network, and it compiles two
//! small libraries and nothing else.
//!
//! The tests run the cargo that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers
//! is the version that the repository installs, and a new pin reaches the
//! tests without a change to them. The nightly channel that cargo-udeps
//! needs is one of those pins.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Summary};
use rakko_check_unused_deps::CheckUnusedDeps;
use tempfile::TempDir;

/// A source file that the compiler refuses
const BROKEN: &str = "pub fn broken() -> i32 {\n    \"no\"\n}\n";

/// A source file that the compiler accepts without a word
const CLEAN: &str = "pub fn clean() {}\n";

/// The table that declares the helper as a dependency of the tests and the
/// examples
const DEVELOPMENT: &str = "\n[dev-dependencies]\nhelper = { path = \"helper\" }\n";

/// A build script that fails
///
/// Cargo reports the failure on its standard error stream and not as a
/// diagnostic of the compiler, so the run holds nothing that the action
/// recognizes.
const FAILING_SCRIPT: &str = "fn main() {\n    std::process::exit(1);\n}\n";

/// The manifest of the package that a project carries to depend on
const HELPER: &str = "[package]\nname = \"helper\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// The library of that package
const HELPER_LIBRARY: &str = "pub fn help() {}\n";

/// The table that declares the helper as a dependency of the library
const NORMAL: &str = "\n[dependencies]\nhelper = { path = \"helper\" }\n";

/// The tables that declare the helper as a dependency behind a feature that
/// the project does not turn on
///
/// Only a run that turns every feature on compiles the dependency, and only
/// such a run can say whether anything uses it.
const OPTIONAL: &str = concat!(
    "\n[features]\nextra = [\"dep:helper\"]\n",
    "\n[dependencies]\nhelper = { path = \"helper\", optional = true }\n"
);

/// A source file that reaches the helper
const USES_HELPER: &str = "pub fn uses() {\n    helper::help();\n}\n";

/// A source file that the compiler warns about
const WARNED: &str = "pub fn warned() {\n    let unused = 1;\n}\n";

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

    /// Creates a project with the cargo and the toolchains of this
    /// repository
    ///
    /// The project copies the `mise.toml` of this repository, so the cargo
    /// that mise resolves for it is the cargo that the repository pins and
    /// installs, and the nightly channel that cargo-udeps needs is the one
    /// that the repository pins. Mise ignores a configuration that nobody
    /// trusts, so the copy is trusted right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project
    }

    /// Creates a project that declares the helper in the given tables and
    /// holds the given source in its library
    fn with_dependency(tables: &str, source: &str) -> Self {
        let project = Self::new();
        project.write("Cargo.toml", &workspace(tables));
        project.write("src/lib.rs", source);
        project.write("helper/Cargo.toml", HELPER);
        project.write("helper/src/lib.rs", HELPER_LIBRARY);

        project
    }

    /// Creates a project that pins no nightly toolchain
    ///
    /// The pin names the toolchain that this repository builds with, so mise
    /// reports a cargo for the project and no toolchain of the nightly
    /// channel.
    fn without_nightly() -> Self {
        let project = Self::bare();
        project.pin(&format!("[tools]\nrust = \"{}\"\n", default_rust()));

        project
    }

    /// Creates a project that pins a Rust toolchain that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for cargo, whatever the global configuration of the machine
    /// says about Rust.
    fn without_cargo() -> Self {
        let project = Self::bare();
        project.pin("[tools]\nrust = \"0.0.1\"\n");

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

    /// Writes the mise configuration of the project and trusts it
    fn pin(&self, pins: &str) {
        let path = self.directory.path().join("mise.toml");
        std::fs::write(&path, pins).expect("the test writes the mise.toml of the project");
        trust(&path);
    }

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        CheckUnusedDeps.run(&self.context(), &()).await
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

/// Returns the Rust toolchain that this repository builds with
///
/// A project that pins this version reaches a cargo without reaching a
/// nightly, which is the shape that one test needs. Reading the version from
/// the manifest keeps it out of the tests.
fn default_rust() -> String {
    let manifest = std::fs::read_to_string(repository().join("Cargo.toml"))
        .expect("the test reads the manifest of the repository");

    manifest
        .lines()
        .find_map(|line| line.strip_prefix("rust-version = "))
        .map(|version| version.trim_matches('"').to_owned())
        .expect("the repository declares a rust-version")
}

/// Returns the messages of the findings of an outcome that failed
fn messages(outcome: &Outcome) -> Vec<String> {
    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };

    findings
        .iter()
        .map(|finding| finding.message().get().to_owned())
        .collect()
}

/// Returns the path that a finding names, at whichever level it speaks
fn path(finding: &Finding) -> String {
    match finding.location() {
        Location::File { path } => path.to_string(),
        Location::Span { path, .. } => path.to_string(),
        other => panic!("expected a finding with a path, got {other:?}"),
    }
}

/// Returns the paths that the findings of an outcome that failed name
fn paths(outcome: &Outcome) -> Vec<String> {
    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };

    findings.iter().map(path).collect()
}

/// Returns the root of the repository that the tests run in
fn repository() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    manifest
        .parent()
        .and_then(Path::parent)
        .expect("the crate lives two directories below the root of the repository")
}

/// Returns the summary of an outcome that passed
fn summary(outcome: &Outcome) -> Option<&str> {
    let Outcome::Passed { summary } = outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };

    summary.as_ref().map(Summary::get)
}

/// Returns the manifest of a workspace whose package declares the helper in
/// the given tables
///
/// The workspace lists the helper as a member, so that cargo describes both
/// packages as one workspace and the discovery of the roots finds one root.
fn workspace(tables: &str) -> String {
    format!(
        "[workspace]\nmembers = [\"helper\"]\n\n[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{tables}"
    )
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

// checkunuseddeps[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <CheckUnusedDeps as Action>::Args::schema();

    assert!(schema.arguments().is_empty());
}

// checkunuseddeps[verify name]
#[test]
fn action_identifies_itself_as_check_unused_deps() {
    let name = CheckUnusedDeps.name();

    assert_eq!(name.get(), "check-unused-deps");
}

// checkunuseddeps[verify check.passed]
#[tokio::test]
async fn run_in_a_project_that_uses_every_dependency_counts_the_workspaces() {
    let project = Project::with_dependency(NORMAL, USES_HELPER);

    let outcome = project.run().await;

    assert_eq!(summary(&outcome), Some("examined 1 workspace"));
}

// A run of cargo-udeps needs an unstable option of the compiler, and a
// stable toolchain refuses it and reports nothing, so a run that answers at
// all ran on the nightly toolchain that mise pins for the project.
// checkunuseddeps[verify check.passed]
// checkunuseddeps[verify tool.cargo]
// checkunuseddeps[verify tool.toolchain]
#[tokio::test]
async fn run_in_a_project_that_uses_every_dependency_passes() {
    let project = Project::with_dependency(NORMAL, USES_HELPER);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// checkunuseddeps[verify check.read]
#[tokio::test]
async fn run_leaves_the_project_unchanged() {
    let project = Project::with_dependency(NORMAL, CLEAN);

    project.run().await;

    assert_eq!(project.read("Cargo.toml"), workspace(NORMAL));
}

// checkunuseddeps[verify check.unrecognized]
#[tokio::test]
async fn run_with_a_build_script_that_fails_stops() {
    let project = Project::with_dependency(NORMAL, USES_HELPER);
    project.write(
        "Cargo.toml",
        &workspace(&format!("build = \"build.rs\"\n{NORMAL}")),
    );
    project.write("build.rs", FAILING_SCRIPT);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// checkunuseddeps[verify check.diagnostic]
#[tokio::test]
async fn run_with_a_compiler_error_names_the_file() {
    let project = Project::with_dependency(NORMAL, BROKEN);

    let outcome = project.run().await;

    assert_eq!(paths(&outcome), vec!["src/lib.rs".to_owned()]);
}

// checkunuseddeps[verify check.diagnostic]
#[tokio::test]
async fn run_with_a_compiler_error_reports_the_diagnostic() {
    let project = Project::with_dependency(NORMAL, BROKEN);

    let outcome = project.run().await;

    assert!(
        messages(&outcome)
            .iter()
            .any(|message| message.contains("mismatched types")),
        "expected the error of the compiler, got {outcome:?}"
    );
}

// checkunuseddeps[verify check.operation]
#[tokio::test]
async fn run_with_a_development_dependency_that_a_test_uses_passes() {
    let project = Project::with_dependency(DEVELOPMENT, CLEAN);
    project.write(
        "tests/probe.rs",
        "#[test]\nfn uses() {\n    helper::help();\n}\n",
    );

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// checkunuseddeps[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_a_manifest_only_behind_a_symbolic_link_skips() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("Cargo.toml"), HELPER)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkunuseddeps[verify skip.git]
#[tokio::test]
async fn run_with_a_manifest_only_under_the_git_directory_skips() {
    let project = Project::bare();
    project.write(".git/Cargo.toml", HELPER);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkunuseddeps[verify skip.target]
#[tokio::test]
async fn run_with_a_manifest_only_under_the_target_directory_skips() {
    let project = Project::bare();
    project.write("target/debug/build/dep/Cargo.toml", HELPER);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkunuseddeps[verify roots.error]
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

// A warning of the compiler belongs to the action that lints the code. The
// build finished, so the answer of this run is the report of cargo-udeps.
// checkunuseddeps[verify check.diagnostic]
#[tokio::test]
async fn run_with_a_warning_reports_no_diagnostic() {
    let project = Project::with_dependency(NORMAL, &format!("{USES_HELPER}{WARNED}"));

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// checkunuseddeps[verify check.failed]
#[tokio::test]
async fn run_with_an_unused_dependency_fails() {
    let project = Project::with_dependency(NORMAL, CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// checkunuseddeps[verify roots.every]
#[tokio::test]
async fn run_with_an_unused_dependency_in_a_second_workspace_names_its_manifest() {
    let project = Project::with_dependency(NORMAL, USES_HELPER);
    project.write(
        "tools/harness/Cargo.toml",
        "[workspace]\n\n[package]\nname = \"harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nhelper = { path = \"../../helper\" }\n",
    );
    project.write("tools/harness/src/lib.rs", CLEAN);

    let outcome = project.run().await;

    assert_eq!(paths(&outcome), vec!["tools/harness/Cargo.toml".to_owned()]);
}

// checkunuseddeps[verify check.finding]
#[tokio::test]
async fn run_with_an_unused_dependency_names_the_dependency() {
    let project = Project::with_dependency(NORMAL, CLEAN);

    let outcome = project.run().await;

    assert_eq!(
        messages(&outcome),
        vec!["unused dependency: helper".to_owned()]
    );
}

// checkunuseddeps[verify check.finding]
#[tokio::test]
async fn run_with_an_unused_dependency_names_the_manifest() {
    let project = Project::with_dependency(NORMAL, CLEAN);

    let outcome = project.run().await;

    assert_eq!(paths(&outcome), vec!["Cargo.toml".to_owned()]);
}

// The bare cargo-udeps that this action replaces examines the default
// targets, which leaves a dependency of the tests uncompiled and therefore
// unexamined.
// checkunuseddeps[verify check.operation]
#[tokio::test]
async fn run_with_an_unused_development_dependency_names_its_kind() {
    let project = Project::with_dependency(DEVELOPMENT, CLEAN);

    let outcome = project.run().await;

    assert_eq!(
        messages(&outcome),
        vec!["unused dev-dependency: helper".to_owned()]
    );
}

// The bare cargo-udeps that this action replaces examines the default
// features, which leaves a dependency behind a feature uncompiled and
// therefore unexamined.
// checkunuseddeps[verify check.operation]
#[tokio::test]
async fn run_with_an_unused_optional_dependency_names_it() {
    let project = Project::with_dependency(OPTIONAL, CLEAN);

    let outcome = project.run().await;

    assert_eq!(
        messages(&outcome),
        vec!["unused dependency: helper".to_owned()]
    );
}

// checkunuseddeps[verify roots.every]
#[tokio::test]
async fn run_with_two_workspaces_counts_both() {
    let project = Project::with_dependency(NORMAL, USES_HELPER);
    project.write(
        "tools/harness/Cargo.toml",
        "[workspace]\n\n[package]\nname = \"harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    project.write("tools/harness/src/lib.rs", CLEAN);

    let outcome = project.run().await;

    assert_eq!(summary(&outcome), Some("examined 2 workspaces"));
}

// checkunuseddeps[verify tool.missing]
#[tokio::test]
async fn run_without_a_cargo_stops() {
    let project = Project::without_cargo();
    project.write("Cargo.toml", &workspace(NORMAL));
    project.write("src/lib.rs", CLEAN);
    project.write("helper/Cargo.toml", HELPER);
    project.write("helper/src/lib.rs", HELPER_LIBRARY);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// checkunuseddeps[verify skip.missing]
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

// checkunuseddeps[verify skip.missing]
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

// checkunuseddeps[verify tool.unpinned]
#[tokio::test]
async fn run_without_a_pinned_nightly_stops() {
    let project = Project::without_nightly();
    project.write("Cargo.toml", &workspace(NORMAL));
    project.write("src/lib.rs", CLEAN);
    project.write("helper/Cargo.toml", HELPER);
    project.write("helper/src/lib.rs", HELPER_LIBRARY);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}
