//! Tests of the action that need no installation of tracey
//!
//! The tests cover what the action declares. They also cover the run that
//! stops when mise cannot read the configuration of the project, so that it
//! reports no tracey. A test that needs files builds a project in a temporary
//! directory, so no fixture sits in this repository.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use rakko::{Action, Args, Context, Outcome, ProjectRoot};
use rakko_check_specs::{CheckSpecs, CheckSpecsError};
use rakko_test_utils::run_checking_access;
use tempfile::TempDir;

/// The configuration that points tracey at the files of a project
const CONFIGURATION: &str = r"@schema {id crate:tracey-config@1, cli tracey}

specs (
    {
        name probe
        source_url https://example.com/probe
        include (SPECIFICATION.md)
        impls (
            {
                name rust
                include (src/**/*.rs)
                test_include (tests/**/*.rs)
            }
        )
    }
)
";

/// A specification with one requirement
const SPECIFICATION: &str = "# Probe\n\nprobe[alpha]\nThe thing MUST do alpha.\n";

/// Code that implements the requirement of the specification
const IMPLEMENTATION: &str = "// probe[impl alpha]\npub fn alpha() {}\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project whose mise configuration mise cannot read
    ///
    /// The mise configuration of the project is no TOML, and mise stops on a
    /// configuration that it cannot read. So mise reports no tracey at the
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
            .path(".config/tracey/config.styx")
            .content(CONFIGURATION)
            .call();
        project
            .write()
            .path("SPECIFICATION.md")
            .content(SPECIFICATION)
            .call();
        project
            .write()
            .path("src/lib.rs")
            .content(IMPLEMENTATION)
            .call();

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        Context::builder().root(self.root()).build()
    }

    /// Returns the canonical root of the project
    fn root(&self) -> ProjectRoot {
        ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists")
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        CheckSpecs.run(&self.context(), &()).await
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
    let schema = <<CheckSpecs as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

#[test]
fn action_identifies_itself_as_check_specs() {
    let name = CheckSpecs.name();

    assert_eq!(name.get(), "check-specs");
}

#[tokio::test]
async fn run_with_an_unreadable_mise_configuration_leaves_the_project_unchanged() {
    let project = Project::with_an_unreadable_mise_configuration();

    run_checking_access(&CheckSpecs, &project.context(), &()).await;
}

#[tokio::test]
async fn run_with_an_unreadable_mise_configuration_stops() {
    let project = Project::with_an_unreadable_mise_configuration();

    let outcome = project.run().await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(CheckSpecsError::UnresolvedTool { .. }))
        ),
        "expected the run to stop without a tracey, got {outcome:?}"
    );
}
