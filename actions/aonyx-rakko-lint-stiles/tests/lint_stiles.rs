//! Tests of the action that build no stiles rule
//!
//! The tests cover what the action declares. They also cover the run that
//! stops when mise cannot read the configuration of the project, so that it
//! reports no stiles. One test starts the stiles that the action declares in a
//! project that names no lint, so it needs that stiles installed. A test that
//! reads what stiles reported about a rule would have to build the rule first.
//! So these tests live beside the code that reads the reports, and they read
//! reports in the shape that stiles writes.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use aonyx_rakko_lint_stiles::{LintStiles, LintStilesError};
use rakko::{Action, Args, Context, Outcome, ProjectRoot};
use rakko_test_utils::run_checking_access;
use tempfile::TempDir;

/// A configuration of stiles that names a lint of the project
const CONFIGURATION: &str = "[[lints]]\npath = \"lints/todo_comment\"\n";

/// Code that a stiles rule could report
const SOURCE: &str = "// TODO: finish this\npub fn alpha() {}\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project that holds only code
    ///
    /// The project has no mise configuration and names no lint, so stiles
    /// builds nothing before it answers.
    #[cfg(any(target_os = "linux", all(target_os = "macos", target_arch = "aarch64")))]
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        project.write().path("src/lib.rs").content(SOURCE).call();

        project
    }

    /// Creates a project whose mise configuration mise cannot read
    ///
    /// The mise configuration of the project is no TOML, and mise stops on a
    /// configuration that it cannot read. So mise reports no stiles at the
    /// declared version, whatever versions the machine installed.
    fn with_an_unreadable_mise_configuration() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        project
            .write()
            .path("mise.toml")
            .content("this is not a configuration\n")
            .call();
        project
            .write()
            .path(".config/stiles.toml")
            .content(CONFIGURATION)
            .call();
        project.write().path("src/lib.rs").content(SOURCE).call();

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

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        LintStiles.run(&self.context(), &()).await
    }

    /// Runs the action against this project like [`Project::run`], with a
    /// context that sends what its tools write to a recorder
    #[cfg(any(target_os = "linux", all(target_os = "macos", target_arch = "aarch64")))]
    async fn run_recording(&self, recorder: &rakko_test_utils::ToolRecorder) -> Outcome {
        let context = Context::builder()
            .root(self.context().root().clone())
            .tool_output(recorder.output().clone())
            .build();

        LintStiles.run(&context, &()).await
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

#[test]
fn action_declares_no_argument() {
    let schema = <<LintStiles as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

#[test]
fn action_identifies_itself_as_lint_stiles() {
    let name = LintStiles.name();

    assert_eq!(name.get(), "lint-stiles");
}

/// Shows that the run sends the start of stiles to the tool output
///
/// The project pins stiles only for Linux and for macOS on ARM, so the test
/// runs only there.
#[cfg(any(target_os = "linux", all(target_os = "macos", target_arch = "aarch64")))]
#[tokio::test]
async fn run_sends_the_start_of_stiles_to_the_tool_output_of_the_context() {
    let project = Project::new();
    let recorder = rakko_test_utils::ToolRecorder::new();

    let _outcome = project.run_recording(&recorder).await;

    assert!(
        recorder
            .events()
            .iter()
            .any(|event| matches!(event, rakko::ToolEvent::Started(_))),
        "expected the start of stiles, got {:?}",
        recorder.events()
    );
}

/// Shows that the machinery skips the action where stiles has no build
///
/// Stiles has no build for Windows, so the test runs only there. A run
/// through [`ErasedAction`] is the path that the command line, a group, and a
/// written command take.
#[cfg(windows)]
#[tokio::test]
async fn run_through_the_machinery_on_windows_skips_for_the_missing_build() {
    let project = Project::with_an_unreadable_mise_configuration();

    let outcome = rakko::ErasedAction::run(
        &LintStiles,
        &project.context(),
        &rakko::ArgsValues::default(),
    )
    .await;

    assert!(
        matches!(
            &outcome,
            Outcome::Skipped { reason }
                if reason.get().starts_with("the tool github:aonyx-ai/stiles has no build for windows/")
        ),
        "expected the run to skip for the missing build of stiles, got {outcome:?}"
    );
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_with_an_unreadable_mise_configuration_leaves_the_project_unchanged() {
    let project = Project::with_an_unreadable_mise_configuration();

    run_checking_access(&LintStiles, &project.context(), &()).await;
}

#[tokio::test]
async fn run_with_an_unreadable_mise_configuration_stops() {
    let project = Project::with_an_unreadable_mise_configuration();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(LintStilesError::UnresolvedTool { .. }))
        ),
        "expected the run to stop without a stiles, got {outcome:?}"
    );
}
