//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. No fixture with a deliberately failing test sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the node that this repository pins. Node is a toolchain, so
//! the project pins it. A project copies the `mise.toml` of the repository and
//! trusts it. The version that answers is the version that the repository
//! installs. A new pin reaches the tests without a change to them.
//!
//! A project states no configuration of the runner, because there is none to
//! state. Node carries the patterns that select the test files. A test file
//! needs nothing but an import of `node:test`.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use aonyx_rakko_test_typescript::{TestTypeScript, TestTypeScriptError};
use rakko::{Action, Args, Context, Finding, Location, Outcome, Position, ProjectRoot};
use rakko_test_utils::{path_text, run_checking_access};
use rakko_tool::ResolveRequirementError;
use tempfile::TempDir;

/// A test that passes
const PASSING: &str = "import { test } from \"node:test\";\n\ntest(\"holds\", () => {});\n";

/// A test that fails an assertion
const FAILING: &str = "import { test } from \"node:test\";\nimport assert from \"node:assert\";\n\
                       \n\ntest(\"counts\", () => {\n  assert.strictEqual(1 + 1, 3);\n});\n";

/// A suite whose test fails, so that Node reports the suite as failed too
const NESTED: &str = "import { describe, it } from \"node:test\";\nimport assert from \
                      \"node:assert\";\n\ndescribe(\"arithmetic\", () => {\n  it(\"counts\", () \
                      => {\n    assert.strictEqual(1 + 1, 3);\n  });\n});\n";

/// TypeScript that holds no test
const SOURCE: &str = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";

/// A test that the project marked as still to do, and that fails
///
/// Node reports such a test as failed and counts it as not failed. So a
/// reading that went by the report alone would fail a run that Node passes.
const TO_DO: &str = "import { test } from \"node:test\";\nimport assert from \"node:assert\";\
                     \n\ntest(\"counts\", { todo: true }, () => {\n  assert.strictEqual(1 + \
                     1, 3);\n});\n";

/// A test that writes a file, to show that the action writes none of its own
const WRITING: &str = "import { test } from \"node:test\";\nimport { writeFileSync } from \
                       \"node:fs\";\n\ntest(\"writes\", () => {\n  \
                       writeFileSync(\"written.txt\", \"from the test\");\n});\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project without a node to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the node of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the node that
    /// mise resolves for it is the node that the repository pins and installs.
    /// Mise ignores a configuration that nobody trusts, so the copy is trusted
    /// right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project
    }

    /// Creates a project that pins node at the given version and nothing else
    ///
    /// A test that pins a version that no machine installs gets no location
    /// for it from mise. This holds whatever the global configuration of the
    /// machine says about node. Mise ignores a configuration that nobody
    /// trusts, so the file is trusted right away.
    fn pinning_node(version: &str) -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, format!("[tools]\nnode = \"{version}\"\n"))
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        let root = ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists");

        Context::builder().root(root).build()
    }

    /// Returns the names that the project holds at its root, sorted
    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.directory.path())
            .expect("the test reads the directory of the project")
            .map(|entry| {
                entry
                    .expect("the test reads an entry of the project")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();

        names
    }

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        TestTypeScript.run(&self.context(), &()).await
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

/// Returns the paths that the findings of an outcome name
fn locations(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::File { path } => path.to_string(),
            Location::Position { path, .. } => path.to_string(),
            other => panic!("expected a finding with a path, got {other:?}"),
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
    /// Mise records the trust of a configuration in the state of the user. The
    /// temporary directory of a test never comes back, so the record would pile
    /// up there forever. A project that never wrote a configuration has
    /// nothing to withdraw. Mise resolves a path that is not there to the
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

#[test]
fn action_declares_no_argument() {
    let schema = <<TestTypeScript as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

#[test]
fn action_identifies_itself_as_test_typescript() {
    let name = TestTypeScript.name();

    assert_eq!(name.get(), "test-typescript");
}

#[tokio::test]
async fn run_leaves_a_file_with_a_failing_test_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(FAILING)
        .call();

    project.run().await;

    assert_eq!(project.read("src/add.test.ts"), FAILING);
}

/// Shows that a run of a project adds no file of its own
///
/// A test that writes a file writes it, because the project asked for that.
/// The file shows that the run reached the test at all. Everything else that
/// the project holds afterwards is what it held before. So the action left no
/// report, no cache, and no installed package behind.
#[tokio::test]
async fn run_of_a_project_adds_no_file_of_its_own() {
    let project = Project::new();
    project
        .write()
        .path("src/write.test.ts")
        .content(WRITING)
        .call();

    project.run().await;

    assert_eq!(project.entries(), ["mise.toml", "src", "written.txt"]);
}

#[tokio::test]
async fn run_of_a_project_whose_failing_test_is_still_to_do_passes() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(TO_DO)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

/// Shows that a run of a project whose suite holds a failing test reports one
/// finding
///
/// Node reports the suite as failed as well as the test inside it, and the test
/// inside it is the one that a reader repairs.
#[tokio::test]
async fn run_of_a_project_whose_suite_holds_a_failing_test_reports_one_finding() {
    let project = Project::new();
    project
        .write()
        .path("src/nested.test.ts")
        .content(NESTED)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(findings.len(), 1);
}

#[tokio::test]
async fn run_of_a_project_whose_tests_pass_passes() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(PASSING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_of_a_project_whose_tests_pass_says_how_many_ran() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(PASSING)
        .call();
    project
        .write()
        .path("src/sub.test.ts")
        .content(PASSING)
        .call();

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(rakko::Summary::get),
        Some("ran 2 tests")
    );
}

#[tokio::test]
async fn run_of_a_project_with_a_failing_test_carries_what_node_said() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(FAILING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0].message().get().contains("2 !== 3"),
        "expected what node said about the failure, got {:?}",
        findings[0].message()
    );
}

#[tokio::test]
async fn run_of_a_project_with_a_failing_test_fails() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(FAILING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_of_a_project_with_a_failing_test_leaves_the_project_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(FAILING)
        .call();

    run_checking_access(&TestTypeScript, &project.context(), &()).await;
}

#[tokio::test]
async fn run_of_a_project_with_a_failing_test_names_the_test() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(FAILING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0].message().get().contains("counts"),
        "expected the name of the test, got {:?}",
        findings[0].message()
    );
}

#[tokio::test]
async fn run_of_a_project_with_a_failing_test_reports_where_it_is_declared() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(FAILING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::Position {
            path: "src/add.test.ts"
                .parse()
                .expect("the test names a relative path"),
            position: Position::builder().line(5).column(1).build(),
        }
    );
}

#[tokio::test]
async fn run_of_a_project_without_a_test_skips() {
    let project = Project::new();
    project.write().path("src/index.ts").content(SOURCE).call();

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("no test"),
        "expected the reason to say that node found no test, got {reason:?}"
    );
}

#[tokio::test]
async fn run_reaches_a_test_below_a_directory_of_the_project() {
    let project = Project::new();
    project
        .write()
        .path("packages/core/src/deep.test.ts")
        .content(FAILING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        locations(findings),
        [path_text("packages/core/src/deep.test.ts")]
    );
}

/// Shows that a run reads the report of node rather than its drawing
///
/// The report reads as TAP only because the run asks for it. The drawing that
/// Node makes for a reader carries a summary that no parse answers. So this
/// test also fails wherever the environment would make Node draw.
#[tokio::test]
async fn run_reads_the_report_of_node_rather_than_its_drawing() {
    let project = Project::new();
    project
        .write()
        .path("src/add.test.ts")
        .content(FAILING)
        .call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        !findings[0].message().get().contains('\u{1b}'),
        "expected a message without an escape code, got {:?}",
        findings[0].message()
    );
}

#[tokio::test]
async fn run_with_a_node_between_the_releases_that_strip_types_stops_on_node() {
    let project = Project::pinning_node("23.5.0");
    project
        .write()
        .path("src/add.test.ts")
        .content(PASSING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(TestTypeScriptError::UnresolvedTool {
                        source: ResolveRequirementError::VersionOutOfRange { tool, .. },
                    }) if tool.get() == "node"
                )
        ),
        "expected the run to stop on the Node of the project, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_node_outside_the_range_stops_on_node() {
    let project = Project::pinning_node("20.19.0");
    project
        .write()
        .path("src/add.test.ts")
        .content(PASSING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(TestTypeScriptError::UnresolvedTool {
                        source: ResolveRequirementError::VersionOutOfRange { tool, .. },
                    }) if tool.get() == "node"
                )
        ),
        "expected the run to stop on the Node of the project, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_without_a_node_stops() {
    let project = Project::pinning_node("99.0.0");
    project
        .write()
        .path("src/add.test.ts")
        .content(PASSING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(TestTypeScriptError::UnresolvedTool {
                        source: ResolveRequirementError::UnresolvedVersion { tool, .. },
                    }) if tool.get() == "node"
                )
        ),
        "expected the run to stop without a node, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_without_a_pinned_node_stops() {
    let project = Project::bare();
    project
        .write()
        .path("src/add.test.ts")
        .content(PASSING)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(TestTypeScriptError::UnresolvedTool {
                        source: ResolveRequirementError::UnresolvedToolchain { tool, .. },
                    }) if tool.get() == "node"
                )
        ),
        "expected the run to stop without a pinned node, got {outcome:?}"
    );
}
