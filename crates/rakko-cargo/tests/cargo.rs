//! Tests that drive the crate against real projects
//!
//! Each test builds a project in a temporary directory, so no fixture with a
//! deliberately broken manifest sits in this repository, where cargo would
//! stumble over it.
//!
//! The tests run the cargo that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the toolchain that
//! answers is a toolchain that the repository installs, and a new pin
//! reaches the tests without a change to them.

// An assertion in a test panics by design, and the helpers of this file
// exist only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::process::Command;

use rakko::{Context, ProjectRoot};
use rakko_cargo::{
    Cargo, CargoPackage, CargoReport, CargoRoot, Channel, DiscoverRootsError, Documentation,
    DocumentedNames, PackageName, ResolveNewestToolchainError, ResolveToolchainError, RustVersion,
    Toolchain, lockfile,
};
use rakko_tool::{Invocation, ResolveRequirementError, Tool};
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

/// The manifest of a package that belongs to no workspace and excludes the
/// fixtures below it
///
/// The fixtures are not part of the workspace of the package, and a path that
/// the manifest excludes is relative to the directory of the manifest.
const STANDALONE_EXCLUDING: &str = "[workspace]\nexclude = [\"fixtures\"]\n\n[package]\nname = \"harness\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// The manifest of a workspace whose only member sits in a directory that the
/// workspace excludes
///
/// Cargo lets an explicit member win over the exclusion, for the member and
/// for every manifest below it.
const EXCLUDING_A_MEMBER: &str =
    "[workspace]\nmembers = [\"fixtures/real\"]\nexclude = [\"fixtures\"]\nresolver = \"3\"\n";

/// The manifest of the member of that workspace
const REAL: &str = "[package]\nname = \"real\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// A manifest that cargo cannot read
const BROKEN: &str = "this is not a manifest\n";

/// The marker of a project, which Rakko never reads
const MARKER: &str = ".config/rakko.toml";

/// The manifest of a package whose library builds a C library
///
/// Rustdoc links no example against such a library, and cargo says so when a
/// run asks for the documentation examples of the package.
const FOREIGN_LIBRARY: &str = "[workspace]\n\n[package]\nname = \"native\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n";

/// The manifest of a workspace of a library and a command line tool
const SHARED_NAME: &str = "[workspace]\nmembers = [\"demo\", \"demo-cli\"]\nresolver = \"3\"\n";

/// The manifest of the library of that workspace
const DEMO: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

/// The manifest of a package whose binary carries the name of the library
/// of another package
const DEMO_CLI: &str = "[package]\nname = \"demo-cli\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"demo\"\npath = \"src/main.rs\"\n";

/// The manifest of a package whose binary carries the name of the library
/// of another package and stays out of the documentation
const UNDOCUMENTED_CLI: &str = "[package]\nname = \"demo-cli\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"demo\"\npath = \"src/main.rs\"\ndoc = false\n";

/// The manifest of a workspace of a library whose name holds a hyphen and a
/// command line tool
const HYPHENATED: &str = "[workspace]\nmembers = [\"demo-core\", \"tool\"]\nresolver = \"3\"\n";

/// The manifest of a package whose binary carries the name of the library
/// of another package, with a hyphen where the crate name has an underscore
const TOOL: &str = "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"demo-core\"\npath = \"src/main.rs\"\n";

/// The manifest of a workspace whose members sit in directories that do not
/// follow the order of their names
const UNSORTED: &str = "[workspace]\nmembers = [\"first\", \"second\"]\nresolver = \"3\"\n";

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

    /// Creates a project with a library and a package whose binary carries
    /// the name of the library, and the given manifest for that package
    fn sharing_a_name(manifest: &str) -> Self {
        let project = Self::new();
        project.write("Cargo.toml", SHARED_NAME);
        project.write("demo/Cargo.toml", DEMO);
        project.write("demo/src/lib.rs", "");
        project.write("demo-cli/Cargo.toml", manifest);
        project.write("demo-cli/src/main.rs", "fn main() {}\n");

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

    /// Creates a project with a workspace of two members that excludes the
    /// given path
    fn workspace_excluding(path: &str) -> Self {
        let project = Self::new();
        project.write(
            "Cargo.toml",
            &format!(
                "[workspace]\nmembers = [\"a\", \"b\"]\nexclude = [\"{path}\"]\nresolver = \"3\"\n"
            ),
        );
        project.package("a");
        project.package("b");

        project
    }

    /// Returns the path of a directory of the project, as a root whose
    /// packages hold a library
    fn cargo_root(&self, path: &str) -> CargoRoot {
        CargoRoot::builder()
            .directory(self.root().get().join(path))
            .documentation(Documentation::Testable)
            .build()
    }

    /// Returns the path of a directory of the project, as the root of the
    /// standalone package whose binary is the harness
    fn binary_root(&self, path: &str) -> CargoRoot {
        CargoRoot::builder()
            .directory(self.root().get().join(path))
            .documentation(Documentation::Untestable)
            .packages(vec![package("harness", DocumentedNames::Unique)])
            .build()
    }

    /// Returns the root of the workspace of two members that
    /// [`workspace`][Project::workspace] writes
    fn workspace_root(&self) -> CargoRoot {
        CargoRoot::builder()
            .directory(self.root().get().to_path_buf())
            .documentation(Documentation::Testable)
            .packages(vec![
                package("a", DocumentedNames::Unique),
                package("b", DocumentedNames::Unique),
            ])
            .build()
    }

    /// Writes a package that declares the Rust version it compiles on
    fn declaring(&self, name: &str, version: &str) {
        self.write(
            &format!("{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nrust-version = \"{version}\"\n"
            ),
        );
        self.write(&format!("{name}/src/lib.rs"), "");
    }

    /// Returns the directory above the project, which holds the outer
    /// workspace of a project that sits inside one
    fn outer(&self) -> PathBuf {
        ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists")
            .get()
            .to_path_buf()
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

    /// Returns the context of a run in this project
    fn context(&self) -> Context {
        Context::builder().root(self.root()).build()
    }

    /// Resolves the cargo of this project
    async fn resolve(&self) -> Cargo {
        Cargo::resolve(&self.context())
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
        ProjectRoot::canonical(&self.root).expect("the test names a directory that exists")
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

/// Returns a package of a root with the given name
fn package(name: &str, documented_names: DocumentedNames) -> CargoPackage {
    CargoPackage::new(PackageName::new(name), documented_names)
}

/// Returns the Rust toolchain that this repository builds with
///
/// The crate itself makes the choice, so a test that needs a toolchain which
/// every machine running the tests has installed names the same one that a
/// job of the repository would. Asking the crate keeps the version out of
/// the tests, and the rule that picks it in one place with its own tests.
async fn pinned_rust() -> String {
    Toolchain::newest(&ProjectRoot::new(repository().to_path_buf()))
        .await
        .expect("the repository pins a Rust toolchain by a version")
        .get()
        .to_owned()
}

/// Returns the Rust toolchain that mise selects for this repository
///
/// Mise names the toolchain of the default pin in `RUSTUP_TOOLCHAIN`, and it
/// reports that through another interface than the list of pins that
/// [`pinned_rust`] reads. The test below holds the two answers against each
/// other, so a pin that moves which toolchain the helper picks fails a test
/// instead of sending every test that needs a cargo to another toolchain.
fn selected_rust() -> String {
    let output = Command::new("mise")
        .args(["env", "--json"])
        .current_dir(repository())
        .output()
        .expect("the test starts mise to read the environment of the project");
    let environment: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the test reads the report of mise");

    environment["RUSTUP_TOOLCHAIN"]
        .as_str()
        .expect("mise selects a Rust toolchain for the repository")
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

/// Returns the value of a variable that a command sets
fn variable(invocation: &Invocation, name: &str) -> Option<String> {
    invocation
        .environment()
        .iter()
        .find(|variable| variable.name().get() == name)
        .map(|variable| variable.value().get().to_string_lossy().into_owned())
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

#[tokio::test]
async fn invocation_names_the_stable_toolchain_first() {
    let project = Project::workspace();
    let cargo = project.resolve().await;

    let invocation = cargo.invocation(&project.cargo_root("")).arg("check");

    let arguments: Vec<String> = invocation
        .arguments()
        .iter()
        .map(|argument| argument.get().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        arguments,
        [format!("+{}", cargo.toolchain().get()), "check".to_owned()]
    );
}

// The pins of the project name two toolchains that no machine installs, so
// the choice among them surfaces as the toolchain that the failure names.
// That keeps the test free of whichever versions this repository installs.
#[tokio::test]
async fn newest_in_a_project_that_pins_several_versions_names_the_highest() {
    let project = Project::with_rust("[\"1.71.0\", \"1.70.0\"]");

    let toolchain = Toolchain::newest(&project.root()).await;

    assert!(
        matches!(
            &toolchain,
            Err(ResolveNewestToolchainError::UninstalledToolchain { toolchain })
                if toolchain.get() == "1.71.0"
        ),
        "expected the highest pin, got {toolchain:?}"
    );
}

#[tokio::test]
async fn newest_in_this_repository_names_the_toolchain_it_builds_with() {
    let selected = selected_rust();

    let toolchain = Toolchain::newest(&ProjectRoot::new(repository().to_path_buf())).await;

    assert_eq!(toolchain.ok(), Some(Toolchain::new(selected)));
}

#[tokio::test]
async fn plugin_invocation_selects_the_stable_toolchain_for_the_builds_of_the_plugin() {
    let project = Project::workspace();
    let cargo = project.resolve().await;
    let plugin = Tool::resolve_requirement(&Cargo::requirement(), &project.context())
        .await
        .expect("the test resolves a program to stand in for a plugin");

    let invocation = cargo.plugin_invocation(&project.cargo_root(""), &plugin);

    assert_eq!(
        variable(&invocation, "RUSTUP_TOOLCHAIN"),
        Some(cargo.toolchain().get().to_owned())
    );
}

#[tokio::test]
async fn plugin_invocation_tells_the_plugin_to_start_the_cargo_of_the_project() {
    let project = Project::workspace();
    let cargo = project.resolve().await;
    let plugin = Tool::resolve_requirement(&Cargo::requirement(), &project.context())
        .await
        .expect("the test resolves a program to stand in for a plugin");

    let invocation = cargo.plugin_invocation(&project.cargo_root(""), &plugin);

    assert_eq!(
        variable(&invocation, "CARGO"),
        Some(
            cargo
                .invocation(&project.cargo_root(""))
                .program()
                .get()
                .to_string_lossy()
                .into_owned()
        )
    );
}

#[tokio::test]
async fn plugin_invocation_with_toolchain_selects_the_toolchain_for_the_builds_of_the_plugin() {
    let project = Project::workspace();
    let cargo = project.resolve().await;
    let plugin = Tool::resolve_requirement(&Cargo::requirement(), &project.context())
        .await
        .expect("the test resolves a program to stand in for a plugin");
    let toolchain = Toolchain::new("nightly-2026-08-11");

    let invocation =
        cargo.plugin_invocation_with_toolchain(&project.cargo_root(""), &plugin, &toolchain);

    assert_eq!(
        variable(&invocation, "RUSTUP_TOOLCHAIN"),
        Some("nightly-2026-08-11".to_owned())
    );
}

#[tokio::test]
async fn resolve_in_this_repository_selects_the_toolchain_that_mise_selects() {
    let selected = selected_rust();
    let context = Context::builder().root(repository()).build();

    let cargo = Cargo::resolve(&context).await;

    assert_eq!(
        cargo.ok().map(|cargo| cargo.toolchain().clone()),
        Some(Toolchain::new(selected))
    );
}

#[tokio::test]
async fn resolve_with_a_pin_of_the_stable_channel_says_how_to_repair_it() {
    let project = Project::with_rust("\"stable\"");

    let Err(error) = Cargo::resolve(&project.context()).await else {
        panic!("expected the resolution to fail");
    };

    assert!(
        error
            .to_string()
            .contains("add a pin with a version number"),
        "expected a hint for the repair, got {error}"
    );
}

#[tokio::test]
async fn resolve_with_a_stable_pin_outside_the_range_reports_the_pin() {
    let project = Project::with_rust("\"1.70.0\"");

    let cargo = Cargo::resolve(&project.context()).await;

    assert!(
        matches!(
            &cargo,
            Err(ResolveRequirementError::VersionOutOfRange { version, .. })
                if version.get() == "1.70.0"
        ),
        "expected a pin outside the range, got {cargo:?}"
    );
}

#[tokio::test]
async fn resolve_with_a_stable_pin_that_nothing_installed_reports_the_toolchain() {
    let project = Project::with_rust("\"99.0.0\"");

    let cargo = Cargo::resolve(&project.context()).await;

    assert!(
        matches!(
            &cargo,
            Err(ResolveRequirementError::UnresolvedVersion { version, .. })
                if version.get() == "99.0.0"
        ),
        "expected a toolchain that nothing installed, got {cargo:?}"
    );
}

#[tokio::test]
async fn roots_of_a_workspace_of_binaries_say_that_nothing_documents_an_example() {
    let project = Project::new();
    project.write("Cargo.toml", STANDALONE);
    project.write("src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(CargoRoot::documentation),
        Some(Documentation::Untestable)
    );
}

#[tokio::test]
async fn roots_of_a_workspace_with_a_library_say_that_its_examples_are_testable() {
    let project = Project::workspace();

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(CargoRoot::documentation),
        Some(Documentation::Testable)
    );
}

#[tokio::test]
async fn roots_of_a_workspace_name_its_packages() {
    let project = Project::workspace();

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(|root| root.packages().clone()),
        Some(vec![
            package("a", DocumentedNames::Unique),
            package("b", DocumentedNames::Unique)
        ])
    );
}

#[tokio::test]
async fn roots_of_a_workspace_name_its_packages_in_the_order_of_their_names() {
    let project = Project::new();
    project.write("Cargo.toml", UNSORTED);
    project.write(
        "first/Cargo.toml",
        "[package]\nname = \"zeta\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    project.write("first/src/lib.rs", "");
    project.write(
        "second/Cargo.toml",
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    project.write("second/src/lib.rs", "");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(|root| root.packages().clone()),
        Some(vec![
            package("alpha", DocumentedNames::Unique),
            package("zeta", DocumentedNames::Unique)
        ])
    );
}

#[tokio::test]
async fn roots_of_a_workspace_whose_binary_shares_the_name_of_a_library_say_both_share_it() {
    let project = Project::sharing_a_name(DEMO_CLI);

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(|root| root.packages().clone()),
        Some(vec![
            package("demo", DocumentedNames::Shared),
            package("demo-cli", DocumentedNames::Shared)
        ])
    );
}

#[tokio::test]
async fn roots_of_a_workspace_whose_binary_shares_the_name_of_its_own_library_say_none_shares_it() {
    let project = Project::workspace();
    project.write("a/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(|root| root.packages().clone()),
        Some(vec![
            package("a", DocumentedNames::Unique),
            package("b", DocumentedNames::Unique)
        ])
    );
}

#[tokio::test]
async fn roots_of_a_workspace_whose_hyphenated_binary_shares_a_name_say_both_share_it() {
    let project = Project::new();
    project.write("Cargo.toml", HYPHENATED);
    project.package("demo-core");
    project.write("tool/Cargo.toml", TOOL);
    project.write("tool/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(|root| root.packages().clone()),
        Some(vec![
            package("demo-core", DocumentedNames::Shared),
            package("tool", DocumentedNames::Shared)
        ])
    );
}

#[tokio::test]
async fn roots_of_a_workspace_whose_undocumented_binary_shares_a_name_say_none_shares_it() {
    let project = Project::sharing_a_name(UNDOCUMENTED_CLI);

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(|root| root.packages().clone()),
        Some(vec![
            package("demo", DocumentedNames::Unique),
            package("demo-cli", DocumentedNames::Unique)
        ])
    );
}

#[tokio::test]
async fn roots_of_a_workspace_that_declares_a_version_answer_the_highest() {
    let project = Project::new();
    project.write("Cargo.toml", WORKSPACE);
    project.declaring("a", "1.85.0");
    project.declaring("b", "1.88.0");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots
            .first()
            .and_then(|root| root.rust_version().as_ref().map(RustVersion::get)),
        Some("1.88.0")
    );
}

#[tokio::test]
async fn roots_of_a_workspace_without_a_declaration_answer_no_version() {
    let project = Project::workspace();

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().and_then(|root| root.rust_version().clone()),
        None
    );
}

#[tokio::test]
async fn roots_with_a_library_of_another_form_say_that_nothing_documents_an_example() {
    let project = Project::new();
    project.write("Cargo.toml", FOREIGN_LIBRARY);
    project.write("src/lib.rs", "");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots.first().map(CargoRoot::documentation),
        Some(Documentation::Untestable)
    );
}

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

#[tokio::test]
async fn lockfile_refused_by_a_locked_check_of_a_stale_workspace_answers_yes() {
    let project = Project::workspace();
    let cargo = project.resolve().await;
    let root = project.cargo_root("");
    cargo
        .invocation(&root)
        .arg("generate-lockfile")
        .run()
        .await
        .expect("the test writes the lockfile of the workspace");
    project.write(
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\na = { path = \"../a\" }\n",
    );

    let execution = cargo
        .invocation(&root)
        .args(["check", lockfile::LOCKED])
        .run()
        .await
        .expect("the test runs cargo");

    assert!(
        lockfile::refused(&execution.stderr().to_string_lossy()),
        "expected a refusal, got {execution:?}"
    );
}

#[tokio::test]
async fn lockfile_refused_by_a_locked_check_without_a_lockfile_answers_yes() {
    let project = Project::workspace();
    let cargo = project.resolve().await;

    let execution = cargo
        .invocation(&project.cargo_root(""))
        .args(["check", lockfile::LOCKED])
        .run()
        .await
        .expect("the test runs cargo");

    assert!(
        lockfile::refused(&execution.stderr().to_string_lossy()),
        "expected a refusal, got {execution:?}"
    );
}

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
            report.diagnostics().first().map(|diagnostic| diagnostic
                .finding(&project.cargo_root(""), &project.root())
                .message()
                .get()
                .ends_with("[unused_variables]"))
        ),
        (Some(true), Some(true))
    );
}

#[tokio::test]
async fn resolve_in_a_project_that_pins_rust_finds_the_program() {
    let project = Project::new();

    let cargo = Cargo::resolve(&project.context()).await;

    assert!(cargo.is_ok(), "expected a cargo, got {cargo:?}");
}

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

    assert_eq!(roots, [project.workspace_root()]);
}

#[tokio::test]
async fn roots_find_a_manifest_in_a_hidden_directory() {
    let project = Project::workspace();
    project.write(".tools/harness/Cargo.toml", STANDALONE);
    project.write(".tools/harness/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots,
        [
            project.workspace_root(),
            project.binary_root(".tools/harness")
        ]
    );
}

#[tokio::test]
async fn roots_ignore_a_manifest_under_the_git_directory() {
    let project = Project::workspace();
    project.write(".git/Cargo.toml", BROKEN);

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.workspace_root()]);
}

#[tokio::test]
async fn roots_ignore_a_manifest_under_the_node_modules_directory() {
    let project = Project::workspace();
    project.write("node_modules/@otter/clipboard/Cargo.toml", STANDALONE);
    project.write("node_modules/@otter/clipboard/src/lib.rs", "");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.workspace_root()]);
}

#[tokio::test]
async fn roots_ignore_a_manifest_under_the_target_directory() {
    let project = Project::workspace();
    project.write("target/debug/build/dep/Cargo.toml", BROKEN);

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.workspace_root()]);
}

#[tokio::test]
async fn roots_ignore_a_manifest_that_another_root_excludes() {
    let project = Project::workspace();
    project.write("tools/harness/Cargo.toml", STANDALONE_EXCLUDING);
    project.write("tools/harness/src/main.rs", "fn main() {}\n");
    project.write("tools/harness/fixtures/Cargo.toml", BROKEN);

    let roots = project.roots().await;

    assert!(
        matches!(
            &roots,
            Ok(found) if found == &[project.workspace_root(), project.binary_root("tools/harness")]
        ),
        "expected the workspace and the harness, got {roots:?}"
    );
}

#[tokio::test]
async fn roots_ignore_a_manifest_that_the_workspace_excludes() {
    let project = Project::workspace_excluding("fixtures");
    project.write("fixtures/broken/Cargo.toml", BROKEN);
    project.write("fixtures/own/Cargo.toml", STANDALONE);
    project.write("fixtures/own/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await;

    assert!(
        matches!(&roots, Ok(found) if found == &[project.workspace_root()]),
        "expected only the workspace, got {roots:?}"
    );
}

#[tokio::test]
async fn roots_of_a_workspace_that_excludes_a_member_name_a_workspace_below_the_member() {
    let project = Project::new();
    project.write("Cargo.toml", EXCLUDING_A_MEMBER);
    project.write("fixtures/real/Cargo.toml", REAL);
    project.write("fixtures/real/src/lib.rs", "");
    project.write("fixtures/real/own/Cargo.toml", STANDALONE);
    project.write("fixtures/real/own/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots,
        [
            CargoRoot::builder()
                .directory(project.root().get().to_path_buf())
                .documentation(Documentation::Testable)
                .packages(vec![package("real", DocumentedNames::Unique)])
                .build(),
            project.binary_root("fixtures/real/own")
        ]
    );
}

#[tokio::test]
async fn roots_of_a_workspace_that_excludes_a_pattern_name_the_package_it_matches() {
    let project = Project::workspace_excluding("examples/*");
    project.write("examples/sample/Cargo.toml", STANDALONE);
    project.write("examples/sample/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots,
        [
            project.workspace_root(),
            project.binary_root("examples/sample")
        ]
    );
}

#[tokio::test]
async fn roots_of_a_workspace_that_excludes_a_prefix_name_the_package_it_shares_letters_with() {
    let project = Project::workspace_excluding("fix");
    project.write("fixtures/Cargo.toml", STANDALONE);
    project.write("fixtures/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots,
        [project.workspace_root(), project.binary_root("fixtures")]
    );
}

#[tokio::test]
async fn roots_ignore_a_nested_project() {
    let project = Project::workspace();
    project.write(&format!(".claude/worktrees/feature/{MARKER}"), "");
    project.write(".claude/worktrees/feature/Cargo.toml", BROKEN);

    let roots = project.roots().await;

    assert!(
        matches!(&roots, Ok(found) if found == &[project.workspace_root()]),
        "expected only the workspace, got {roots:?}"
    );
}

// A marker that is a directory cannot be read as a file, so the test fails
// when the discovery decides by reading the marker instead of by its presence.
#[tokio::test]
async fn roots_ignore_a_nested_project_whose_marker_is_a_directory() {
    let project = Project::workspace();
    std::fs::create_dir_all(project.root.join("nested").join(MARKER))
        .expect("the test creates the marker as a directory");
    project.write("nested/Cargo.toml", BROKEN);

    let roots = project.roots().await;

    assert!(
        matches!(&roots, Ok(found) if found == &[project.workspace_root()]),
        "expected only the workspace, got {roots:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn roots_ignore_an_unreadable_directory_of_a_nested_project() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::workspace();
    project.write(&format!("nested/{MARKER}"), "");
    project.write("nested/closed/README.md", "# Project\n");
    let closed = project.root.join("nested/closed");
    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a directory");

    let roots = project.roots().await;

    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o755))
        .expect("the test restores the permissions of a directory");
    assert!(
        matches!(&roots, Ok(found) if found == &[project.workspace_root()]),
        "expected only the workspace, got {roots:?}"
    );
}

#[tokio::test]
async fn roots_of_a_project_that_holds_the_marker_name_its_workspace() {
    let project = Project::workspace();
    project.write(MARKER, "");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.workspace_root()]);
}

#[tokio::test]
async fn roots_of_a_project_with_a_standalone_package_name_both() {
    let project = Project::workspace();
    project.write("tools/harness/Cargo.toml", STANDALONE);
    project.write("tools/harness/src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(
        roots,
        [
            project.workspace_root(),
            project.binary_root("tools/harness")
        ]
    );
}

#[tokio::test]
async fn roots_of_a_project_without_a_workspace_name_the_package() {
    let project = Project::new();
    project.write("Cargo.toml", STANDALONE);
    project.write("src/main.rs", "fn main() {}\n");

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.binary_root("")]);
}

#[tokio::test]
async fn roots_of_a_workspace_name_no_member() {
    let project = Project::workspace();

    let roots = project.roots().await.expect("the test discovers the roots");

    assert!(
        !roots
            .iter()
            .any(|root| root.directory() == &project.root().get().join("a")),
        "expected no member among the roots, got {roots:?}"
    );
}

#[tokio::test]
async fn roots_of_a_workspace_name_the_workspace_once() {
    let project = Project::workspace();

    let roots = project.roots().await.expect("the test discovers the roots");

    assert_eq!(roots, [project.workspace_root()]);
}

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

#[tokio::test]
async fn toolchain_resolve_a_pin_that_nothing_installed_names_the_toolchain() {
    let pin = pinned_rust().await;
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

#[tokio::test]
async fn toolchain_resolve_a_pinned_channel_answers_the_toolchain() {
    let pin = pinned_rust().await;
    let project = Project::with_rust(&format!("\"{pin}\""));

    let toolchain = Toolchain::resolve(Channel::new(pin.clone()), &project.root()).await;

    assert_eq!(toolchain.ok(), Some(Toolchain::new(pin)));
}

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

#[tokio::test]
async fn toolchain_resolve_requirement_of_a_release_answers_the_pin_of_that_release() {
    let pin = pinned_rust().await;
    let project = Project::with_rust(&format!("\"{pin}\""));
    project.declaring("declared", &pin);
    let roots = project.roots().await.expect("the test discovers the roots");
    let version = roots
        .first()
        .and_then(|root| root.rust_version().clone())
        .expect("the package declares a version");

    let toolchain =
        Toolchain::resolve_requirement(&Toolchain::release(&version), &project.context()).await;

    assert_eq!(toolchain.ok(), Some(Toolchain::new(pin)));
}

#[tokio::test]
async fn toolchain_resolve_requirement_of_the_nightly_answers_the_dated_pin() {
    let context = Context::builder().root(repository()).build();

    let toolchain = Toolchain::resolve_requirement(&Toolchain::nightly(), &context).await;

    assert!(
        toolchain
            .as_ref()
            .is_ok_and(|toolchain| toolchain.get().starts_with("nightly-")),
        "expected the dated nightly of the repository, got {toolchain:?}"
    );
}

// A machine that holds a dated nightly resolves a bare `nightly` to it, so
// the pin would count on that machine and on no other.
#[tokio::test]
async fn toolchain_resolve_requirement_of_the_nightly_with_a_bare_channel_reports_it() {
    let pin = pinned_rust().await;
    let project = Project::with_rust(&format!("[\"{pin}\", \"nightly\"]"));

    let toolchain = Toolchain::resolve_requirement(&Toolchain::nightly(), &project.context()).await;

    assert!(
        matches!(
            &toolchain,
            Err(ResolveRequirementError::UnresolvedToolchain { .. })
        ),
        "expected the bare channel to count for nothing, got {toolchain:?}"
    );
}

#[tokio::test]
async fn toolchain_resolve_requirement_of_the_nightly_without_a_dated_pin_reports_it() {
    let pin = pinned_rust().await;
    let project = Project::with_rust(&format!("\"{pin}\""));

    let toolchain = Toolchain::resolve_requirement(&Toolchain::nightly(), &project.context()).await;

    assert!(
        matches!(
            &toolchain,
            Err(ResolveRequirementError::UnresolvedToolchain { .. })
        ),
        "expected no nightly, got {toolchain:?}"
    );
}
