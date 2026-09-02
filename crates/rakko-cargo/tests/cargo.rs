//! Tests that drive the crate against real projects
//!
//! Each test builds a project in a temporary directory, so no fixture with a
//! deliberately broken manifest sits in this repository, where cargo would
//! stumble over it.
//!
//! The tests run the cargo that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers
//! is the version that the repository installs, and a new pin reaches the
//! tests without a change to them.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::process::Command;

use rakko_action::ProjectRoot;
use rakko_cargo::{
    Cargo, CargoReport, CargoRoot, Channel, DiscoverRootsError, ResolveToolchainError, Toolchain,
};
use tempfile::TempDir;

/// The manifest of a workspace with two members
const WORKSPACE: &str = "[workspace]\nmembers = [\"a\", \"b\"]\nresolver = \"3\"\n";

/// The manifest of a package that belongs to no workspace
///
/// The empty workspace table tells cargo that the package is a root of its
/// own, which cargo demands from a package below a workspace that does not
/// list it.
const STANDALONE: &str =
    "[workspace]\n\n[package]\nname = \"harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// A manifest that cargo cannot read
const BROKEN: &str = "this is not a manifest\n";

/// The manifest of a workspace that lists a project below it as a member
const OUTER: &str = "[workspace]\nmembers = [\"project\"]\nresolver = \"3\"\n";

/// The manifest of a package that belongs to the workspace above it
const MEMBER: &str = "[package]\nname = \"project\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,

    /// The root of the project, which is the directory or a directory below it
    root: PathBuf,
}

impl Project {
    /// Creates a project without a cargo to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when it only looks at the project.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let root = directory.path().to_path_buf();

        Self { directory, root }
    }

    /// Creates a project with the cargo of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the cargo
    /// that mise resolves for it is the cargo that the repository pins and
    /// installs. Mise ignores a configuration that nobody trusts, so the
    /// copy is trusted right away.
    fn new() -> Self {
        let project = Self::bare();
        project.pin_repository();

        project
    }

    /// Creates a project below a workspace that lists it as a member
    ///
    /// The workspace sits in the temporary directory, and the project is the
    /// package in its `project` directory, so the root that cargo names for
    /// the manifest of the project lies above the project.
    fn inside_a_workspace() -> Self {
        let mut project = Self::bare();
        std::fs::write(project.directory.path().join("Cargo.toml"), OUTER)
            .expect("the test writes the manifest of the outer workspace");
        project.root = project.directory.path().join("project");
        project.write("Cargo.toml", MEMBER);
        project.write("src/lib.rs", "");
        project.pin_repository();

        project
    }

    /// Creates a project that pins the given Rust toolchains
    fn with_rust(pins: &str) -> Self {
        let project = Self::bare();

        let configuration = project.root.join("mise.toml");
        std::fs::write(&configuration, format!("[tools]\nrust = {pins}\n"))
            .expect("the test writes the mise.toml of the project");
        trust(&configuration);

        project
    }

    /// Creates a project with a workspace of two members
    fn workspace() -> Self {
        let project = Self::new();
        project.write("Cargo.toml", WORKSPACE);
        project.package("a");
        project.package("b");

        project
    }

    /// Returns whether cargo has anything to do in this project
    async fn applies(&self) -> bool {
        Cargo::applies(&self.root()).await
    }

    /// Returns the path of a directory of the project, as a root
    fn cargo_root(&self, path: &str) -> CargoRoot {
        CargoRoot::new(self.root().get().join(path))
    }

    /// Returns the directory above the project, which holds the outer
    /// workspace of a project that sits inside one
    fn outer(&self) -> PathBuf {
        self.directory
            .path()
            .canonicalize()
            .expect("the test names a directory that exists")
    }

    /// Writes a package with the given name into the directory of that name
    fn package(&self, name: &str) {
        self.write(
            &format!("{name}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        );
        self.write(&format!("{name}/src/lib.rs"), "");
    }

    /// Copies the mise configuration of this repository into the project
    ///
    /// The cargo that mise resolves for the project is then the cargo that
    /// the repository pins and installs. Mise ignores a configuration that
    /// nobody trusts, so the copy is trusted right away.
    fn pin_repository(&self) {
        let pins = repository().join("mise.toml");
        let copy = self.root.join("mise.toml");
        std::fs::create_dir_all(&self.root).expect("the test creates the root of the project");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);
    }

    /// Resolves the cargo of this project
    async fn resolve(&self) -> Cargo {
        Cargo::resolve(self.root())
            .await
            .expect("the test resolves the cargo that the repository pins")
    }

    /// Returns the roots of this project
    async fn roots(&self) -> Result<Vec<CargoRoot>, DiscoverRootsError> {
        self.resolve().await.roots().await
    }

    /// Returns the root of this project
    ///
    /// The root is canonical, so the paths that a run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn root(&self) -> ProjectRoot {
        let path = self
            .root
            .canonicalize()
            .expect("the test names a directory that exists");

        ProjectRoot::new(path)
    }

    /// Writes a file of the project, with the directories that lead to it
    fn write(&self, path: &str, content: &str) {
        let path = self.root.join(path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the test creates the directories of a file");
        }

        std::fs::write(&path, content).expect("the test writes a file of the project");
    }
}

/// Returns the Rust toolchain that this repository pins, as mise reports it
///
/// The tests resolve a channel that the copied configuration pins, and the
/// pin of the repository is the one channel that every machine which runs
/// the tests has installed. Reading the pin from mise keeps the version out
/// of the tests.
fn pinned_rust() -> String {
    let output = Command::new("mise")
        .args(["ls", "--current", "--json", "rust"])
        .current_dir(repository())
        .output()
        .expect("the test starts mise to list the toolchains");
    let pins: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the test reads the report of mise");

    pins[0]["requested_version"]
        .as_str()
        .expect("the repository pins a Rust toolchain")
        .to_owned()
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
            .arg(self.root.join("mise.toml"))
            .status();
    }
}

// cargo[verify look.manifest]
#[tokio::test]
async fn applies_with_a_manifest_in_a_hidden_directory_is_true() {
    let project = Project::bare();
    project.write(".tools/harness/Cargo.toml", STANDALONE);

    let applies = project.applies().await;

    assert!(applies);
}

// cargo[verify look.links]
#[cfg(unix)]
#[tokio::test]
async fn applies_with_a_manifest_only_behind_a_symbolic_link_is_false() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("Cargo.toml"), STANDALONE)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let applies = project.applies().await;

    assert!(!applies);
}

// cargo[verify look.git]
#[tokio::test]
async fn applies_with_a_manifest_only_under_the_git_directory_is_false() {
    let project = Project::bare();
    project.write(".git/Cargo.toml", STANDALONE);

    let applies = project.applies().await;

    assert!(!applies);
}

// cargo[verify look.target]
#[tokio::test]
async fn applies_with_a_manifest_only_under_the_target_directory_is_false() {
    let project = Project::bare();
    project.write("target/debug/build/dep/Cargo.toml", STANDALONE);

    let applies = project.applies().await;

    assert!(!applies);
}

// cargo[verify look.unreadable]
#[cfg(unix)]
#[tokio::test]
async fn applies_with_an_unreadable_directory_is_true() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::bare();
    project.write("closed/README.md", "# Project\n");
    let closed = project.directory.path().join("closed");
    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a directory");

    let applies = project.applies().await;

    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o755))
        .expect("the test restores the permissions of a directory");
    assert!(applies);
}

// cargo[verify look.manifest]
#[tokio::test]
async fn applies_without_a_manifest_is_false() {
    let project = Project::bare();
    project.write("README.md", "# Project\n");

    let applies = project.applies().await;

    assert!(!applies);
}

// cargo[verify run.directory]
#[tokio::test]
async fn invocation_runs_in_the_directory_of_the_root() {
    let project = Project::workspace();
    let cargo = project.resolve().await;
    let root = project.cargo_root("tools/harness");

    let invocation = cargo.invocation(&root);

    assert_eq!(
        invocation
            .working_directory()
            .map(|directory| directory.get().to_path_buf()),
        Some(root.directory().clone())
    );
}

// cargo[verify tool.resolve]
#[tokio::test]
async fn invocation_starts_the_program_that_mise_reported() {
    let project = Project::workspace();
    let cargo = project.resolve().await;

    let invocation = cargo.invocation(&project.cargo_root(""));

    assert!(
        invocation.program().get().is_file(),
        "expected an installed program, got {:?}",
        invocation.program()
    );
}

// cargo[verify run.toolchain]
#[tokio::test]
async fn invocation_with_toolchain_names_the_toolchain_first() {
    let project = Project::workspace();
    let cargo = project.resolve().await;
    let toolchain = Toolchain::new("nightly-2026-08-11");

    let invocation = cargo
        .invocation_with_toolchain(&project.cargo_root(""), &toolchain)
        .arg("fmt");

    let arguments: Vec<String> = invocation
        .arguments()
        .iter()
        .map(|argument| argument.get().to_string_lossy().into_owned())
        .collect();
    assert_eq!(arguments, ["+nightly-2026-08-11", "fmt"]);
}

// cargo[verify diagnostic.finished]
// cargo[verify diagnostic.read]
#[tokio::test]
async fn report_of_a_check_reads_what_the_pinned_cargo_wrote() {
    let project = Project::workspace();
    project.write("a/src/lib.rs", "pub fn unused() {\n    let value = 1;\n}\n");
    let cargo = project.resolve().await;

    let execution = cargo
        .invocation(&project.cargo_root(""))
        .args(["check", "--message-format=json"])
        .run()
        .await
        .expect("the test runs cargo");
    let report = CargoReport::read(&execution.stdout().to_string_lossy())
        .expect("the test reads what cargo wrote");

    assert_eq!(
        (
            report.finished(),
            report
                .diagnostics()
                .first()
                .and_then(|diagnostic| diagnostic.code().as_deref())
        ),
        (Some(true), Some("unused_variables"))
    );
}

// cargo[verify tool.resolve]
#[tokio::test]
async fn resolve_in_a_project_that_pins_rust_finds_the_program() {
    let project = Project::new();

    let cargo = Cargo::resolve(project.root()).await;

    assert!(cargo.is_ok(), "expected a cargo, got {cargo:?}");
}

// cargo[verify tool.missing]
#[tokio::test]
async fn resolve_without_a_cargo_reports_the_tool() {
    let project = Project::with_rust("\"0.0.1\"");

    let cargo = Cargo::resolve(project.root()).await;

    let Err(error) = cargo else {
        panic!("expected the resolution to fail, got {cargo:?}");
    };
    assert!(
        error.to_string().contains("cargo"),
        "expected the error to name the tool, got {error}"
    );
}

// cargo[verify root.walk]
#[cfg(unix)]
#[tokio::test]
async fn roots_ignore_a_manifest_behind_a_symbolic_link() {
    let project = Project::workspace();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("Cargo.toml"), STANDALONE)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.cargo_root("")]);
}

// cargo[verify root.walk]
#[tokio::test]
async fn roots_ignore_a_manifest_under_the_git_directory() {
    let project = Project::workspace();
    project.write(".git/Cargo.toml", BROKEN);

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.cargo_root("")]);
}

// cargo[verify root.walk]
#[tokio::test]
async fn roots_ignore_a_manifest_under_the_target_directory() {
    let project = Project::workspace();
    project.write("target/debug/build/dep/Cargo.toml", BROKEN);

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.cargo_root("")]);
}

// cargo[verify root.discover]
#[tokio::test]
async fn roots_of_a_project_with_a_standalone_package_name_both() {
    let project = Project::workspace();
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots,
        [project.cargo_root(""), project.cargo_root("tools/harness")]
    );
}

// cargo[verify root.discover]
#[tokio::test]
async fn roots_of_a_project_without_a_workspace_name_the_package() {
    let project = Project::new();
    project.write("Cargo.toml", STANDALONE);
    project.write("src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.cargo_root("")]);
}

// cargo[verify root.member]
#[tokio::test]
async fn roots_of_a_workspace_name_no_member() {
    let project = Project::workspace();

    let roots = project.roots().await.expect("the test discovers the roots");

    assert!(
        !roots.contains(&project.cargo_root("a")),
        "expected no member among the roots, got {roots:?}"
    );
}

// cargo[verify root.discover]
#[tokio::test]
async fn roots_of_a_workspace_name_the_workspace_once() {
    let project = Project::workspace();

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.cargo_root("")]);
}

// cargo[verify root.contained]
#[tokio::test]
async fn roots_with_a_manifest_of_an_outer_workspace_name_the_workspace() {
    let project = Project::inside_a_workspace();

    let roots = project.roots().await;

    assert!(
        matches!(
            &roots,
            Err(DiscoverRootsError::ForeignWorkspace { manifest, workspace })
                if manifest == &project.root().get().join("Cargo.toml")
                    && workspace == &project.outer()
        ),
        "expected the outer workspace, got {roots:?}"
    );
}

// cargo[verify root.manifest]
#[tokio::test]
async fn roots_with_a_manifest_that_cargo_cannot_read_hold_what_cargo_said() {
    let project = Project::new();
    project.write("Cargo.toml", BROKEN);

    let roots = project.roots().await;

    let Err(DiscoverRootsError::UnreadableManifest { details, .. }) = roots else {
        panic!("expected the broken manifest, got {roots:?}");
    };
    assert!(
        details.contains("Cargo.toml:1"),
        "expected the diagnosis of cargo, got {details}"
    );
}

// cargo[verify root.manifest]
#[tokio::test]
async fn roots_with_a_manifest_that_cargo_cannot_read_name_the_manifest() {
    let project = Project::new();
    project.write("Cargo.toml", BROKEN);

    let roots = project.roots().await;

    assert!(
        matches!(
            &roots,
            Err(DiscoverRootsError::UnreadableManifest { manifest, .. })
                if manifest == &project.root().get().join("Cargo.toml")
        ),
        "expected the broken manifest, got {roots:?}"
    );
}

// cargo[verify root.directory]
#[cfg(unix)]
#[tokio::test]
async fn roots_with_an_unreadable_directory_name_the_directory() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::workspace();
    project.write("closed/README.md", "# Project\n");
    let closed = project.directory.path().join("closed");
    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a directory");

    let roots = project.roots().await;

    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o755))
        .expect("the test restores the permissions of a directory");
    assert!(
        matches!(
            &roots,
            Err(DiscoverRootsError::UnreadableDirectory { directory, .. })
                if directory.ends_with("closed")
        ),
        "expected the unreadable directory, got {roots:?}"
    );
}

// cargo[verify toolchain.uninstalled]
#[tokio::test]
async fn toolchain_resolve_a_pin_that_nothing_installed_names_the_toolchain() {
    let pin = pinned_rust();
    let project = Project::with_rust(&format!("[\"{pin}\", \"nightly-2020-01-01\"]"));

    let toolchain = Toolchain::resolve(Channel::new("nightly"), &project.root()).await;

    assert!(
        matches!(
            &toolchain,
            Err(ResolveToolchainError::UninstalledToolchain { toolchain, .. })
                if toolchain.get() == "nightly-2020-01-01"
        ),
        "expected an uninstalled toolchain, got {toolchain:?}"
    );
}

// cargo[verify toolchain.resolve]
#[tokio::test]
async fn toolchain_resolve_a_pinned_channel_answers_the_toolchain() {
    let pin = pinned_rust();
    let project = Project::with_rust(&format!("\"{pin}\""));

    let toolchain = Toolchain::resolve(Channel::new(pin.clone()), &project.root()).await;

    assert_eq!(toolchain.ok(), Some(Toolchain::new(pin)));
}

// cargo[verify toolchain.unpinned]
#[tokio::test]
async fn toolchain_resolve_an_unpinned_channel_names_the_channel() {
    let project = Project::new();

    let toolchain =
        Toolchain::resolve(Channel::new("rakko-pins-no-such-channel"), &project.root()).await;

    assert!(
        matches!(
            &toolchain,
            Err(ResolveToolchainError::UnpinnedToolchain { channel })
                if channel.get() == "rakko-pins-no-such-channel"
        ),
        "expected an unpinned channel, got {toolchain:?}"
    );
}

// cargo[verify toolchain.report]
#[tokio::test]
async fn toolchain_resolve_outside_a_project_reports_what_mise_wrote() {
    let project = Project::bare();
    project.write("mise.toml", "this is not a configuration\n");

    let toolchain = Toolchain::resolve(Channel::new("nightly"), &project.root()).await;

    assert!(
        matches!(
            &toolchain,
            Err(ResolveToolchainError::UnreadableReport { details, .. })
                if details.contains("mise.toml")
        ),
        "expected what mise wrote about mise.toml, got {toolchain:?}"
    );
}
