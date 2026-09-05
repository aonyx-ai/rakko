//! Tests that drive the crate against real projects
//!
//! Each test builds a project in a temporary directory and runs nextest
//! against it, so no fixture with a deliberately failing test sits in this
//! repository, where the tests of the repository itself would fight it.
//!
//! The tests run the cargo and the nextest that this repository pins. A
//! project copies the `mise.toml` of the repository and trusts it, so the
//! versions that answer are the versions that the repository installs, and a
//! new pin reaches the tests without a change to them.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Location, Position, ProjectRoot};
use rakko_cargo::Cargo;
use rakko_nextest::{Lockfile, Nextest, Observation, ObserveNextestError};
use tempfile::TempDir;

/// The manifest of a package that nextest tests
const PACKAGE: &str = "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// A manifest with a build script that fails
///
/// Cargo reports the failure on its standard error stream and not as a
/// diagnostic of the compiler, so the report holds nothing that the crate
/// recognizes.
const FAILING_BUILD: &str =
    "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n";

/// A build script that fails
const FAILING_SCRIPT: &str = "fn main() {\n    std::process::exit(1);\n}\n";

/// A library with one test that passes
const PASSING: &str = "pub fn value() -> i32 {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn passes() {\n        assert_eq!(super::value(), 1);\n    }\n}\n";

/// A library with one test that fails
///
/// The panic sits on the fifth line, at the ninth column.
const FAILING: &str = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn fails() {\n        panic!(\"the probe fails on purpose\");\n    }\n}\n";

/// A library without a test
const UNTESTED: &str = "pub fn value() -> i32 {\n    1\n}\n";

/// A library that the compiler refuses
const BROKEN: &str = "pub fn broken() -> i32 {\n    \"no\"\n}\n";

/// The variable that gives consent to the experimental report of nextest
const CONSENT_VARIABLE: &str = "NEXTEST_EXPERIMENTAL_LIBTEST_JSON";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project with the cargo of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the cargo
    /// and the nextest that mise resolves for it are the ones that the
    /// repository pins and installs. Mise ignores a configuration that
    /// nobody trusts, so the copy is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
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

    /// Tests the one workspace of the project and returns what it reported
    ///
    /// Cargo resolves the dependencies of the build, which is what a run
    /// against the project as it stands does.
    ///
    /// # Errors
    ///
    /// Returns the error of the run when the reports of nextest and cargo
    /// leave it without an answer.
    async fn observe(&self) -> Result<Observation, ObserveNextestError> {
        self.observe_with(Lockfile::Writable).await
    }

    /// Tests the one workspace of the project the way the lockfile says
    ///
    /// # Errors
    ///
    /// Returns the error of the run when the reports of nextest and cargo
    /// leave it without an answer.
    async fn observe_with(&self, lockfile: Lockfile) -> Result<Observation, ObserveNextestError> {
        let root = self.root();
        let cargo = Cargo::resolve(root.clone())
            .await
            .expect("the test resolves the cargo that the project pins");
        let workspace = cargo
            .roots()
            .await
            .expect("the test discovers the workspaces of the project")
            .into_iter()
            .next()
            .expect("the project holds one workspace");

        Nextest::new(cargo, lockfile)
            .observe(&workspace, &root)
            .await
    }

    /// Returns whether the project holds a file at the path
    fn holds(&self, path: &str) -> bool {
        self.directory.path().join(path).exists()
    }

    /// Returns the root of the project
    ///
    /// The root is canonical, so the paths that a run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn root(&self) -> ProjectRoot {
        let root = self
            .directory
            .path()
            .canonicalize()
            .expect("the test names a directory that exists");

        ProjectRoot::new(root)
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

// nextest[verify report.unrecognized]
#[tokio::test]
async fn observe_a_build_script_that_fails_holds_what_nextest_wrote() {
    let project = Project::new();
    project.write("Cargo.toml", FAILING_BUILD);
    project.write("build.rs", FAILING_SCRIPT);
    project.write("src/lib.rs", PASSING);

    let observation = project.observe().await;

    let Err(ObserveNextestError::UnrecognizedReport { stderr, .. }) = &observation else {
        panic!("expected an unrecognized report, got {observation:?}");
    };
    assert!(
        stderr.contains("build"),
        "expected the diagnosis of the build, got {stderr}"
    );
}

// nextest[verify report.unrecognized]
#[tokio::test]
async fn observe_a_build_script_that_fails_stops() {
    let project = Project::new();
    project.write("Cargo.toml", FAILING_BUILD);
    project.write("build.rs", FAILING_SCRIPT);
    project.write("src/lib.rs", PASSING);

    let observation = project.observe().await;

    assert!(
        matches!(
            observation,
            Err(ObserveNextestError::UnrecognizedReport { .. })
        ),
        "expected an unrecognized report, got {observation:?}"
    );
}

// nextest[verify finding.build]
#[tokio::test]
async fn observe_a_compiler_error_reports_the_diagnostic() {
    let project = Project::with_library(BROKEN);

    let observation = project.observe().await;

    let Ok(observation) = &observation else {
        panic!("expected an observation, got {observation:?}");
    };
    assert!(
        observation
            .findings()
            .iter()
            .any(|finding| finding.message().get().contains("mismatched types")),
        "expected the error of the compiler, got {:?}",
        observation.findings()
    );
}

// nextest[verify finding.failed]
#[tokio::test]
async fn observe_a_failing_test_carries_the_message_of_the_panic() {
    let project = Project::with_library(FAILING);

    let observation = project.observe().await;

    let Ok(observation) = &observation else {
        panic!("expected an observation, got {observation:?}");
    };
    assert!(
        observation.findings()[0]
            .message()
            .get()
            .contains("the probe fails on purpose"),
        "expected the message of the panic, got {:?}",
        observation.findings()[0].message()
    );
}

// nextest[verify finding.failed]
#[tokio::test]
async fn observe_a_failing_test_names_the_test() {
    let project = Project::with_library(FAILING);

    let observation = project.observe().await;

    let Ok(observation) = &observation else {
        panic!("expected an observation, got {observation:?}");
    };
    assert!(
        observation.findings()[0]
            .message()
            .get()
            .contains("tests::fails"),
        "expected the name of the test, got {:?}",
        observation.findings()[0].message()
    );
}

// nextest[verify finding.position]
#[tokio::test]
async fn observe_a_failing_test_reports_where_it_panicked() {
    let project = Project::with_library(FAILING);

    let observation = project.observe().await;

    let Ok(observation) = &observation else {
        panic!("expected an observation, got {observation:?}");
    };
    let Location::Position { path, position } = observation.findings()[0].location() else {
        panic!(
            "expected a finding at a position, got {:?}",
            observation.findings()[0]
        );
    };
    assert_eq!(
        (path.to_string(), *position),
        (
            "src/lib.rs".to_owned(),
            Position::builder().line(5).column(9).build()
        )
    );
}

// nextest[verify report.ran]
// nextest[verify run.consent]
// nextest[verify run.operation+2]
#[tokio::test]
async fn observe_a_passing_test_counts_it() {
    let project = Project::with_library(PASSING);

    let observation = project.observe().await;

    let Ok(observation) = &observation else {
        panic!("expected an observation, got {observation:?}");
    };
    assert_eq!(observation.ran(), 1);
}

// nextest[verify run.operation+2]
#[tokio::test]
async fn observe_a_passing_test_finds_nothing() {
    let project = Project::with_library(PASSING);

    let observation = project.observe().await;

    let Ok(observation) = &observation else {
        panic!("expected an observation, got {observation:?}");
    };
    assert!(
        observation.findings().is_empty(),
        "expected no finding, got {:?}",
        observation.findings()
    );
}

// A workspace that nothing resolved yet holds no lockfile, so a run that
// builds the versions of the lockfile has none to build and refuses. That
// refusal is a report that the crate cannot answer from, which is what a
// caller reads when the resolution it asked about is not the one on disk.
// nextest[verify run.lockfile]
#[tokio::test]
async fn observe_a_workspace_without_a_lockfile_with_a_locked_run_stops() {
    let project = Project::with_library(PASSING);

    let observation = project.observe_with(Lockfile::Locked).await;

    assert!(
        matches!(
            observation,
            Err(ObserveNextestError::UnrecognizedReport { .. })
        ),
        "expected the run to stop, got {observation:?}"
    );
}

// nextest[verify run.lockfile]
#[tokio::test]
async fn observe_a_workspace_without_a_lockfile_with_a_writable_run_resolves_it() {
    let project = Project::with_library(PASSING);

    let _observation = project.observe_with(Lockfile::Writable).await;

    assert!(project.holds("Cargo.lock"));
}

// nextest[verify report.none]
#[tokio::test]
async fn observe_a_workspace_without_a_test_counts_zero() {
    let project = Project::with_library(UNTESTED);

    let observation = project.observe().await;

    let Ok(observation) = &observation else {
        panic!("expected an observation, got {observation:?}");
    };
    assert_eq!(observation.ran(), 0);
}

// nextest[verify report.none]
#[tokio::test]
async fn observe_a_workspace_without_a_test_finds_nothing() {
    let project = Project::with_library(UNTESTED);

    let observation = project.observe().await;

    let Ok(observation) = &observation else {
        panic!("expected an observation, got {observation:?}");
    };
    assert!(
        observation.findings().is_empty(),
        "expected no finding, got {:?}",
        observation.findings()
    );
}

// nextest[verify run.consent]
#[tokio::test]
async fn observe_gives_consent_to_no_other_process() {
    let project = Project::with_library(PASSING);
    let before = std::env::var_os(CONSENT_VARIABLE);

    let _observation = project.observe().await;

    assert_eq!(std::env::var_os(CONSENT_VARIABLE), before);
}
