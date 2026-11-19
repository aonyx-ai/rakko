//! The tests of the action of the guide "Use a Toolchain From an Action"
//!
//! A test that runs the action needs mise. A project that runs Node copies the
//! `mise.toml` of this repository, so Node runs at the version that the
//! repository pins and installs.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

// region: imports
use otter_test_node::TestNode;
use rakko::{Action, Context, Outcome, ProjectRoot};
use rakko_tool::ResolveToolchainError;
// endregion: imports
use tempfile::TempDir;

/// A test file in TypeScript that passes
const PASSING: &str = "import { test } from \"node:test\";\n\ntest(\"holds\", (): void => {});\n";

/// A test file in TypeScript that fails
const FAILING: &str = "import { test } from \"node:test\";\nimport assert from \"node:assert\";\n\
                       \ntest(\"counts\", (): void => {\n  assert.strictEqual(1 + 1, 3);\n});\n";

/// A project in a temporary directory, with a mise configuration that mise
/// trusts
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project that pins Node at the given version and nothing else
    fn pinning_node(version: &str) -> Self {
        Self::with_pins(&format!("[tools]\nnode = \"{version}\"\n"))
    }

    /// Creates a project with the pins of this repository
    fn with_repository_pins() -> Self {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../mise.toml");
        let pins = std::fs::read_to_string(repository)
            .expect("the test reads the mise.toml of the repository");

        Self::with_pins(&pins)
    }

    /// Creates a project with the given `mise.toml`, and trusts it
    ///
    /// Mise ignores a configuration that nobody trusts.
    fn with_pins(pins: &str) -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let configuration = directory.path().join("mise.toml");
        std::fs::write(&configuration, pins).expect("the test writes the mise.toml");

        let trusted = Command::new("mise")
            .args(["trust", "--quiet"])
            .arg(&configuration)
            .status()
            .expect("the test starts mise");
        assert!(trusted.success(), "expected mise to trust the mise.toml");

        Self { directory }
    }

    /// Writes a file of the project
    fn write(&self, path: &str, content: &str) {
        std::fs::write(self.directory.path().join(path), content)
            .expect("the test writes a file of the project");
    }

    /// Runs the action against the project
    async fn run(&self) -> Outcome {
        let root = ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists");
        let context = Context::builder().root(root).build();

        TestNode.run(&context, &()).await
    }
}

impl Drop for Project {
    /// Withdraws the trust of the configuration, so that the records of mise
    /// do not keep the paths of temporary directories
    fn drop(&mut self) {
        let _ = Command::new("mise")
            .args(["trust", "--quiet", "--untrust"])
            .arg(self.directory.path().join("mise.toml"))
            .status();
    }
}

// region: unmet
#[tokio::test]
async fn run_on_a_node_that_skips_typescript_stops_with_an_error() {
    let project = Project::pinning_node("20.19.0");

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(ResolveToolchainError::UnmetRequirement { .. })
                )
        ),
        "expected the run to stop on the pin of Node, got {outcome:?}"
    );
}
// endregion: unmet

#[tokio::test]
async fn run_with_a_failing_test_fails() {
    let project = Project::with_repository_pins();
    project.write("add.test.ts", FAILING);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected a failed run, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_passing_test_passes() {
    let project = Project::with_repository_pins();
    project.write("add.test.ts", PASSING);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected a passed run, got {outcome:?}"
    );
}

// region: toolchains
#[test]
fn toolchains_accept_only_a_node_that_runs_typescript() {
    let requirements = TestNode.toolchains();

    let ranges: Vec<&str> = requirements
        .iter()
        .map(|requirement| requirement.versions().get())
        .collect();

    assert_eq!(ranges, ["^22.18 || >=23.6"]);
}
// endregion: toolchains
