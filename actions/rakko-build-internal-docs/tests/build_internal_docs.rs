//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately faulty crate sits in this
//! repository, where the lints of the repository itself would fight it.
//!
//! The tests run the cargo that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers
//! is the version that the repository installs, and a new pin reaches the
//! tests without a change to them.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Summary};
use rakko_build_internal_docs::BuildInternalDocs;
use tempfile::TempDir;

/// The manifest of a package that rustdoc documents
const PACKAGE: &str = "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// The manifest of a package with a feature that is off by default
const FEATURED: &str = "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\nextra = []\n";

/// The manifest of a package that belongs to no workspace
///
/// The empty workspace table tells cargo that the package is a root of its
/// own, which cargo demands from a package below a workspace that does not
/// list it.
const STANDALONE: &str =
    "[workspace]\n\n[package]\nname = \"harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// A manifest with a build script that fails
///
/// Cargo reports the failure on its standard error stream and not as a
/// diagnostic of the compiler, so the report holds nothing that the action
/// recognizes.
const FAILING_BUILD: &str =
    "[package]\nname = \"probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n";

/// A source file that rustdoc documents without a word
const CLEAN: &str = "pub fn clean() {}\n";

/// A source file whose public item links to nothing
///
/// The link sits on the first line, and the name that it fails to resolve
/// starts at the seventeenth column.
const BROKEN_LINK: &str = "/// A link to [`Missing`].\npub fn documented() {}\n";

/// A source file whose private item links to nothing
///
/// Rustdoc reads the documentation of a private item only when the run asks
/// for the private items.
const PRIVATE_LINK: &str = "/// A link to [`Missing`].\nfn hidden() {}\n";

/// A source file whose item behind a feature links to nothing
///
/// Rustdoc reads the documentation of the item only when the feature is on,
/// and the feature of the manifest is off by default.
const GATED_LINK: &str =
    "#[cfg(feature = \"extra\")]\n/// A link to [`Missing`].\npub fn gated() {}\n";

/// A source file that rustdoc refuses
const BROKEN: &str = "pub fn broken() -> NoSuchType {\n    todo!()\n}\n";

/// A build script that fails
const FAILING_SCRIPT: &str = "fn main() {\n    std::process::exit(1);\n}\n";

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

    /// Creates a project with one package whose library holds the given
    /// source
    fn with_library(source: &str) -> Self {
        Self::with_manifest(PACKAGE, source)
    }

    /// Creates a project with one package of the given manifest and source
    fn with_manifest(manifest: &str, source: &str) -> Self {
        let project = Self::new();
        project.write("Cargo.toml", manifest);
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
        BuildInternalDocs.run(&self.context(), &()).await
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
            Location::Span { path, .. } => path.to_string(),
            other => panic!("expected a finding with a range, got {other:?}"),
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

// buildinternaldocs[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <BuildInternalDocs as Action>::Args::schema();

    assert!(schema.arguments().is_empty());
}

// buildinternaldocs[verify name]
#[test]
fn action_identifies_itself_as_build_internal_docs() {
    let name = BuildInternalDocs.name();

    assert_eq!(name.get(), "build-internal-docs");
}

// buildinternaldocs[verify build.passed]
#[tokio::test]
async fn run_in_a_clean_project_counts_the_workspaces() {
    let project = Project::with_library(CLEAN);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("documented 1 workspace")
    );
}

// buildinternaldocs[verify build.operation]
// buildinternaldocs[verify build.passed]
// buildinternaldocs[verify tool.cargo]
#[tokio::test]
async fn run_in_a_clean_project_passes() {
    let project = Project::with_library(CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// buildinternaldocs[verify build.sources]
#[tokio::test]
async fn run_leaves_the_sources_unchanged() {
    let project = Project::with_library(BROKEN_LINK);

    project.run().await;

    assert_eq!(project.read("src/lib.rs"), BROKEN_LINK);
}

// buildinternaldocs[verify build.operation]
#[tokio::test]
async fn run_with_a_broken_link_behind_a_feature_fails() {
    let project = Project::with_manifest(FEATURED, GATED_LINK);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// buildinternaldocs[verify build.diagnostic]
#[tokio::test]
async fn run_with_a_broken_link_carries_the_code_in_the_message() {
    let project = Project::with_library(BROKEN_LINK);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings.iter().any(|finding| finding
            .message()
            .get()
            .ends_with("[rustdoc::broken_intra_doc_links]")),
        "expected the code of the lint, got {findings:?}"
    );
}

// buildinternaldocs[verify build.failed]
#[tokio::test]
async fn run_with_a_broken_link_fails() {
    let project = Project::with_library(BROKEN_LINK);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// buildinternaldocs[verify build.operation]
#[tokio::test]
async fn run_with_a_broken_link_in_a_private_item_fails() {
    let project = Project::with_library(PRIVATE_LINK);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// buildinternaldocs[verify roots.all]
#[tokio::test]
async fn run_with_a_broken_link_in_a_second_workspace_names_its_path() {
    let project = Project::with_library(CLEAN);
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/lib.rs", BROKEN_LINK);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings)[0], "tools/harness/src/lib.rs");
}

// buildinternaldocs[verify build.diagnostic]
#[tokio::test]
async fn run_with_a_broken_link_reports_the_range() {
    let project = Project::with_library(BROKEN_LINK);

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
        (1, Some(17))
    );
}

// buildinternaldocs[verify build.unrecognized]
#[tokio::test]
async fn run_with_a_build_script_that_fails_reports_what_cargo_said() {
    let project = Project::new();
    project.write("Cargo.toml", FAILING_BUILD);
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

// buildinternaldocs[verify build.unrecognized]
#[tokio::test]
async fn run_with_a_build_script_that_fails_stops() {
    let project = Project::new();
    project.write("Cargo.toml", FAILING_BUILD);
    project.write("build.rs", FAILING_SCRIPT);
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// buildinternaldocs[verify build.failed]
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
            .any(|finding| finding.message().get().contains("cannot find type")),
        "expected the error of the compiler, got {findings:?}"
    );
}

// buildinternaldocs[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_a_manifest_only_behind_a_symbolic_link_skips() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("Cargo.toml"), PACKAGE)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// buildinternaldocs[verify skip.git]
#[tokio::test]
async fn run_with_a_manifest_only_under_the_git_directory_skips() {
    let project = Project::bare();
    project.write(".git/Cargo.toml", PACKAGE);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// buildinternaldocs[verify skip.target]
#[tokio::test]
async fn run_with_a_manifest_only_under_the_target_directory_skips() {
    let project = Project::bare();
    project.write("target/debug/build/dep/Cargo.toml", PACKAGE);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// buildinternaldocs[verify roots.error]
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

// buildinternaldocs[verify roots.all]
#[tokio::test]
async fn run_with_two_workspaces_counts_both() {
    let project = Project::with_library(CLEAN);
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/lib.rs", CLEAN);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("documented 2 workspaces")
    );
}

// buildinternaldocs[verify tool.missing]
#[tokio::test]
async fn run_without_a_cargo_stops() {
    let project = Project::without_cargo();
    project.write("Cargo.toml", PACKAGE);
    project.write("src/lib.rs", CLEAN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// buildinternaldocs[verify skip.missing]
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

// buildinternaldocs[verify skip.missing]
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
