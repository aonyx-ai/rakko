//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately unformatted file sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the oxfmt that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers is
//! the version that the repository installs, and a new pin reaches the tests
//! without a change to them.
//!
//! A project states no configuration of oxfmt unless the test is about one.
//! Oxfmt formats with its own defaults in that case, and it reads no
//! configuration of the user, so a project without one answers the same way on
//! the machine of every contributor.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Position, ProjectRoot};
use rakko_format_typescript::{FormatTypeScript, FormatTypeScriptArgs};
use tempfile::TempDir;

/// TypeScript that oxfmt would rewrite
const UNFORMATTED: &str = "export  const   answer=42\n";

/// The same TypeScript as oxfmt writes it
const FORMATTED: &str = "export const answer = 42;\n";

/// TypeScript that oxfmt cannot parse
const BROKEN: &str = "export const = ;;;\n";

/// Markdown that oxfmt would rewrite if the action asked it about Markdown
///
/// Oxfmt formats the code that a fenced block of a Markdown file holds, and
/// the action that wraps prettier owns Markdown in a project that mounts it.
const UNFORMATTED_MARKDOWN: &str = "# Guide\n\n```js\nexport  const   answer=42\n```\n";

/// A configuration that oxfmt refuses to read
const REFUSED_CONFIGURATION: &str = "{ this is not JSON\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without an oxfmt to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the oxfmt of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the oxfmt that
    /// mise resolves for it is the oxfmt that the repository pins and installs.
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

    /// Creates a project that pins an oxfmt that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about oxfmt.
    fn without_oxfmt() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\noxfmt = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend on
    /// the symbolic links of the temporary directory.
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

    /// Runs the action against this project, and reports what it found
    async fn report(&self) -> Outcome {
        self.run(&FormatTypeScriptArgs::default()).await
    }

    /// Runs the action against this project with the given arguments
    async fn run(&self, args: &FormatTypeScriptArgs) -> Outcome {
        FormatTypeScript.run(&self.context(), args).await
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

/// Returns the arguments of a run that rewrites what it can
fn fixing() -> FormatTypeScriptArgs {
    FormatTypeScriptArgs::from_values(&rakko_action::ArgsValues::new([(
        rakko_action::argument_name!("fix"),
        rakko_action::ArgumentValue::new("true"),
    )]))
    .expect("the test builds arguments that ask for a rewrite")
}

/// Returns the paths that the findings of an outcome name
fn locations(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::File { path } | Location::Position { path, .. } => path.to_string(),
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
    /// Mise records the trust of a configuration in the state of the user, and
    /// the temporary directory of a test never comes back, so the record would
    /// pile up there forever. A project that never wrote a configuration has
    /// nothing to withdraw, and mise resolves a path that is not there to the
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

// formattypescript[verify args.fix]
#[test]
fn action_declares_the_fix_argument() {
    let schema = <<FormatTypeScript as Action>::Args as Args>::schema();

    let names: Vec<&str> = schema
        .arguments()
        .iter()
        .map(|argument| argument.name().get())
        .collect();

    assert_eq!(names, ["fix"]);
}

// formattypescript[verify name]
#[test]
fn action_identifies_itself_as_format_typescript() {
    let name = FormatTypeScript.name();

    assert_eq!(name.get(), "format-typescript");
}

// formattypescript[verify files.extensions]
#[tokio::test]
async fn fixing_run_leaves_the_other_languages_of_a_project_alone() {
    let project = Project::new();
    project.write("src/index.ts", UNFORMATTED);
    project.write("docs/guide.md", UNFORMATTED_MARKDOWN);

    project.run(&fixing()).await;

    assert_eq!(project.read("docs/guide.md"), UNFORMATTED_MARKDOWN);
}

// formattypescript[verify fix.changed]
#[tokio::test]
async fn fixing_run_reports_the_file_that_it_rewrote() {
    let project = Project::new();
    project.write("src/index.ts", UNFORMATTED);

    let outcome = project.run(&fixing()).await;

    let Outcome::Changed { repairs } = outcome else {
        panic!("expected the run to report a change, got {outcome:?}");
    };
    assert_eq!(locations(&repairs), ["src/index.ts"]);
}

// formattypescript[verify fix.write]
#[tokio::test]
async fn fixing_run_rewrites_an_unformatted_file() {
    let project = Project::new();
    project.write("src/index.ts", UNFORMATTED);

    project.run(&fixing()).await;

    assert_eq!(project.read("src/index.ts"), FORMATTED);
}

// formattypescript[verify fix.partial]
#[tokio::test]
async fn fixing_run_that_left_a_problem_fails() {
    let project = Project::new();
    project.write("src/index.ts", UNFORMATTED);
    project.write("src/broken.ts", BROKEN);

    let outcome = project.run(&fixing()).await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the run to fail, got {outcome:?}"
    );
}

// formattypescript[verify fix.partial]
#[tokio::test]
async fn fixing_run_that_left_a_problem_holds_the_repairs_beside_it() {
    let project = Project::new();
    project.write("src/index.ts", UNFORMATTED);
    project.write("src/broken.ts", BROKEN);

    let outcome = project.run(&fixing()).await;

    let Outcome::Failed { repairs, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(&repairs), ["src/index.ts"]);
}

// formattypescript[verify check.decorated]
// formattypescript[verify check.invalid]
#[tokio::test]
async fn report_of_a_file_that_oxfmt_cannot_parse_sits_at_the_place_of_oxfmt() {
    let project = Project::new();
    project.write("src/index.ts", BROKEN);

    let outcome = project.report().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings.first().map(Finding::location),
        Some(&Location::Position {
            path: "src/index.ts".parse().expect("the test names a file"),
            position: Position::builder().line(1).column(14).build(),
        })
    );
}

// formattypescript[verify check.operation]
// formattypescript[verify check.passed]
// formattypescript[verify tool.oxfmt]
#[tokio::test]
async fn report_of_a_formatted_project_passes() {
    let project = Project::new();
    project.write("src/index.ts", FORMATTED);

    let outcome = project.report().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// formattypescript[verify tool.missing]
#[tokio::test]
async fn report_of_a_project_whose_oxfmt_mise_does_not_report_holds_the_error() {
    let project = Project::without_oxfmt();
    project.write("src/index.ts", UNFORMATTED);

    let outcome = project.report().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to end with an error, got {outcome:?}"
    );
}

// formattypescript[verify skip.unmatched]
#[tokio::test]
async fn report_of_a_project_without_typescript_names_the_tool() {
    let project = Project::new();
    project.write("docs/guide.md", "# Guide\n");

    let outcome = project.report().await;

    let Outcome::Skipped { reason } = outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("oxfmt"),
        "expected the reason to name the tool, got {reason:?}"
    );
}

// formattypescript[verify skip.unmatched]
#[tokio::test]
async fn report_of_a_project_without_typescript_skips() {
    let project = Project::new();
    project.write("docs/guide.md", "# Guide\n");

    let outcome = project.report().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// formattypescript[verify check.configuration]
#[tokio::test]
async fn report_of_a_refused_configuration_holds_what_oxfmt_reported() {
    let project = Project::new();
    project.write("src/index.ts", UNFORMATTED);
    project.write(".oxfmtrc.json", REFUSED_CONFIGURATION);

    let outcome = project.report().await;

    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to end with an error, got {outcome:?}");
    };
    assert!(
        source.to_string().contains("configuration"),
        "expected the diagnosis of oxfmt, got {source}"
    );
}

// formattypescript[verify check.read]
#[tokio::test]
async fn report_of_an_unformatted_file_leaves_it_alone() {
    let project = Project::new();
    project.write("src/index.ts", UNFORMATTED);

    project.report().await;

    assert_eq!(project.read("src/index.ts"), UNFORMATTED);
}

// formattypescript[verify check.unformatted]
#[tokio::test]
async fn report_of_an_unformatted_file_names_it() {
    let project = Project::new();
    project.write("src/index.ts", UNFORMATTED);

    let outcome = project.report().await;

    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(&findings), ["src/index.ts"]);
}
