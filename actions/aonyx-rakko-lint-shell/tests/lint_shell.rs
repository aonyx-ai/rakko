//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. So no fixture with a deliberately broken script sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the shfmt and the ShellCheck that the action declares. Each
//! project pins other versions that nothing installs, so a test passes only
//! when the run ignores the pins of the project.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use aonyx_rakko_lint_shell::{LintShell, LintShellError};
use rakko::{Action, Context, Finding, Location, Outcome, Position, ProjectRoot, Span, Summary};
use rakko_test_utils::{path_text, run_checking_access};
use tempfile::TempDir;

/// A script that ShellCheck finds nothing in
const CLEAN: &str = "#!/bin/sh\necho \"$1\"\n";

/// A script with an argument that is not quoted
const UNQUOTED: &str = "#!/bin/sh\necho $1\n";

/// A script with an argument that is not quoted, in an indented block
const INDENTED: &str = "#!/bin/sh\nif true; then\n\techo $1\nfi\n";

/// A script that ShellCheck cannot parse
const INVALID: &str = "#!/bin/sh\nif then\n";

/// A configuration of ShellCheck that turns off the check for quotes
const NO_QUOTES: &str = "disable=SC2086\n";

/// An editorconfig that makes shfmt leave out one script
const IGNORE_ONE: &str = "root = true\n\n[skipped.sh]\nignore = true\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project that pins tools that nothing installed
    ///
    /// The pins hold versions that no machine installs. The run resolves the
    /// tools that the action declares, so the pins of the project must not
    /// change what runs. Mise ignores a configuration that nobody trusts, so
    /// the file is trusted right away.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let project = Self { directory };

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(
            &pins,
            "[tools]\nshellcheck = \"0.0.1\"\nshfmt = \"0.0.1\"\n",
        )
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
            .expect("the test gives a directory that exists");

        Context::builder().root(root).build()
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        LintShell.run(&self.context(), &()).await
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

/// Returns the paths of the findings of an outcome
fn locations(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::Span { path, .. } => path.to_string(),
            other => panic!("expected a finding over a range, got {other:?}"),
        })
        .collect()
}

/// Returns the messages of the findings of an outcome
fn messages(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.message().get())
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
fn action_identifies_itself_as_lint_shell() {
    let name = LintShell.name();

    assert_eq!(name.get(), "lint-shell");
}

#[tokio::test]
async fn run_in_a_clean_project_counts_the_scripts() {
    let project = Project::new();
    project.write().path("run.sh").content(CLEAN).call();
    project.write().path("bin/tool").content(CLEAN).call();

    let outcome = project.run().await;

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
async fn run_in_a_clean_project_passes() {
    let project = Project::new();
    project.write().path("run.sh").content(CLEAN).call();

    let outcome = project.run().await;

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

    let outcome = LintShell.run(&context, &()).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(LintShellError::UnresolvedShfmt { .. }))
        ),
        "expected the run to stop without a shfmt, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_without_a_script_skips() {
    let project = Project::new();
    project.write().path("notes.txt").content(UNQUOTED).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_leaves_the_project_unchanged() {
    let project = Project::new();
    project.write().path("run.sh").content(UNQUOTED).call();

    run_checking_access(&LintShell, &project.context(), &()).await;
}

#[tokio::test]
async fn run_whose_editorconfig_ignores_a_script_leaves_it_alone() {
    let project = Project::new();
    project
        .write()
        .path(".editorconfig")
        .content(IGNORE_ONE)
        .call();
    project.write().path("run.sh").content(CLEAN).call();
    project.write().path("skipped.sh").content(UNQUOTED).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_whose_shellcheckrc_disables_a_check_passes() {
    let project = Project::new();
    project
        .write()
        .path(".shellcheckrc")
        .content(NO_QUOTES)
        .call();
    project.write().path("run.sh").content(UNQUOTED).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_a_problem_after_a_tab_counts_the_tab_as_one_column() {
    let project = Project::new();
    project.write().path("run.sh").content(INDENTED).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    let start = findings.first().map(|finding| match finding.location() {
        Location::Span { span, .. } => *span.start(),
        other => panic!("expected a finding over a range, got {other:?}"),
    });
    assert_eq!(start, Some(Position::builder().line(3).column(7).build()));
}

#[tokio::test]
async fn run_with_a_problem_carries_the_message_of_shellcheck() {
    let project = Project::new();
    project.write().path("run.sh").content(UNQUOTED).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        messages(findings),
        ["[info] Double quote to prevent globbing and word splitting. (SC2086)"]
    );
}

#[tokio::test]
async fn run_with_a_problem_covers_the_range_of_shellcheck() {
    let project = Project::new();
    project.write().path("run.sh").content(UNQUOTED).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    let span = findings.first().map(|finding| match finding.location() {
        Location::Span { span, .. } => *span,
        other => panic!("expected a finding over a range, got {other:?}"),
    });
    assert_eq!(
        span,
        Some(
            Span::builder()
                .start(Position::builder().line(2).column(6).build())
                .end(Position::builder().line(2).column(8).build())
                .build()
        )
    );
}

#[tokio::test]
async fn run_with_a_problem_in_a_subdirectory_gives_the_path() {
    let project = Project::new();
    project.write().path("run.sh").content(CLEAN).call();
    project.write().path("sub/run.sh").content(UNQUOTED).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("sub/run.sh")]);
}

#[tokio::test]
async fn run_with_a_script_whose_name_starts_with_a_dash_examines_it() {
    let project = Project::new();
    project.write().path("-run.sh").content(UNQUOTED).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), ["-run.sh"]);
}

#[tokio::test]
async fn run_with_an_invalid_script_fails() {
    let project = Project::new();
    project.write().path("broken.sh").content(INVALID).call();

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        messages(findings)
            .iter()
            .all(|message| message.starts_with("[error] ")),
        "expected only errors, got {findings:?}"
    );
}

/// Shows that a script that nobody can read stops the run
///
/// Shfmt opens each file during its search, so it stops at the script before
/// ShellCheck sees it.
#[cfg(unix)]
#[tokio::test]
async fn run_with_an_unreadable_script_stops() {
    use std::os::unix::fs::PermissionsExt;

    let project = Project::new();
    project.write().path("secret.sh").content(CLEAN).call();
    let path = project.directory.path().join("secret.sh");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
        .expect("the test removes the permissions of a file");

    let outcome = project.run().await;

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("the test restores the permissions of a file");
    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(LintShellError::UnrecognizedShfmtReport { .. }))
        ),
        "expected the run to stop, got {outcome:?}"
    );
}
