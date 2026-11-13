//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. So no fixture with a deliberately unformatted file sits in this
//! repository, where the formatting of the repository itself would fight it.
//!
//! The tests run the prettier that the action declares. A project copies the
//! `mise.toml` of the repository and trusts it. So the Node that answers is
//! the one that the repository installs. The copy pins another prettier that
//! nothing installs. So a test passes only when the run ignores the pin of
//! the project.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use aonyx_rakko_format_json::{FormatJson, FormatJsonArgs, FormatJsonError};
use aonyx_rakko_prettier::{Operation, Prettier};
use rakko::{
    Action, Args, ArgsValues, ArgumentValue, Context, Location, Outcome, Position, ProjectRoot,
    argument_name,
};
use rakko_test_utils::run_checking_access;
use tempfile::TempDir;

/// A JSON document that prettier leaves alone
const FORMATTED: &str = "{ \"a\": 1 }\n";

/// A JSON document that prettier rewrites
const UNFORMATTED: &str = "{\"a\":   1}\n";

/// A Markdown document that prettier rewrites, and that the action leaves alone
const UNFORMATTED_MARKDOWN: &str = "#  Notes\n";

/// A JSON document that prettier cannot parse
const INVALID: &str = "{ \"a\": 1,, }\n";

/// A prettier configuration that prettier cannot read
const BROKEN_CONFIGURATION: &str = "{ \"printWidth\": ,, }\n";

/// A prettier configuration that names an option prettier does not know
///
/// Prettier reports the option, ignores it, and formats with its defaults, so
/// the run ends with success and without the configuration of the project.
const IGNORED_CONFIGURATION: &str = "{ \"notAnOption\": 5 }\n";

/// The key that pins prettier in the mise configuration of a project
const PRETTIER_PIN: &str = "\"npm:prettier\"";

/// The header of the table of tools in a mise configuration
const TOOLS_TABLE: &str = "[tools]\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project with the Node of this repository and a prettier that
    /// nothing installed
    ///
    /// The project copies the `mise.toml` of this repository, so the Node that
    /// mise resolves for it is the Node that the repository pins and installs.
    /// The copy pins prettier at a version that no machine installs. The run
    /// resolves the prettier that the action declares, so the pin of the
    /// project must not change what runs. Mise ignores a configuration that
    /// nobody trusts, so the copy is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

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

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        let root = ProjectRoot::canonical(self.directory.path())
            .expect("the test names a directory that exists");

        Context::builder().root(root).build()
    }

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self, fix: Operation) -> Outcome {
        let values = match fix {
            Operation::Rewrite => {
                ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
            }
            _ => ArgsValues::empty(),
        };
        let args = FormatJsonArgs::from_values(&values)
            .expect("the test builds the arguments from values that the action reads");

        FormatJson.run(&self.context(), &args).await
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

#[test]
fn action_declares_prettier_and_its_node() {
    let tools = FormatJson.tools();

    assert_eq!(
        tools,
        vec![Prettier::requirement(), Prettier::node_requirement()]
    );
}

#[test]
fn action_identifies_itself_as_format_json() {
    let name = FormatJson.name();

    assert_eq!(name.get(), "format-json");
}

#[tokio::test]
async fn run_in_a_formatted_project_passes() {
    let project = Project::new();
    project.write().path("clean.json").content(FORMATTED).call();

    let outcome = project.run(Operation::Report).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_missing_directory_stops_without_a_prettier() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();
    let args = FormatJsonArgs::from_values(&ArgsValues::empty())
        .expect("the test builds the arguments of a run without fix");

    let outcome = FormatJson.run(&context, &args).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(FormatJsonError::UnresolvedTool { .. }))
        ),
        "expected the run to stop without a prettier, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_without_json_skips() {
    let project = Project::new();
    project.write().path("notes.txt").content(FORMATTED).call();

    let outcome = project.run(Operation::Report).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_broken_configuration_stops() {
    let project = Project::new();
    project
        .write()
        .path(".prettierrc.json")
        .content(BROKEN_CONFIGURATION)
        .call();
    project.write().path("clean.json").content(FORMATTED).call();

    let outcome = project.run(Operation::Report).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_an_ignored_option_reports_what_prettier_said() {
    let project = Project::new();
    project
        .write()
        .path(".prettierrc.json")
        .content(IGNORED_CONFIGURATION)
        .call();
    project.write().path("clean.json").content(FORMATTED).call();

    let outcome = project.run(Operation::Report).await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("notAnOption"),
        "expected the diagnosis of prettier, got {source}"
    );
}

#[tokio::test]
async fn run_with_an_invalid_file_carries_the_message_of_prettier() {
    let project = Project::new();
    project.write().path("broken.json").content(INVALID).call();

    let outcome = project.run(Operation::Report).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0].message().get().contains("SyntaxError"),
        "expected the message of prettier, got {:?}",
        findings[0].message()
    );
}

#[tokio::test]
async fn run_with_an_invalid_file_reports_the_position() {
    let project = Project::new();
    project.write().path("broken.json").content(INVALID).call();

    let outcome = project.run(Operation::Report).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    let Location::Position { path, position } = findings[0].location() else {
        panic!("expected a finding at a position, got {:?}", findings[0]);
    };
    assert_eq!(
        (path.to_string(), *position),
        (
            "broken.json".to_owned(),
            Position::builder().line(1).column(10).build()
        )
    );
}

#[tokio::test]
async fn run_with_an_unformatted_file_names_the_file() {
    let project = Project::new();
    project
        .write()
        .path("sub/messy.json")
        .content(UNFORMATTED)
        .call();

    let outcome = project.run(Operation::Report).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::File {
            path: "sub/messy.json"
                .parse()
                .expect("the test names a relative path"),
        }
    );
}

#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_and_fix_keeps_the_problem() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project
        .write()
        .path("messy.json")
        .content(UNFORMATTED)
        .call();
    project
        .write()
        .path("secret.json")
        .content(FORMATTED)
        .call();
    let path = project.directory.path().join("secret.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(Operation::Rewrite).await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(findings.len(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_and_fix_reports_the_repair() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project
        .write()
        .path("messy.json")
        .content(UNFORMATTED)
        .call();
    project
        .write()
        .path("secret.json")
        .content(FORMATTED)
        .call();
    let path = project.directory.path().join("secret.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(Operation::Rewrite).await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    let Outcome::Failed { repairs, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(repairs.len(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_carries_the_reason_of_prettier() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project
        .write()
        .path("secret.json")
        .content(FORMATTED)
        .call();
    let path = project.directory.path().join("secret.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(Operation::Report).await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0].message().get().contains("EACCES"),
        "expected the reason of prettier, got {:?}",
        findings[0].message()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_names_the_file() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project
        .write()
        .path("secret.json")
        .content(FORMATTED)
        .call();
    let path = project.directory.path().join("secret.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(Operation::Report).await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::File {
            path: "secret.json"
                .parse()
                .expect("the test names a relative path"),
        }
    );
}

#[tokio::test]
async fn run_with_fix_leaves_the_other_languages_alone() {
    let project = Project::new();
    project
        .write()
        .path("messy.json")
        .content(UNFORMATTED)
        .call();
    project
        .write()
        .path("notes.md")
        .content(UNFORMATTED_MARKDOWN)
        .call();

    project.run(Operation::Rewrite).await;

    assert_eq!(project.read("notes.md"), UNFORMATTED_MARKDOWN);
}

#[tokio::test]
async fn run_with_fix_names_the_file_that_it_repaired() {
    let project = Project::new();
    project
        .write()
        .path("sub/messy.json")
        .content(UNFORMATTED)
        .call();

    let outcome = project.run(Operation::Rewrite).await;

    let Outcome::Changed { repairs } = &outcome else {
        panic!("expected the run to report a change, got {outcome:?}");
    };
    assert_eq!(
        repairs[0].location(),
        &Location::File {
            path: "sub/messy.json"
                .parse()
                .expect("the test names a relative path"),
        }
    );
}

#[tokio::test]
async fn run_with_fix_rewrites_the_file() {
    let project = Project::new();
    project
        .write()
        .path("messy.json")
        .content(UNFORMATTED)
        .call();

    project.run(Operation::Rewrite).await;

    assert_eq!(project.read("messy.json"), FORMATTED);
}

#[tokio::test]
async fn run_without_fix_leaves_the_file_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("messy.json")
        .content(UNFORMATTED)
        .call();

    project.run(Operation::Report).await;

    assert_eq!(project.read("messy.json"), UNFORMATTED);
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_without_fix_leaves_the_project_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("messy.json")
        .content(UNFORMATTED)
        .call();

    run_checking_access(&FormatJson, &project.context(), &FormatJsonArgs::default()).await;
}
