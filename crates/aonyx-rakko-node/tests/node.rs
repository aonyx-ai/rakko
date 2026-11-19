//! Tests that resolve Node and the programs that run on it in real projects
//!
//! The root of this repository pins a Node, and mise installs it. A project in
//! a temporary directory pins what a test needs, or nothing. A project that
//! must show which Node runs a program pins a Node of its own. That Node is a
//! directory of the project, which mise reads as an installed Node. It writes
//! a word that no real Node writes.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::process::Command;

use aonyx_rakko_node::{Node, ResolveNodeToolError, ResolveNodeToolchainError};
use rakko::__private::{MiseText, ToolPin};
use rakko::{
    Action, Context, Documented, Name, Outcome, ProjectRoot, ToolchainRequirement, Tools,
    VersionRange, action_name,
};
use rakko_tool::{ResolveToolError, ResolveToolchainError};
use tempfile::TempDir;

/// The name that mise knows Node by
const NODE: &str = "node";

/// The tool of the tests, a package for Node that this repository installs
const PRETTIER: &str = "npm:prettier";

/// The program of the tool of the tests
const PRETTIER_PROGRAM: &str = "prettier";

/// The toolchain of the tests, a package for Node that this repository pins
const TYPESCRIPT: &str = "npm:typescript";

/// The program of the toolchain of the tests
const TSC: &str = "tsc";

/// The word that the Node of a project writes in place of a version
#[cfg(unix)]
const PINNED: &str = "pinned";

/// A program that stands in for the Node that a project pins
///
/// The program writes a word that no real Node writes, whatever it is asked.
/// So the output of a program that runs on Node shows which Node ran it.
#[cfg(unix)]
const PINNED_NODE: &str = "#!/bin/sh\necho pinned\n";

/// An action that declares the tools of the mise files that it holds, and
/// that requires the toolchains in the requirements that it holds
#[derive(Documented)]
struct Declaring {
    /// The tools of the mise files that the action declares
    tools: Vec<Tools>,

    /// The requirements of toolchains that the action declares
    requirements: Vec<ToolchainRequirement>,
}

impl Declaring {
    /// Creates an action that declares nothing
    fn nothing() -> Self {
        Self {
            tools: Vec::new(),
            requirements: Vec::new(),
        }
    }

    /// Creates an action that declares the tool of the tests at the version
    /// that this repository installs
    ///
    /// Only the reader of a mise file creates a declaration, and the version
    /// comes from the file that Rakko generates for this repository. So the
    /// reader works at run time, and the text and the pin stay for the rest of
    /// the run of the tests.
    fn prettier() -> Self {
        let version = pin(&generated(), PRETTIER);
        let text: &'static str = format!("[tools]\n\"{PRETTIER}\" = \"{version}\"\n").leak();
        let pins: &'static [ToolPin; 1] = Box::leak(Box::new(MiseText::new(text).pins::<1>()));

        Self {
            tools: vec![rakko::__private::tools(pins)],
            requirements: Vec::new(),
        }
    }

    /// Creates an action that requires Node in the range
    fn requiring(range: &str) -> Self {
        Self {
            tools: Vec::new(),
            requirements: vec![
                ToolchainRequirement::builder()
                    .toolchain(NODE)
                    .versions(VersionRange::new(range))
                    .build(),
            ],
        }
    }
}

impl Action for Declaring {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("declaring")
    }

    fn additional_tools(&self) -> Vec<Tools> {
        self.tools.clone()
    }

    fn toolchains(&self) -> Vec<ToolchainRequirement> {
        self.requirements.clone()
    }

    async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
        Outcome::Passed { summary: None }
    }
}

/// A project in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project that pins nothing
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project whose mise configuration holds the pins
    ///
    /// Mise ignores a configuration that nobody trusts, so the configuration
    /// is trusted right away.
    fn pinning(pins: &str) -> Self {
        let project = Self::bare();
        let configuration = project.directory.path().join("mise.toml");
        std::fs::write(&configuration, format!("[tools]\n{pins}"))
            .expect("the test writes the mise.toml of the project");
        trust(&configuration);

        project
    }

    /// Creates a project that pins a Node of its own, and the pins
    ///
    /// The Node is [`PINNED_NODE`] in a directory of the project.
    #[cfg(unix)]
    fn on_pinned_node(pins: &str) -> Self {
        Self::on_node_in(Path::new("runtime"), pins)
    }

    /// Creates a project that pins a Node of its own in a directory that
    /// cannot be an entry of `PATH`, and the pins
    ///
    /// The name of the directory holds `:`, the separator of the entries of
    /// a path on Unix.
    #[cfg(unix)]
    fn on_unplaceable_node(pins: &str) -> Self {
        Self::on_node_in(Path::new("run:time"), pins)
    }

    /// Creates a project that pins [`PINNED_NODE`] in a directory of the
    /// project, and the pins
    #[cfg(unix)]
    fn on_node_in(directory: &Path, pins: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;

        let project = Self::bare();
        let runtime = project.directory.path().join(directory);
        let program = runtime.join("bin").join(NODE);
        std::fs::create_dir_all(runtime.join("bin"))
            .expect("the test creates the directory of the Node");
        std::fs::write(&program, PINNED_NODE).expect("the test writes the Node");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
            .expect("the test makes the Node executable");

        let configuration = project.directory.path().join("mise.toml");
        std::fs::write(
            &configuration,
            format!("[tools]\n{NODE} = \"path:{}\"\n{pins}", runtime.display()),
        )
        .expect("the test writes the mise.toml of the project");
        trust(&configuration);

        project
    }

    /// Returns the context of a run in the project
    ///
    /// The root is canonical, so that mise and the test agree on the path of
    /// the project despite the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        let root = ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists");

        Context::builder().root(root).build()
    }

    /// Returns the canonical directory of the program of the Node that
    /// [`on_pinned_node`][pinned] pins
    ///
    /// [pinned]: Project::on_pinned_node
    #[cfg(unix)]
    fn pinned_node_directory(&self) -> PathBuf {
        std::fs::canonicalize(self.directory.path().join("runtime").join("bin"))
            .expect("the project holds the directory of its Node")
    }
}

impl Drop for Project {
    /// Withdraws the trust that the project received
    ///
    /// Mise records the trust of a configuration in the state of the user. The
    /// temporary directory of a test never comes back, so the record would
    /// pile up there forever. A failure stays quiet, because the test already
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

/// Trusts a mise configuration, so that mise reads its pins
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

/// Returns the root of this repository, two directories above the crate
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate lives two directories below the root of the repository")
        .to_path_buf()
}

/// Returns the context of a run in the root of this repository
fn context_of_the_repository() -> Context {
    Context::builder().root(repository().as_path()).build()
}

/// Returns the path of the mise configuration of this repository
fn configuration() -> PathBuf {
    repository().join("mise.toml")
}

/// Returns the path of the tool pins that Rakko generates for this repository
fn generated() -> PathBuf {
    repository().join(".config/mise/conf.d/rakko.toml")
}

/// Returns the version that a mise configuration pins for a tool on a line
/// of its own, as `node = "26.13.1"`
fn pin(configuration: &Path, tool: &str) -> String {
    std::fs::read_to_string(configuration)
        .expect("the test reads a mise configuration of the repository")
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;

            (key.trim().trim_matches('"') == tool)
                .then(|| value.trim().trim_matches('"').to_owned())
        })
        .expect("the configuration pins the tool on a line of its own")
}

/// Returns the canonical first entry of the path that a command of a program
/// sets, if it sets one
#[cfg(unix)]
fn first_on_the_path(tool: &rakko_tool::Tool) -> Option<PathBuf> {
    let invocation = tool.invocation();
    let path = invocation
        .environment()
        .iter()
        .find(|variable| variable.name().get() == "PATH")
        .map(|variable| variable.value().get().to_owned())?;
    let first = std::env::split_paths(&path).next()?;

    std::fs::canonicalize(first).ok()
}

/// Returns what a program writes when it is asked for its version
#[cfg(unix)]
async fn version_of(tool: &rakko_tool::Tool) -> String {
    let execution = tool
        .invocation()
        .arg("--version")
        .run()
        .await
        .expect("expected the program to run");

    execution.stdout().to_string_lossy().trim().to_owned()
}

#[tokio::test]
async fn resolve_in_a_project_that_pins_no_node_reports_node() {
    let project = Project::bare();

    let node = Node::resolve()
        .action(&Declaring::nothing())
        .context(&project.context())
        .call()
        .await;

    assert!(
        matches!(
            &node,
            Err(ResolveToolchainError::UnpinnedToolchain { toolchain, .. }) if toolchain.get() == NODE
        ),
        "expected the lookup to report the missing pin of Node, got {node:?}"
    );
}

#[tokio::test]
async fn resolve_in_a_project_that_pins_node_finds_the_pinned_version() {
    let node = Node::resolve()
        .action(&Declaring::nothing())
        .context(&context_of_the_repository())
        .call()
        .await;

    assert_eq!(
        node.ok().map(|node| node.version().get().to_owned()),
        Some(pin(&configuration(), NODE))
    );
}

#[tokio::test]
async fn resolve_tool_in_a_missing_directory_reports_the_tool() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();

    let tool = Node::resolve_tool()
        .action(&Declaring::prettier())
        .tool(PRETTIER)
        .executable(PRETTIER_PROGRAM)
        .context(&context)
        .call()
        .await;

    assert!(
        matches!(&tool, Err(ResolveNodeToolError::UnresolvedTool { .. })),
        "expected the lookup to report the tool before Node, got {tool:?}"
    );
}

#[tokio::test]
async fn resolve_tool_in_a_project_that_pins_no_node_reports_node() {
    let project = Project::bare();

    let tool = Node::resolve_tool()
        .action(&Declaring::prettier())
        .tool(PRETTIER)
        .executable(PRETTIER_PROGRAM)
        .context(&project.context())
        .call()
        .await;

    assert!(
        matches!(
            &tool,
            Err(ResolveNodeToolError::UnresolvedNode {
                source: ResolveToolchainError::UnpinnedToolchain { toolchain, .. },
            }) if toolchain.get() == NODE
        ),
        "expected the lookup to report the missing pin of Node, got {tool:?}"
    );
}

#[tokio::test]
async fn resolve_tool_of_an_undeclared_tool_reports_the_tool() {
    let tool = Node::resolve_tool()
        .action(&Declaring::nothing())
        .tool(PRETTIER)
        .executable(PRETTIER_PROGRAM)
        .context(&context_of_the_repository())
        .call()
        .await;

    assert!(
        matches!(
            &tool,
            Err(ResolveNodeToolError::UnresolvedTool {
                source: ResolveToolError::UndeclaredTool { .. },
            })
        ),
        "expected the lookup to report the undeclared tool, got {tool:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn resolve_tool_on_a_node_that_cannot_stand_on_the_path_reports_node() {
    let project = Project::on_unplaceable_node("");

    let tool = Node::resolve_tool()
        .action(&Declaring::prettier())
        .tool(PRETTIER)
        .executable(PRETTIER_PROGRAM)
        .context(&project.context())
        .call()
        .await;

    assert!(
        matches!(
            &tool,
            Err(ResolveNodeToolError::UnresolvedNode {
                source: ResolveToolchainError::UnplaceableToolchain { .. },
            })
        ),
        "expected the lookup to refuse the directory of Node, got {tool:?}"
    );
}

/// Shows that no Node of the inherited path comes before the pinned Node
///
/// The command of the tool puts the directory of the pinned Node first. So
/// whatever Node the inherited path holds, the pinned Node answers first.
#[cfg(unix)]
#[tokio::test]
async fn resolve_tool_puts_the_pinned_node_first_on_the_path() {
    let project = Project::on_pinned_node("");
    let tool = Node::resolve_tool()
        .action(&Declaring::prettier())
        .tool(PRETTIER)
        .executable(PRETTIER_PROGRAM)
        .context(&project.context())
        .call()
        .await
        .expect("the test resolves the tool on the Node of the project");

    let first = first_on_the_path(&tool);

    assert_eq!(first, Some(project.pinned_node_directory()));
}

/// Shows that a tool runs on the Node that the project pins
///
/// The process of the tests inherits a path that can hold another Node
/// first, such as the Node that this repository installs. The tool still
/// writes the word of the pinned Node.
#[cfg(unix)]
#[tokio::test]
async fn resolve_tool_runs_the_tool_on_the_pinned_node() {
    let project = Project::on_pinned_node("");
    let tool = Node::resolve_tool()
        .action(&Declaring::prettier())
        .tool(PRETTIER)
        .executable(PRETTIER_PROGRAM)
        .context(&project.context())
        .call()
        .await
        .expect("the test resolves the tool on the Node of the project");

    let version = version_of(&tool).await;

    assert_eq!(version, PINNED);
}

#[tokio::test]
async fn resolve_toolchain_in_a_project_that_pins_no_node_reports_node() {
    let project = Project::bare();

    let tool = Node::resolve_toolchain()
        .action(&Declaring::nothing())
        .toolchain(TYPESCRIPT)
        .executable(TSC)
        .context(&project.context())
        .call()
        .await;

    assert!(
        matches!(
            &tool,
            Err(ResolveNodeToolchainError::UnresolvedNode {
                source: ResolveToolchainError::UnpinnedToolchain { toolchain, .. },
            }) if toolchain.get() == NODE
        ),
        "expected the lookup to report the missing pin of Node, got {tool:?}"
    );
}

#[tokio::test]
async fn resolve_toolchain_in_a_project_that_pins_no_toolchain_reports_the_toolchain() {
    let project = Project::pinning(&format!("{NODE} = \"{}\"\n", pin(&configuration(), NODE)));

    let tool = Node::resolve_toolchain()
        .action(&Declaring::nothing())
        .toolchain(TYPESCRIPT)
        .executable(TSC)
        .context(&project.context())
        .call()
        .await;

    assert!(
        matches!(
            &tool,
            Err(ResolveNodeToolchainError::UnresolvedToolchain {
                source: ResolveToolchainError::UnpinnedToolchain { toolchain, .. },
            }) if toolchain.get() == TYPESCRIPT
        ),
        "expected the lookup to report the missing pin of the toolchain, got {tool:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn resolve_toolchain_on_a_node_that_cannot_stand_on_the_path_reports_node() {
    let version = pin(&configuration(), TYPESCRIPT);
    let project = Project::on_unplaceable_node(&format!("\"{TYPESCRIPT}\" = \"{version}\"\n"));

    let tool = Node::resolve_toolchain()
        .action(&Declaring::nothing())
        .toolchain(TYPESCRIPT)
        .executable(TSC)
        .context(&project.context())
        .call()
        .await;

    assert!(
        matches!(
            &tool,
            Err(ResolveNodeToolchainError::UnresolvedNode {
                source: ResolveToolchainError::UnplaceableToolchain { .. },
            })
        ),
        "expected the lookup to refuse the directory of Node, got {tool:?}"
    );
}

/// Shows that no Node of the inherited path comes before the pinned Node
///
/// The command of the compiler puts the directory of the pinned Node first.
/// So whatever Node the inherited path holds, the pinned Node answers first.
#[cfg(unix)]
#[tokio::test]
async fn resolve_toolchain_puts_the_pinned_node_first_on_the_path() {
    let version = pin(&configuration(), TYPESCRIPT);
    let project = Project::on_pinned_node(&format!("\"{TYPESCRIPT}\" = \"{version}\"\n"));
    let tool = Node::resolve_toolchain()
        .action(&Declaring::nothing())
        .toolchain(TYPESCRIPT)
        .executable(TSC)
        .context(&project.context())
        .call()
        .await
        .expect("the test resolves the toolchain on the Node of the project");

    let first = first_on_the_path(&tool);

    assert_eq!(first, Some(project.pinned_node_directory()));
}

/// Shows that a toolchain runs on the Node that the project pins
///
/// The process of the tests inherits a path that can hold another Node
/// first, such as the Node that this repository installs. The compiler still
/// writes the word of the pinned Node.
#[cfg(unix)]
#[tokio::test]
async fn resolve_toolchain_runs_the_toolchain_on_the_pinned_node() {
    let version = pin(&configuration(), TYPESCRIPT);
    let project = Project::on_pinned_node(&format!("\"{TYPESCRIPT}\" = \"{version}\"\n"));
    let tool = Node::resolve_toolchain()
        .action(&Declaring::nothing())
        .toolchain(TYPESCRIPT)
        .executable(TSC)
        .context(&project.context())
        .call()
        .await
        .expect("the test resolves the toolchain on the Node of the project");

    let version = version_of(&tool).await;

    assert_eq!(version, PINNED);
}

#[tokio::test]
async fn resolve_with_a_requirement_that_the_pin_does_not_meet_reports_node() {
    let node = Node::resolve()
        .action(&Declaring::requiring("<1"))
        .context(&context_of_the_repository())
        .call()
        .await;

    assert!(
        matches!(&node, Err(ResolveToolchainError::UnmetRequirement { .. })),
        "expected the lookup to report the unmet requirement, got {node:?}"
    );
}
