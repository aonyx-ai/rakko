//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately unformatted file sits in this
//! repository, where the formatting of the repository itself would fight it.
//!
//! The tests run the prettier that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers is
//! the version that the repository installs, and a new pin reaches the tests
//! without a change to them.
//!
//! Prettier runs on Node, and the program that mise reports starts Node from
//! the environment of the process. The tests therefore run inside the
//! environment of mise, which every Just recipe of this repository enters.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{
    Action, Args, ArgsValues, ArgumentValue, Context, Location, Outcome, Position, argument_name,
};
use rakko_format_json::{FormatJson, FormatJsonArgs};
use tempfile::TempDir;

/// A JSON document that prettier leaves alone
const FORMATTED: &str = "{ \"a\": 1 }\n";

/// A JSON document that prettier rewrites
const UNFORMATTED: &str = "{\"a\":   1}\n";

/// A JSON document that prettier cannot parse
const INVALID: &str = "{ \"a\": 1,, }\n";

/// A prettier configuration that prettier cannot read
const BROKEN_CONFIGURATION: &str = "{ \"printWidth\": ,, }\n";

/// A prettier configuration that names an option prettier does not know
///
/// Prettier reports the option, ignores it, and formats with its defaults, so
/// the run ends with success and without the configuration of the project.
const IGNORED_CONFIGURATION: &str = "{ \"notAnOption\": 5 }\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a prettier to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool
    /// runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the prettier of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the prettier
    /// that mise resolves for it is the prettier that the repository pins and
    /// installs. Mise ignores a configuration that nobody trusts, so the copy
    /// is trusted right away.
    fn new() -> Self {
        let project = Self::bare();

        let pins = repository().join("mise.toml");
        let copy = project.directory.path().join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        project
    }

    /// Creates a project that pins a prettier that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about prettier.
    fn without_prettier() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\n\"npm:prettier\" = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

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

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self, fix: bool) -> Outcome {
        let values = if fix {
            ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
        } else {
            ArgsValues::empty()
        };
        let args = FormatJsonArgs::from_values(&values)
            .expect("the test builds the arguments from values that the action reads");

        FormatJson.run(&self.context(), &args).await
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
    /// Mise records the trust of a configuration in the state of the user, and
    /// the temporary directory of a test never comes back, so the record would
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

// formatjson[verify name]
#[test]
fn action_identifies_itself_as_format_json() {
    let name = FormatJson.name();

    assert_eq!(name.get(), "format-json");
}

// formatjson[verify tool.prettier]
// formatjson[verify check.passed]
#[tokio::test]
async fn run_in_a_formatted_project_passes() {
    let project = Project::new();
    project.write("clean.json", FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// formatjson[verify skip.missing]
#[tokio::test]
async fn run_in_a_project_without_json_names_what_it_looked_for() {
    let project = Project::bare();
    project.write("notes.txt", FORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains(".json"),
        "expected the reason to name the extension, got {reason}"
    );
}

// formatjson[verify skip.missing]
#[tokio::test]
async fn run_in_a_project_without_json_skips() {
    let project = Project::bare();
    project.write("notes.txt", FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formatjson[verify check.configuration]
#[tokio::test]
async fn run_with_a_broken_configuration_stops() {
    let project = Project::new();
    project.write(".prettierrc.json", BROKEN_CONFIGURATION);
    project.write("clean.json", FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}

// formatjson[verify check.configuration]
#[tokio::test]
async fn run_with_an_ignored_option_reports_what_prettier_said() {
    let project = Project::new();
    project.write(".prettierrc.json", IGNORED_CONFIGURATION);
    project.write("clean.json", FORMATTED);

    let outcome = project.run(false).await;

    let Outcome::Errored { source } = &outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("notAnOption"),
        "expected the diagnosis of prettier, got {source}"
    );
}

// formatjson[verify fix.partial]
#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_and_fix_keeps_the_problem() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write("messy.json", UNFORMATTED);
    project.write("secret.json", FORMATTED);
    let path = project.directory.path().join("secret.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(true).await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(findings.len(), 1);
}

// formatjson[verify fix.partial]
#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_and_fix_reports_the_repair() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write("messy.json", UNFORMATTED);
    project.write("secret.json", FORMATTED);
    let path = project.directory.path().join("secret.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(true).await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    let Outcome::Failed { repairs, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(repairs.len(), 1);
}

// formatjson[verify check.unreadable]
#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_carries_the_reason_of_prettier() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write("secret.json", FORMATTED);
    let path = project.directory.path().join("secret.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(false).await;

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

// formatjson[verify check.unreadable]
#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_file_names_the_file() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write("secret.json", FORMATTED);
    let path = project.directory.path().join("secret.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(false).await;

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

// formatjson[verify check.invalid]
#[tokio::test]
async fn run_with_an_invalid_file_carries_the_message_of_prettier() {
    let project = Project::new();
    project.write("broken.json", INVALID);

    let outcome = project.run(false).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0].message().get().contains("SyntaxError"),
        "expected the message of prettier, got {:?}",
        findings[0].message()
    );
}

// formatjson[verify check.invalid]
#[tokio::test]
async fn run_with_an_invalid_file_reports_the_position() {
    let project = Project::new();
    project.write("broken.json", INVALID);

    let outcome = project.run(false).await;

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

// formatjson[verify check.unformatted]
#[tokio::test]
async fn run_with_an_unformatted_file_names_the_file() {
    let project = Project::new();
    project.write("sub/messy.json", UNFORMATTED);

    let outcome = project.run(false).await;

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

// formatjson[verify fix.changed]
#[tokio::test]
async fn run_with_fix_names_the_file_that_it_repaired() {
    let project = Project::new();
    project.write("sub/messy.json", UNFORMATTED);

    let outcome = project.run(true).await;

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

// formatjson[verify fix.write]
#[tokio::test]
async fn run_with_fix_rewrites_the_file() {
    let project = Project::new();
    project.write("messy.json", UNFORMATTED);

    project.run(true).await;

    assert_eq!(project.read("messy.json"), FORMATTED);
}

// formatjson[verify skip.dependencies]
#[tokio::test]
async fn run_with_json_only_under_the_dependencies_skips() {
    let project = Project::bare();
    project.write("node_modules/package/README.json", UNFORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formatjson[verify skip.git]
#[tokio::test]
async fn run_with_json_only_under_the_git_directory_skips() {
    let project = Project::bare();
    project.write(".git/description.json", UNFORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formatjson[verify skip.links]
#[cfg(unix)]
#[tokio::test]
async fn run_with_json_only_behind_a_symbolic_link_skips() {
    let project = Project::bare();
    let elsewhere = tempfile::tempdir().expect("the test creates a temporary directory");
    std::fs::write(elsewhere.path().join("linked.json"), UNFORMATTED)
        .expect("the test writes a file outside the project");
    std::os::unix::fs::symlink(elsewhere.path(), project.directory.path().join("linked"))
        .expect("the test links a directory into the project");

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formatjson[verify check.read]
#[tokio::test]
async fn run_without_fix_leaves_the_file_unchanged() {
    let project = Project::new();
    project.write("messy.json", UNFORMATTED);

    project.run(false).await;

    assert_eq!(project.read("messy.json"), UNFORMATTED);
}

// formatjson[verify tool.missing]
#[tokio::test]
async fn run_without_a_prettier_stops() {
    let project = Project::without_prettier();
    project.write("clean.json", FORMATTED);

    let outcome = project.run(false).await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}
