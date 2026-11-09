//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it.
//! So no fixture with a deliberately failing test sits in this repository,
//! where the tests of the repository itself would fight it.
//!
//! A project of a test is a git repository with one empty commit, and the
//! files of the project are untracked on top of it. That is the tree of a
//! contributor who has not committed yet, which is the tree that the copy has
//! to carry over.
//!
//! No project of a test declares a dependency that a registry holds. The
//! resolution then needs no network, and what a test asserts does not depend
//! on what crates.io published this morning.
//!
//! The tests run the nextest that the action declares, with the cargo that
//! this repository pins. A project copies the `mise.toml` of the repository
//! and trusts it, so the Rust toolchains that answer are ones that the
//! repository installs. The copy pins another nextest that nothing installs,
//! so a test passes only when the run ignores the pin of the project.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko::{Action, Args, Context, Finding, Location, Outcome, ProjectRoot, Summary};
use rakko_check_latest_deps::{CheckLatestDeps, CheckLatestDepsError};
use rakko_test_utils::{path_text, run_checking_access};
use tempfile::TempDir;

/// The manifest of a package that nextest tests
const PACKAGE: &str = "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// The manifest of a package that belongs to no workspace
///
/// The empty workspace table tells cargo that the package is a root of its own.
/// Cargo demands this from a package below a workspace that does not list it.
const STANDALONE: &str =
    "[workspace]\n\n[package]\nname = \"harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// The manifest of a workspace whose package depends on its member
///
/// The dependency lives in the project, so the resolution needs no network,
/// and it still gives the lockfile something to hold.
const WITH_MEMBER: &str = "[workspace]\nmembers = [\"dep\"]\n\n[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\ndep = { path = \"dep\" }\n";

/// The manifest of the member that the workspace above depends on
const MEMBER: &str = "[package]\nname = \"dep\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// The library of that member
const MEMBER_LIBRARY: &str = "pub fn value() -> i32 {\n    1\n}\n";

/// The manifest of a package whose dependency lies at a path that nothing
/// holds
///
/// Cargo describes the workspace of such a manifest without a word.
/// The reason is that the description reads no dependency.
/// The resolution then fails over the directory that is not there.
const MISSING_DEPENDENCY: &str = "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nmissing = { path = \"missing\" }\n";

/// A lockfile that names the package of the project and not its member
///
/// A build that holds cargo to this lockfile ends without success, because the
/// member is missing from it. A build that runs after an update passes,
/// because the update wrote the member into it.
const STALE_LOCK: &str = "version = 4\n\n[[package]]\nname = \"probe\"\nversion = \"0.1.0\"\n";

/// A manifest with a build script that fails
///
/// Cargo reports the failure on its standard error stream and not
/// as a diagnostic of the compiler.
/// So the report holds nothing that the action recognizes.
const FAILING_BUILD: &str =
    "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n";

/// A build script that fails
const FAILING_SCRIPT: &str = "fn main() {\n    std::process::exit(1);\n}\n";

/// A library with one test that passes
const PASSING: &str = "pub fn value() -> i32 {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn passes() {\n        assert_eq!(super::value(), 1);\n    }\n}\n";

/// A library with one test that passes through the member of the workspace
const PASSING_THROUGH_MEMBER: &str = "pub fn value() -> i32 {\n    dep::value()\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn passes() {\n        assert_eq!(super::value(), 1);\n    }\n}\n";

/// A library with one test that fails
const FAILING: &str = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn fails() {\n        panic!(\"the probe fails on purpose\");\n    }\n}\n";

/// A library without a test
const UNTESTED: &str = "pub fn value() -> i32 {\n    1\n}\n";

/// A library that the compiler refuses
const BROKEN: &str = "pub fn broken() -> i32 {\n    \"no\"\n}\n";

/// The prefix of the names of the environment that git reads for itself
///
/// A test that runs inside a hook, a rebase, or a bisect of another repository
/// inherits variables that name that repository.
/// Those variables beat the directory that a command runs in.
/// They therefore leave the environment of every git command of a test.
const GIT_PREFIX: &[u8] = b"GIT_";

/// The key that pins nextest in the mise configuration of a project
const UNINSTALLED_PIN_KEY: &str = "\"cargo:cargo-nextest\"";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project without a cargo to resolve and without a repository
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool
    /// runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the cargo of this repository, in a repository
    ///
    /// The project copies the `mise.toml` of this repository, so the cargo
    /// that mise resolves for it is the one that the repository pins and
    /// installs. The copy pins a nextest that nothing installs. Mise ignores
    /// a configuration that nobody trusts, so the copy is trusted right
    /// away.
    ///
    /// The repository holds one empty commit.
    /// Every file that a test writes afterwards is untracked.
    /// This is the tree of a contributor who has not committed their work yet.
    fn new() -> Self {
        let project = Self::without_repository();

        project.git(&["init", "--quiet", "--initial-branch=main", "."]);
        project.git(&["config", "user.name", "Rakko"]);
        project.git(&["config", "user.email", "rakko@example.com"]);
        project.git(&["config", "commit.gpgsign", "false"]);
        project.git(&["commit", "--quiet", "--allow-empty", "--message", "Start"]);

        project
    }

    /// Creates a project with one package whose library holds the given source
    fn with_library(source: &str) -> Self {
        let project = Self::new();
        project.write().path("Cargo.toml").content(PACKAGE).call();
        project.write().path("src/lib.rs").content(source).call();

        project
    }

    /// Creates a project with the cargo of this repository, outside a
    /// repository
    ///
    /// A test uses this shape when the run must reach the copy and find that
    /// there is no repository to copy from.
    fn without_repository() -> Self {
        let project = Self::bare();

        let copy = project.directory.path().join("mise.toml");
        std::fs::write(&copy, pins_with_an_uninstalled_tool())
            .expect("the test writes the mise.toml of the project");
        trust(&copy);

        project
    }

    /// Creates a project that pins a Rust toolchain that nothing installed
    ///
    /// The pin names a version that no machine installs.
    /// So mise reports no location for cargo, whatever the global
    /// configuration of the machine says about Rust.
    fn without_cargo() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\nrust = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);
        project.write().path("Cargo.toml").content(PACKAGE).call();
        project.write().path("src/lib.rs").content(PASSING).call();

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical.
    /// So the paths that the run reports do not depend on the symbolic
    /// links of the temporary directory.
    fn context(&self) -> Context {
        let root = ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists");

        Context::builder().root(root).build()
    }

    /// Runs git in the project
    ///
    /// The command forgets the variables that name a repository.
    /// So a test that runs inside a hook of another repository
    /// still works on the one that it created.
    fn git(&self, arguments: &[&str]) {
        let mut command = Command::new("git");
        command.current_dir(self.directory.path()).args(arguments);

        for (name, _) in std::env::vars_os() {
            if name.as_encoded_bytes().starts_with(GIT_PREFIX) {
                command.env_remove(name);
            }
        }

        let status = command.status().expect("the test starts git");

        assert!(status.success(), "expected git to run {arguments:?}");
    }

    /// Returns whether the project holds anything at the path
    fn holds(&self, path: &str) -> bool {
        self.directory.path().join(path).exists()
    }

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        CheckLatestDeps.run(&self.context(), &()).await
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
    /// Mise records the trust of a configuration in the state of the user. The
    /// temporary directory of a test never comes back, so the record would pile
    /// up there forever. A failure stays quiet, because the test already
    /// reported what it was about.
    fn drop(&mut self) {
        let _ = Command::new("mise")
            .arg("trust")
            .arg("--quiet")
            .arg("--untrust")
            .arg(self.directory.path().join("mise.toml"))
            .status();
    }
}

/// Returns the paths that the findings of an outcome name
fn locations(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::File { path } => path.to_string(),
            Location::Position { path, .. } => path.to_string(),
            Location::Span { path, .. } => path.to_string(),
            other => panic!("expected a finding with a path, got {other:?}"),
        })
        .collect()
}

/// Returns the mise configuration of this repository, with a pin of
/// nextest that nothing installed
///
/// The run resolves the nextest that the action declares, so the pin of the
/// project must not change what runs.
fn pins_with_an_uninstalled_tool() -> String {
    let pins = std::fs::read_to_string(repository().join("mise.toml"))
        .expect("the test reads the mise.toml of the repository");

    let pin = pins
        .lines()
        .find(|line| line.starts_with(UNINSTALLED_PIN_KEY))
        .expect("the mise.toml of the repository pins nextest");

    pins.replace(pin, &format!("{UNINSTALLED_PIN_KEY} = \"0.0.1\""))
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

#[test]
fn action_declares_no_argument() {
    let schema = <CheckLatestDeps as Action>::Args::schema();

    assert!(schema.arguments().is_empty());
}

#[test]
fn action_identifies_itself_as_check_latest_deps() {
    let name = CheckLatestDeps.name();

    assert_eq!(name.get(), "check-latest-deps");
}

#[tokio::test]
async fn run_in_a_project_with_a_passing_test_counts_it() {
    let project = Project::with_library(PASSING);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("updated 1 workspace and ran 1 test")
    );
}

#[tokio::test]
async fn run_in_a_project_with_a_passing_test_passes() {
    let project = Project::with_library(PASSING);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_without_a_repository_stops() {
    let project = Project::without_repository();
    project.write().path("Cargo.toml").content(PACKAGE).call();
    project.write().path("src/lib.rs").content(PASSING).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_workspace_without_tests_counts_zero() {
    let project = Project::with_library(UNTESTED);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("updated 1 workspace and ran 0 tests")
    );
}

/// Shows that a run leaves the lockfile of the project alone
///
/// The lockfile of the project is the one file that the recipe this action
/// replaces used to leave rewritten.
/// That is the reason that the recipe refused a tree with changes in it.
#[tokio::test]
async fn run_leaves_the_lockfile_of_the_project_alone() {
    let project = Project::new();
    project
        .write()
        .path("Cargo.toml")
        .content(WITH_MEMBER)
        .call();
    project
        .write()
        .path("src/lib.rs")
        .content(PASSING_THROUGH_MEMBER)
        .call();
    project
        .write()
        .path("dep/Cargo.toml")
        .content(MEMBER)
        .call();
    project
        .write()
        .path("dep/src/lib.rs")
        .content(MEMBER_LIBRARY)
        .call();
    project
        .write()
        .path("Cargo.lock")
        .content(STALE_LOCK)
        .call();

    project.run().await;

    assert_eq!(project.read("Cargo.lock"), STALE_LOCK);
}

#[tokio::test]
async fn run_leaves_the_project_without_a_build_directory() {
    let project = Project::with_library(PASSING);

    project.run().await;

    assert!(!project.holds("target"));
}

#[tokio::test]
async fn run_with_a_build_script_that_fails_stops() {
    let project = Project::new();
    project
        .write()
        .path("Cargo.toml")
        .content(FAILING_BUILD)
        .call();
    project
        .write()
        .path("build.rs")
        .content(FAILING_SCRIPT)
        .call();
    project.write().path("src/lib.rs").content(PASSING).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

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

#[tokio::test]
async fn run_with_a_dependency_that_cargo_cannot_resolve_fails_at_the_manifest() {
    let project = Project::new();
    project
        .write()
        .path("Cargo.toml")
        .content(MISSING_DEPENDENCY)
        .call();
    project.write().path("src/lib.rs").content(PASSING).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["Cargo.toml"]);
}

#[tokio::test]
async fn run_with_a_dependency_that_cargo_cannot_resolve_holds_what_cargo_wrote() {
    let project = Project::new();
    project
        .write()
        .path("Cargo.toml")
        .content(MISSING_DEPENDENCY)
        .call();
    project.write().path("src/lib.rs").content(PASSING).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0].message().get().contains("missing"),
        "expected the diagnosis of cargo, got {findings:?}"
    );
}

/// Shows that a dependency that cargo cannot resolve runs no test
///
/// A run that tested a workspace whose resolution failed would answer
/// for a lockfile that nobody wrote.
/// So the finding of the update is the only one that such a run reports.
#[tokio::test]
async fn run_with_a_dependency_that_cargo_cannot_resolve_runs_no_test() {
    let project = Project::new();
    project
        .write()
        .path("Cargo.toml")
        .content(MISSING_DEPENDENCY)
        .call();
    project.write().path("src/lib.rs").content(FAILING).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(findings.len(), 1);
}

#[tokio::test]
async fn run_with_a_failing_test_fails() {
    let project = Project::with_library(FAILING);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

/// Shows that a failing test in a second workspace names its path
///
/// The run works in the copy.
/// The finding still names the file that the contributor can open.
#[tokio::test]
async fn run_with_a_failing_test_in_a_second_workspace_names_its_path() {
    let project = Project::with_library(PASSING);
    project
        .write()
        .path("tools/harness/Cargo.toml")
        .content(STANDALONE)
        .call();
    project
        .write()
        .path("tools/harness/src/lib.rs")
        .content(FAILING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("tools/harness/src/lib.rs")]);
}

#[tokio::test]
async fn run_with_a_manifest_that_cargo_cannot_read_stops() {
    let project = Project::new();
    project
        .write()
        .path("Cargo.toml")
        .content("this is not a manifest\n")
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_with_a_stale_lockfile_leaves_the_project_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("Cargo.toml")
        .content(WITH_MEMBER)
        .call();
    project
        .write()
        .path("src/lib.rs")
        .content(PASSING_THROUGH_MEMBER)
        .call();
    project
        .write()
        .path("dep/Cargo.toml")
        .content(MEMBER)
        .call();
    project
        .write()
        .path("dep/src/lib.rs")
        .content(MEMBER_LIBRARY)
        .call();
    project
        .write()
        .path("Cargo.lock")
        .content(STALE_LOCK)
        .call();

    run_checking_access(&CheckLatestDeps, &project.context(), &()).await;
}

/// Shows that a run with a stale lockfile passes
///
/// The lockfile of the project names no member.
/// So a build that holds cargo to it ends without success.
/// The run passes. It can only do so when the update wrote
/// the member into the lockfile of the copy and the tests built
/// what that lockfile holds.
#[tokio::test]
async fn run_with_a_stale_lockfile_passes() {
    let project = Project::new();
    project
        .write()
        .path("Cargo.toml")
        .content(WITH_MEMBER)
        .call();
    project
        .write()
        .path("src/lib.rs")
        .content(PASSING_THROUGH_MEMBER)
        .call();
    project
        .write()
        .path("dep/Cargo.toml")
        .content(MEMBER)
        .call();
    project
        .write()
        .path("dep/src/lib.rs")
        .content(MEMBER_LIBRARY)
        .call();
    project
        .write()
        .path("Cargo.lock")
        .content(STALE_LOCK)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_two_workspaces_sums_the_tests() {
    let project = Project::with_library(PASSING);
    project
        .write()
        .path("tools/harness/Cargo.toml")
        .content(STANDALONE)
        .call();
    project
        .write()
        .path("tools/harness/src/lib.rs")
        .content(PASSING)
        .call();

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("updated 2 workspaces and ran 2 tests")
    );
}

#[tokio::test]
async fn run_without_a_cargo_stops() {
    let project = Project::without_cargo();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(CheckLatestDepsError::UnresolvedCargo { .. }))
        ),
        "expected the run to stop without a cargo, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_without_a_manifest_skips() {
    let project = Project::new();
    project
        .write()
        .path("README.md")
        .content("# Project\n")
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

#[test]
fn tools_declare_the_stable_toolchain_and_nextest() {
    let tools: Vec<String> = CheckLatestDeps
        .tools()
        .iter()
        .map(|requirement| requirement.tool().get().to_owned())
        .collect();

    assert_eq!(tools, ["rust", "cargo:cargo-nextest"]);
}
