//! Tests that drive the crate against real projects
//!
//! Each test builds a project in a temporary directory. So no fixture with a
//! deliberately broken file sits in this repository, where the formatting of
//! the repository itself would fight it.
//!
//! The tests run the prettier that the crate declares. A project copies the
//! `mise.toml` of the repository and trusts it, so the Node that answers is
//! the one that the repository installs. The copy pins another prettier that
//! nothing installs, so a test passes only when the run ignores the pin of
//! the project.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use aonyx_rakko_prettier::{
    Exit, FileExtension, Filter, Observation, Operation, Prettier, ProblemDetail,
};
use rakko::{Context, ProjectRoot, ToolEvent};
use rakko_test_utils::ToolRecorder;
use tempfile::TempDir;

/// A Markdown document that prettier leaves alone
const FORMATTED: &str = "# Project\n";

/// A Markdown document that prettier rewrites
const UNFORMATTED: &str = "#  Project\n";

/// A file of a language that prettier does not know
const FOREIGN: &str = "fn main() {}\n";

/// The key that pins prettier in the mise configuration of a project
const PRETTIER_PIN: &str = "\"npm:prettier\"";

/// The header of the table of tools in a mise configuration
const TOOLS_TABLE: &str = "[tools]\n";

/// A prettier configuration that names an option prettier does not know
///
/// Prettier reports the option and then formats without it, which is the
/// shortest way to make a run write a marker line of its own.
const IGNORED_CONFIGURATION: &str = "{ \"notAnOption\": 5 }\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project without a mise configuration
    ///
    /// The project pins nothing, so it pins no Node either, and nothing in it
    /// reaches a tool.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the Node of this repository and a prettier that
    /// nothing installed
    ///
    /// The project copies the `mise.toml` of this repository, so the Node that
    /// mise resolves for it is the Node that the repository pins and installs.
    /// The copy pins prettier at a version that no machine installs. The run
    /// resolves the prettier that the crate declares, so the pin of the project
    /// must not change what runs. Mise ignores a configuration that nobody
    /// trusts, so the copy is trusted right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = std::fs::read_to_string(repository().join("mise.toml"))
            .expect("the test reads the mise.toml of the repository");
        let (head, tail) = pins
            .split_once(TOOLS_TABLE)
            .expect("the mise.toml of the repository holds a table of tools");
        let copy = project.directory.path().join("mise.toml");
        std::fs::write(
            &copy,
            format!("{head}{TOOLS_TABLE}{PRETTIER_PIN} = \"0.0.1\"\n{tail}"),
        )
        .expect("the test writes the mise.toml of the project");
        trust(&copy);

        project
    }

    /// Returns what a file of this project holds
    fn content(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs one operation of prettier against this project
    async fn observe(&self, operation: Operation, filter: &Filter) -> Observation {
        let prettier = Prettier::resolve(&self.context())
            .await
            .expect("the test resolves the prettier that the crate declares");

        prettier
            .observe(operation, filter)
            .await
            .expect("the test reads a report of prettier")
    }

    /// Returns the context of a run against this project
    fn context(&self) -> Context {
        Context::builder().root(self.root()).build()
    }

    /// Returns the root of this project
    ///
    /// The root is canonical, so the paths that a run reports do not depend on
    /// the symbolic links of the temporary directory.
    fn root(&self) -> ProjectRoot {
        ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists")
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

/// Returns the filter that selects the Markdown files of a project
fn markdown() -> Filter {
    Filter::new([FileExtension::new("md")])
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

#[tokio::test]
async fn observe_a_report_changes_nothing() {
    let project = Project::new();
    project.write().path("messy.md").content(UNFORMATTED).call();

    project.observe(Operation::Report, &markdown()).await;

    assert_eq!(project.content("messy.md"), UNFORMATTED);
}

#[tokio::test]
async fn observe_a_report_names_a_file_that_is_not_formatted() {
    let project = Project::new();
    project.write().path("messy.md").content(UNFORMATTED).call();

    let observation = project.observe(Operation::Report, &markdown()).await;

    assert_eq!(
        observation
            .problems()
            .first()
            .map(|problem| problem.detail()),
        Some(&ProblemDetail::Unformatted)
    );
}

#[tokio::test]
async fn observe_a_rewrite_formats_the_project() {
    let project = Project::new();
    project.write().path("messy.md").content(UNFORMATTED).call();

    project.observe(Operation::Rewrite, &markdown()).await;

    assert_eq!(project.content("messy.md"), FORMATTED);
}

#[tokio::test]
async fn observe_reads_a_report_without_color_codes() {
    let project = Project::new();
    project
        .write()
        .path(".prettierrc.json")
        .content(IGNORED_CONFIGURATION)
        .call();
    project.write().path("clean.md").content(FORMATTED).call();

    let observation = project.observe(Operation::Report, &markdown()).await;

    assert!(
        !observation.stderr().contains('\u{1b}'),
        "expected a report without an escape code, got {:?}",
        observation.stderr()
    );
}

#[tokio::test]
async fn observe_reads_the_marker_of_a_line_that_a_color_would_hide() {
    let project = Project::new();
    project
        .write()
        .path(".prettierrc.json")
        .content(IGNORED_CONFIGURATION)
        .call();
    project.write().path("clean.md").content(FORMATTED).call();

    let observation = project.observe(Operation::Report, &markdown()).await;

    assert!(
        observation.rejected_configuration().is_some(),
        "expected the ignored option, got {:?}",
        observation.stderr()
    );
}

#[tokio::test]
async fn observe_sends_the_start_of_prettier_to_the_tool_output_of_the_context() {
    let project = Project::new();
    project.write().path("messy.md").content(UNFORMATTED).call();
    let recorder = ToolRecorder::new();
    let context = Context::builder()
        .root(project.root())
        .tool_output(recorder.output().clone())
        .build();
    let prettier = Prettier::resolve(&context)
        .await
        .expect("the test resolves the prettier that the crate declares");

    let _observation = prettier.observe(Operation::Report, &markdown()).await;

    assert!(
        recorder
            .events()
            .iter()
            .any(|event| matches!(event, ToolEvent::Started(_))),
        "expected the start of prettier, got {:?}",
        recorder.events()
    );
}

#[tokio::test]
async fn observe_with_a_filter_for_every_extension_skips_an_unknown_language() {
    let project = Project::new();
    project.write().path("main.rs").content(FOREIGN).call();

    let observation = project.observe(Operation::Report, &Filter::default()).await;

    assert_eq!(
        observation.succeeded(),
        Exit::Succeeded,
        "expected a run that ended with success, got {:?}",
        observation.stderr()
    );
}

#[tokio::test]
async fn resolve_in_a_missing_directory_reports_an_error() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();

    let prettier = Prettier::resolve(&context).await;

    assert!(
        prettier.is_err(),
        "expected the resolution to fail, got {prettier:?}"
    );
}

#[tokio::test]
async fn resolve_in_a_project_that_pins_another_prettier_finds_the_declared_one() {
    let project = Project::new();

    let prettier = Prettier::resolve(&project.context()).await;

    assert!(
        prettier.is_ok(),
        "expected the declared prettier, got {prettier:?}"
    );
}

#[tokio::test]
async fn resolve_in_a_project_without_node_reports_node() {
    let project = Project::bare();

    let prettier = Prettier::resolve(&project.context()).await;

    let Err(error) = prettier else {
        panic!("expected the resolution to fail, got {prettier:?}");
    };
    assert!(
        error.to_string().contains("`node`"),
        "expected the error to name node, got {error}"
    );
}
