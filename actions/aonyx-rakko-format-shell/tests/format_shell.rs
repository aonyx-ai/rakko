//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. So no fixture with a deliberately broken script sits in this
//! repository, where the formatting of the repository itself would fight it.
//!
//! The tests run the shfmt that the action declares. Each project pins
//! another shfmt that nothing installs, so a test passes only when the run
//! ignores the pin of the project.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use aonyx_rakko_format_shell::{FormatMode, FormatShell, FormatShellArgs, FormatShellError};
use aonyx_rakko_shfmt::Shfmt;
use rakko::{
    Action, Args, ArgsValues, ArgumentValue, Context, Finding, Location, Outcome, Position,
    ProjectRoot, Summary, argument_name,
};
use rakko_test_utils::{path_text, run_checking_access};
use tempfile::TempDir;

/// A script that shfmt leaves alone
const FORMATTED: &str = "echo hi\n";

/// A script that shfmt rewrites
const UNFORMATTED: &str = "echo  hi\n";

/// A script that shfmt cannot parse
const INVALID: &str = "if then\n";

/// A script without an extension, which shfmt finds by its shebang
const SHEBANG: &str = "#!/bin/bash\necho hi\n";

/// A script with a nested command, which shfmt indents
const NESTED: &str = "if true; then\necho hi\nfi\n";

/// An editorconfig that makes shfmt indent with two spaces
const TWO_SPACES: &str = "root = true\n\n[*.sh]\nindent_style = space\nindent_size = 2\n";

/// An editorconfig that makes shfmt leave out one script
const IGNORE_ONE: &str = "root = true\n\n[skipped.sh]\nignore = true\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project that pins a shfmt that nothing installed
    ///
    /// The pin holds a version that no machine installs. The run resolves the
    /// shfmt that the action declares, so the pin of the project must not
    /// change what runs. Mise ignores a configuration that nobody trusts, so
    /// the file is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\nshfmt = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not
    /// depend on the symbolic links of the temporary directory.
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
    async fn run(&self, format_mode: FormatMode) -> Outcome {
        let values = match format_mode {
            FormatMode::Rewrite => {
                ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))])
            }
            FormatMode::Report => ArgsValues::empty(),
        };
        let args = FormatShellArgs::from_values(&values)
            .expect("the test builds the arguments from values that the action reads");

        FormatShell.run(&self.context(), &args).await
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
    /// Mise records the trust of a configuration in the state of the user.
    /// The temporary directory of a test never comes back, so the record
    /// would pile up there forever. A failure stays quiet, because the test
    /// already reported what it was about.
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
fn action_declares_the_shfmt_of_rakko_shfmt() {
    let tools = FormatShell.tools();

    assert_eq!(tools, vec![Shfmt::requirement()]);
}

#[test]
fn action_identifies_itself_as_format_shell() {
    let name = FormatShell.name();

    assert_eq!(name.get(), "format-shell");
}

#[tokio::test]
async fn run_in_a_formatted_project_counts_the_scripts() {
    let project = Project::new();
    project.write().path("run.sh").content(FORMATTED).call();
    project.write().path("bin/tool").content(SHEBANG).call();

    let outcome = project.run(FormatMode::Report).await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("checked 2 files"),
        "the project holds a script with an extension and one with a shebang"
    );
}

#[tokio::test]
async fn run_in_a_formatted_project_passes() {
    let project = Project::new();
    project.write().path("run.sh").content(FORMATTED).call();

    let outcome = project.run(FormatMode::Report).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_missing_directory_stops_without_a_shfmt() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();
    let args = FormatShellArgs::from_values(&ArgsValues::empty())
        .expect("the test builds the arguments of a run without fix");

    let outcome = FormatShell.run(&context, &args).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(FormatShellError::UnresolvedTool { .. }))
        ),
        "expected the run to stop without a shfmt, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_without_a_script_skips() {
    let project = Project::new();
    project
        .write()
        .path("notes.txt")
        .content("echo  hi\n")
        .call();

    let outcome = project.run(FormatMode::Report).await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_whose_editorconfig_ignores_a_script_leaves_it_alone() {
    let project = Project::new();
    project
        .write()
        .path(".editorconfig")
        .content(IGNORE_ONE)
        .call();
    project.write().path("run.sh").content(FORMATTED).call();
    project
        .write()
        .path("skipped.sh")
        .content(UNFORMATTED)
        .call();

    let outcome = project.run(FormatMode::Report).await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_an_editorconfig_formats_with_its_options() {
    let project = Project::new();
    project
        .write()
        .path(".editorconfig")
        .content(TWO_SPACES)
        .call();
    project.write().path("run.sh").content(NESTED).call();

    project.run(FormatMode::Rewrite).await;

    assert_eq!(project.read("run.sh"), "if true; then\n  echo hi\nfi\n");
}

#[tokio::test]
async fn run_with_an_invalid_script_carries_the_message_of_shfmt() {
    let project = Project::new();
    project.write().path("broken.sh").content(INVALID).call();

    let outcome = project.run(FormatMode::Report).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings.first().map(|finding| finding.message().get()),
        Some("`if` must be followed by a statement list")
    );
}

#[tokio::test]
async fn run_with_an_invalid_script_reports_the_position() {
    let project = Project::new();
    project.write().path("broken.sh").content(INVALID).call();

    let outcome = project.run(FormatMode::Report).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    let position = findings.first().map(|finding| match finding.location() {
        Location::Position { position, .. } => *position,
        other => panic!("expected a finding at a position, got {other:?}"),
    });
    assert_eq!(
        position,
        Some(Position::builder().line(1).column(1).build())
    );
}

#[tokio::test]
async fn run_with_an_unformatted_script_names_the_file() {
    let project = Project::new();
    project.write().path("run.sh").content(FORMATTED).call();
    project
        .write()
        .path("sub/messy.sh")
        .content(UNFORMATTED)
        .call();

    let outcome = project.run(FormatMode::Report).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("sub/messy.sh")]);
}

#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_script_stops() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write().path("secret.sh").content(FORMATTED).call();
    let path = project.directory.path().join("secret.sh");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run(FormatMode::Report).await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(FormatShellError::UnrecognizedReport { .. }))
        ),
        "expected the run to stop, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_fix_and_an_invalid_script_fails() {
    let project = Project::new();
    project.write().path("messy.sh").content(UNFORMATTED).call();
    project.write().path("broken.sh").content(INVALID).call();

    let outcome = project.run(FormatMode::Rewrite).await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_fix_and_an_invalid_script_holds_the_repairs() {
    let project = Project::new();
    project.write().path("messy.sh").content(UNFORMATTED).call();
    project.write().path("broken.sh").content(INVALID).call();

    let outcome = project.run(FormatMode::Rewrite).await;

    let Outcome::Failed { repairs, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(repairs), ["messy.sh"]);
}

#[tokio::test]
async fn run_with_fix_and_an_invalid_script_keeps_the_problem() {
    let project = Project::new();
    project.write().path("messy.sh").content(UNFORMATTED).call();
    project.write().path("broken.sh").content(INVALID).call();

    let outcome = project.run(FormatMode::Rewrite).await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["broken.sh"]);
}

#[tokio::test]
async fn run_with_fix_holds_one_repair_for_each_file() {
    let project = Project::new();
    project.write().path("messy.sh").content(UNFORMATTED).call();
    project
        .write()
        .path("sub/other.sh")
        .content(UNFORMATTED)
        .call();

    let outcome = project.run(FormatMode::Rewrite).await;

    let Outcome::Changed { repairs } = &outcome else {
        panic!("expected the run to change the project, got {outcome:?}");
    };
    let mut repaired = locations(repairs);
    repaired.sort();
    assert_eq!(repaired, ["messy.sh".to_owned(), path_text("sub/other.sh")]);
}

#[tokio::test]
async fn run_with_fix_reports_the_change() {
    let project = Project::new();
    project.write().path("messy.sh").content(UNFORMATTED).call();

    let outcome = project.run(FormatMode::Rewrite).await;

    assert!(
        matches!(outcome, Outcome::Changed { .. }),
        "expected the run to change the project, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_fix_rewrites_the_file() {
    let project = Project::new();
    project.write().path("messy.sh").content(UNFORMATTED).call();

    project.run(FormatMode::Rewrite).await;

    assert_eq!(project.read("messy.sh"), FORMATTED);
}

#[tokio::test]
async fn run_without_fix_leaves_the_file_unchanged() {
    let project = Project::new();
    project.write().path("messy.sh").content(UNFORMATTED).call();

    project.run(FormatMode::Report).await;

    assert_eq!(project.read("messy.sh"), UNFORMATTED);
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_without_fix_leaves_the_project_unchanged() {
    let project = Project::new();
    project.write().path("messy.sh").content(UNFORMATTED).call();

    run_checking_access(
        &FormatShell,
        &project.context(),
        &FormatShellArgs::default(),
    )
    .await;
}
