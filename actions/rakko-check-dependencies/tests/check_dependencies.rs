//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a license that this repository refuses sits
//! in the repository, where the checks of the repository itself would fight
//! it.
//!
//! The tests run the cargo-deny that this repository pins. A project copies
//! the `mise.toml` of the repository and trusts it, so the version that
//! answers is the version that the repository installs, and a new pin reaches
//! the tests without a change to them.
//!
//! Every project depends on nothing outside itself. Cargo-deny then resolves
//! the graph of the project without a registry, so the tests need no network
//! and answer the same on every machine.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, DirectoryPath, Finding, Location, Outcome};
use rakko_check_dependencies::CheckDependencies;
use tempfile::TempDir;

/// A package whose license the tests accept
const PACKAGE: &str = "\
[package]
name = \"otter\"
version = \"0.1.0\"
edition = \"2024\"
license = \"MIT\"
";

/// A workspace that holds one member next to the package of its root
///
/// The member is no dependency of the root package, so a run that takes the
/// root package as the only root of the graph never reaches it.
const WORKSPACE_WITH_MEMBER: &str = "\
[package]
name = \"otter\"
version = \"0.1.0\"
edition = \"2024\"
license = \"MIT\"

[workspace]
members = [\"crates/sidecar\"]
";

/// A member of a workspace whose license the tests refuse
const MEMBER: &str = "\
[package]
name = \"sidecar\"
version = \"0.1.0\"
edition = \"2024\"
license = \"Apache-2.0\"
";

/// A workspace of its own, below the root of the project
const HARNESS: &str = "\
[workspace]

[package]
name = \"harness\"
version = \"0.1.0\"
edition = \"2024\"
license = \"Apache-2.0\"
";

/// The library of a package that a test builds
const LIBRARY: &str = "pub fn otter() {}\n";

/// A configuration that accepts the license of the package at the root
const ALLOW_MIT: &str = "[licenses]\nallow = [\"MIT\"]\n";

/// A configuration that accepts the license of the workspace below the root
///
/// The package at the root of a project carries the other license, so this
/// configuration refuses it.
const ALLOW_APACHE: &str = "[licenses]\nallow = [\"Apache-2.0\"]\n";

/// A configuration that allows a license which no package of a project has
///
/// Cargo-deny reports the unused permission as a warning, which is the weight
/// that a project gives a shape that it wants to read about and not fail
/// over.
const UNUSED: &str = "[licenses]\nallow = [\"MIT\", \"MPL-2.0\"]\n";

/// A configuration that cargo-deny refuses to read
const BROKEN: &str = "no-such-key = true\n";

/// A configuration whose advisory database no run can reach
///
/// The path cannot be created and the address does not resolve, so a run that
/// asked cargo-deny for the advisories check would stop before it checked
/// anything. A run that asks for the other three checks never reads either.
const UNREACHABLE_ADVISORIES: &str = "\
[licenses]
allow = [\"MIT\"]

[advisories]
db-path = \"/nonexistent/rakko-advisory-db\"
db-urls = [\"https://example.invalid/advisory-db\"]
";

/// The directory that holds the second workspace of a project
const TOOLS: &str = "tools/harness";

/// The key that pins cargo-deny in the mise configuration of a project
const DENY_PIN: &str = "\"cargo:cargo-deny\"";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a tool to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before a tool runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the tools of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the cargo
    /// and the cargo-deny that mise resolves for it are the ones that the
    /// repository pins and installs. Mise ignores a configuration that nobody
    /// trusts, so the copy is trusted right away.
    fn new() -> Self {
        let project = Self::bare();
        project.pin(&pins());

        project
    }

    /// Creates a project with one package that the given configuration judges
    fn with_configuration(configuration: &str) -> Self {
        let project = Self::new();
        project.write("Cargo.toml", PACKAGE);
        project.write("src/lib.rs", LIBRARY);
        project.write("deny.toml", configuration);

        project
    }

    /// Creates a project that pins a cargo-deny that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for cargo-deny, whatever the global configuration of the
    /// machine says about it. The other pins of the repository stay, so the
    /// cargo of the run resolves and the run reaches the missing tool.
    fn without_deny() -> Self {
        let project = Self::bare();
        let pins: Vec<String> = pins()
            .lines()
            .map(|line| {
                if line.starts_with(DENY_PIN) {
                    format!("{DENY_PIN} = \"0.0.1\"")
                } else {
                    line.to_owned()
                }
            })
            .collect();
        project.pin(&pins.join("\n"));

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
        CheckDependencies.run(&self.context(), &()).await
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

/// Returns the findings of an outcome that failed
fn findings(outcome: &Outcome) -> &[Finding] {
    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };

    findings
}

/// Returns the messages of the given findings
fn messages(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.message().get())
        .collect()
}

/// Returns the mise configuration of this repository
fn pins() -> String {
    std::fs::read_to_string(repository().join("mise.toml"))
        .expect("the test reads the mise.toml of the repository")
}

/// Returns the root of the repository that the tests run in
fn repository() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));

    manifest
        .parent()
        .and_then(Path::parent)
        .expect("the crate lives two directories below the root of the repository")
}

/// Returns what a passing run said about the project
fn summary(outcome: &Outcome) -> String {
    let Outcome::Passed { summary } = outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };

    summary
        .as_ref()
        .expect("the action reports what a passing run examined")
        .get()
        .to_owned()
}

/// Returns a project of two workspaces, the second below `tools/harness`
///
/// The package at the root of the project carries one license and the package
/// of the second workspace carries the other, and each workspace holds the
/// configuration that the caller gives it. A caller therefore decides which of
/// the two answers, and whether either of them refuses what it found.
fn two_workspaces(harness: &str) -> Project {
    let project = Project::with_configuration(ALLOW_MIT);
    project.write(&format!("{TOOLS}/Cargo.toml"), HARNESS);
    project.write(&format!("{TOOLS}/src/lib.rs"), LIBRARY);
    project.write(&format!("{TOOLS}/deny.toml"), harness);

    project
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

// checkdependencies[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <<CheckDependencies as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

// checkdependencies[verify name]
#[test]
fn action_identifies_itself_as_check_dependencies() {
    let name = CheckDependencies.name();

    assert_eq!(name.get(), "check-dependencies");
}

// checkdependencies[verify roots.all]
#[tokio::test]
async fn run_checks_every_workspace_of_the_project() {
    let project = two_workspaces(ALLOW_APACHE);

    let outcome = project.run().await;

    assert_eq!(summary(&outcome), "checked 2 workspaces");
}

// checkdependencies[verify run.checks]
#[tokio::test]
async fn run_does_not_ask_for_the_advisories_check() {
    let project = Project::with_configuration(UNREACHABLE_ADVISORIES);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to leave the advisory database alone, got {outcome:?}"
    );
}

// checkdependencies[verify check.passed]
#[tokio::test]
async fn run_in_a_clean_project_counts_the_workspace_that_it_checked() {
    let project = Project::with_configuration(ALLOW_MIT);

    let outcome = project.run().await;

    assert_eq!(summary(&outcome), "checked 1 workspace");
}

// checkdependencies[verify check.passed]
// checkdependencies[verify tool.cargo]
// checkdependencies[verify tool.deny]
#[tokio::test]
async fn run_in_a_clean_project_passes() {
    let project = Project::with_configuration(ALLOW_MIT);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// checkdependencies[verify skip.missing]
#[tokio::test]
async fn run_in_a_project_without_a_manifest_names_what_it_looked_for() {
    let project = Project::without_deny();
    project.write("README.md", "# Notes\n");

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert_eq!(reason.get(), "the project holds no file named Cargo.toml");
}

// checkdependencies[verify skip.missing]
#[tokio::test]
async fn run_in_a_project_without_a_manifest_skips() {
    let project = Project::without_deny();
    project.write("README.md", "# Notes\n");

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checkdependencies[verify check.read]
#[tokio::test]
async fn run_leaves_the_manifest_of_a_refused_package_unchanged() {
    let project = Project::with_configuration(ALLOW_APACHE);

    project.run().await;

    assert_eq!(project.read("Cargo.toml"), PACKAGE);
}

// checkdependencies[verify roots.members]
#[tokio::test]
async fn run_reaches_a_member_that_no_other_member_depends_on() {
    let project = Project::new();
    project.write("Cargo.toml", WORKSPACE_WITH_MEMBER);
    project.write("src/lib.rs", LIBRARY);
    project.write("crates/sidecar/Cargo.toml", MEMBER);
    project.write("crates/sidecar/src/lib.rs", LIBRARY);
    project.write("deny.toml", ALLOW_MIT);

    let outcome = project.run().await;

    assert!(
        messages(findings(&outcome))
            .contains(&"[rejected] failed to satisfy license requirements (sidecar 0.1.0)"),
        "expected the member to be checked, got {:?}",
        messages(findings(&outcome))
    );
}

// checkdependencies[verify run.directory]
#[tokio::test]
async fn run_reads_the_configuration_that_lies_closest_to_a_workspace() {
    let project = two_workspaces(ALLOW_APACHE);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the workspace below the root to answer from its own \
         configuration, got {outcome:?}"
    );
}

// checkdependencies[verify skip.links]
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

// checkdependencies[verify skip.git]
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

// checkdependencies[verify skip.target]
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

// checkdependencies[verify roots.error]
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

// checkdependencies[verify check.configuration]
#[tokio::test]
async fn run_with_a_refused_configuration_holds_what_cargo_deny_wrote() {
    let project = Project::with_configuration(BROKEN);

    let outcome = project.run().await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("unexpected keys"),
        "expected the diagnosis of cargo-deny, got {source}"
    );
}

// checkdependencies[verify check.configuration]
// checkdependencies[verify check.incomplete]
#[tokio::test]
async fn run_with_a_refused_configuration_stops() {
    let project = Project::with_configuration(BROKEN);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// checkdependencies[verify check.location]
#[tokio::test]
async fn run_with_a_refused_license_at_the_project_root_names_the_project() {
    let project = Project::with_configuration(ALLOW_APACHE);

    let outcome = project.run().await;

    assert!(
        findings(&outcome)
            .iter()
            .all(|finding| finding.location() == &Location::Project),
        "expected the project, got {:?}",
        findings(&outcome)
            .iter()
            .map(Finding::location)
            .collect::<Vec<_>>()
    );
}

// checkdependencies[verify check.location]
#[tokio::test]
async fn run_with_a_refused_license_below_the_project_root_names_the_workspace() {
    let project = two_workspaces(ALLOW_MIT);

    let outcome = project.run().await;

    assert_eq!(
        findings(&outcome)
            .iter()
            .map(Finding::location)
            .collect::<Vec<_>>(),
        [&Location::Directory {
            path: DirectoryPath::try_from(TOOLS).expect("the test names a relative path"),
        }]
    );
}

// checkdependencies[verify check.failed]
#[tokio::test]
async fn run_with_a_refused_license_fails() {
    let project = Project::with_configuration(ALLOW_APACHE);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// checkdependencies[verify check.finding]
// checkdependencies[verify run.structured]
#[tokio::test]
async fn run_with_a_refused_license_names_the_check_the_words_and_the_package() {
    let project = Project::with_configuration(ALLOW_APACHE);

    let outcome = project.run().await;

    assert!(
        messages(findings(&outcome))
            .contains(&"[rejected] failed to satisfy license requirements (otter 0.1.0)"),
        "expected the message of cargo-deny, got {:?}",
        messages(findings(&outcome))
    );
}

// checkdependencies[verify check.passed]
// checkdependencies[verify check.warning]
#[tokio::test]
async fn run_with_a_warning_counts_it() {
    let project = Project::with_configuration(UNUSED);

    let outcome = project.run().await;

    assert_eq!(summary(&outcome), "checked 1 workspace, 1 warning");
}

// checkdependencies[verify check.warning]
#[tokio::test]
async fn run_with_a_warning_passes() {
    let project = Project::with_configuration(UNUSED);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the warning to leave the run passing, got {outcome:?}"
    );
}

// checkdependencies[verify tool.missing]
#[tokio::test]
async fn run_without_a_cargo_deny_stops() {
    let project = Project::without_deny();
    project.write("Cargo.toml", PACKAGE);
    project.write("src/lib.rs", LIBRARY);
    project.write("deny.toml", ALLOW_MIT);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}
