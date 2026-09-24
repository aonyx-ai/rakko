//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately broken file sits in this
//! repository, where the checks of the repository itself would fight it.
//!
//! The tests run the tsc that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers is
//! the version that the repository installs, and a new pin reaches the tests
//! without a change to them.
//!
//! A project states the compiler options that its test is about and no others.
//! Tsc applies its own defaults for the rest, and it reads no configuration of
//! the user, so a project answers the same way on the machine of every
//! contributor.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};
use std::process::Command;

use rakko_action::{Action, Args, Context, Finding, Location, Outcome, Position, ProjectRoot};
use rakko_check_typescript::CheckTypeScript;
use rakko_test_utils::path_text;
use tempfile::TempDir;

/// A configuration that selects the source directory of the project
const CONFIGURATION: &str = "{\n  \"include\": [\"src\"]\n}\n";

/// A configuration that selects a directory which the project does not have
const ELSEWHERE: &str = "{\n  \"include\": [\"elsewhere\"]\n}\n";

/// A configuration that also selects a directory beside the project
const PARENT: &str = "{\n  \"include\": [\"src\", \"../shared\"]\n}\n";

/// A configuration that writes the output of a build into a directory
const EMITTING: &str = "{\n  \"compilerOptions\": { \"outDir\": \"dist\" },\n  \"include\": \
                        [\"src\"]\n}\n";

/// A configuration that asks tsc for a rule that it applies to nothing else
///
/// The rule reports a parameter that the body of a function never reads, and
/// tsc applies it only where the project asks for it, so a project that states
/// this and a project that states nothing disagree about the same file.
const UNUSED_PARAMETERS: &str = "{\n  \"compilerOptions\": { \"noUnusedParameters\": true },\n  \
                                 \"include\": [\"src\"]\n}\n";

/// A configuration that names a compiler option which tsc does not have
const UNKNOWN_OPTION: &str = "{\n  \"compilerOptions\": { \"noBadCode\": true },\n  \"include\": \
                              [\"src\"]\n}\n";

/// TypeScript that holds together
const VALID: &str = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";

/// TypeScript that gives a value the wrong type
const WRONG_TYPE: &str = "export const count: number = \"three\";\n";

/// TypeScript whose parameter nothing reads
const UNUSED_PARAMETER: &str = "export function greet(name: string): string {\n  return \
                                \"hello\";\n}\n";

/// TypeScript whose function does not fit the type that it is given
///
/// Tsc explains this one in further lines, because the first sentence says
/// that the two do not fit and never says why.
const EXPLAINED: &str = "type Greet = (name: string) => string;\n\nexport const greet: Greet = \
                         (name: number): string => `${name}`;\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The temporary directory that holds the project
    directory: TempDir,

    /// The root of the project, which is the temporary directory or a
    /// directory in it
    root: PathBuf,
}

impl Project {
    /// Creates a project without a tsc to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");
        let root = directory.path().to_path_buf();

        Self { directory, root }
    }

    /// Returns the project with the tsc of this repository pinned in its root
    ///
    /// The project copies the `mise.toml` of this repository into its root,
    /// so the tsc that mise resolves for it is the tsc that the repository
    /// pins and installs. Mise ignores a configuration that nobody trusts, so
    /// the copy is trusted right away.
    fn pinned(self) -> Self {
        let pins = repository().join("mise.toml");
        let copy = self.root.join("mise.toml");
        std::fs::copy(&pins, &copy).expect("the test copies the mise.toml of the repository");
        trust(&copy);

        self
    }

    /// Creates a project with the tsc of this repository
    fn new() -> Self {
        Self::bare().pinned()
    }

    /// Creates a project with the tsc of this repository, in a directory of
    /// the temporary directory
    ///
    /// The temporary directory around the project gives a test a place for
    /// files outside the project, which [`write_beside`][beside] fills.
    ///
    /// [beside]: Project::write_beside
    fn nested() -> Self {
        let mut project = Self::bare();
        project.root = project.directory.path().join("project");
        std::fs::create_dir(&project.root).expect("the test creates the root of the project");

        project.pinned()
    }

    /// Creates a project that pins a TypeScript that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about TypeScript.
    fn without_tsc() -> Self {
        let project = Self::bare();

        let pins = project.root.join("mise.toml");
        std::fs::write(&pins, "[tools]\n\"npm:typescript\" = \"0.0.1\"\n")
            .expect("the test writes the mise.toml of the project");
        trust(&pins);

        project
    }

    /// Returns the context of a run against this project
    ///
    /// The root is canonical, so the paths that the run reports do not depend
    /// on the symbolic links of the temporary directory.
    fn context(&self) -> Context {
        let root =
            ProjectRoot::canonical(&self.root).expect("the test names a directory that exists");

        Context::builder().root(root).build()
    }

    /// Returns whether the project holds the given path
    fn holds(&self, path: &str) -> bool {
        self.root.join(path).exists()
    }

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.root.join(path)).expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        CheckTypeScript.run(&self.context(), &()).await
    }

    /// Writes a file of the project, with the directories that lead to it
    fn write(&self, path: &str, content: &str) {
        create(&self.root.join(path), content);
    }

    /// Writes a file beside the project, in the temporary directory that
    /// holds a [nested][nested] project
    ///
    /// A project that is not nested is the temporary directory itself, so
    /// there the file lands in the project.
    ///
    /// [nested]: Project::nested
    fn write_beside(&self, path: &str, content: &str) {
        create(&self.directory.path().join(path), content);
    }
}

/// Writes a file, with the directories that lead to it
fn create(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the test creates the directories of a file");
    }

    std::fs::write(path, content).expect("the test writes a file");
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
    /// Mise records the trust of a configuration in the state of the user, and
    /// the temporary directory of a test never comes back, so the record would
    /// pile up there forever. A project that never wrote a configuration has
    /// nothing to withdraw, and mise resolves a path that is not there to the
    /// directory above it, so the file has to exist first. A failure stays
    /// quiet, because the test already reported what it was about.
    fn drop(&mut self) {
        let pins = self.root.join("mise.toml");

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

// checktypescript[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <<CheckTypeScript as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

// checktypescript[verify name]
#[test]
fn action_identifies_itself_as_check_typescript() {
    let name = CheckTypeScript.name();

    assert_eq!(name.get(), "check-typescript");
}

// checktypescript[verify check.passed]
// checktypescript[verify tool.tsc]
#[tokio::test]
async fn run_in_a_valid_project_passes() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// checktypescript[verify check.read]
#[tokio::test]
async fn run_leaves_a_file_with_a_type_error_unchanged() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", WRONG_TYPE);

    project.run().await;

    assert_eq!(project.read("src/index.ts"), WRONG_TYPE);
}

// checktypescript[verify run.project]
#[tokio::test]
async fn run_reaches_a_file_below_a_directory_of_the_project() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", VALID);
    project.write("src/deep/sub/broken.ts", WRONG_TYPE);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("src/deep/sub/broken.ts")]);
}

// checktypescript[verify check.configuration]
#[tokio::test]
async fn run_reads_the_configuration_of_the_project() {
    let project = Project::new();
    project.write("tsconfig.json", UNUSED_PARAMETERS);
    project.write("src/index.ts", UNUSED_PARAMETER);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Failed { .. }),
        "expected the options of the project to answer, got {outcome:?}"
    );
}

// checktypescript[verify check.configuration]
#[tokio::test]
async fn run_without_the_option_of_the_project_passes_the_same_file() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", UNUSED_PARAMETER);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the defaults of tsc to answer, got {outcome:?}"
    );
}

// checktypescript[verify run.noemit]
#[tokio::test]
async fn run_of_a_project_that_builds_writes_no_output() {
    let project = Project::new();
    project.write("tsconfig.json", EMITTING);
    project.write("src/index.ts", VALID);

    project.run().await;

    assert!(
        !project.holds("dist"),
        "expected the run to write no output directory"
    );
}

// A configuration that tsc refuses belongs to the file that states it, so the
// run reports it where a reader can repair it.
// checktypescript[verify check.diagnostic+2]
#[tokio::test]
async fn run_with_a_refused_configuration_reports_the_configuration_file() {
    let project = Project::new();
    project.write("tsconfig.json", UNKNOWN_OPTION);
    project.write("src/index.ts", VALID);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(locations(findings), [path_text("tsconfig.json")]);
}

// checktypescript[verify check.foreign]
#[tokio::test]
async fn run_with_a_type_error_beside_the_project_names_its_place() {
    let project = Project::nested();
    project.write("tsconfig.json", PARENT);
    project.write("src/index.ts", VALID);
    project.write_beside("shared/index.ts", WRONG_TYPE);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].message().get(),
        "error TS2322: Type 'string' is not assignable to type 'number'. at \
         ../shared/index.ts:1:14"
    );
}

// The message reads like the line that tsc writes, which it does only because
// the run asks for the diagnostics without the drawing. A drawn report names
// the place in another shape, and it carries the source of the file and the
// marks under it, so this test also fails wherever the environment would make
// tsc draw.
// checktypescript[verify check.diagnostic+2]
#[tokio::test]
async fn run_with_a_type_error_carries_the_message_of_tsc() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", WRONG_TYPE);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].message().get(),
        "error TS2322: Type 'string' is not assignable to type 'number'."
    );
}

// Tsc draws its diagnostics in color when it writes to a terminal, and an
// environment that states a terminal is capable of color makes it draw without
// one, which is how a report that reads on a workstation arrives in escape
// codes on a build server.
// checktypescript[verify run.plain]
#[tokio::test]
async fn run_with_a_type_error_reports_a_message_without_color_codes() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", WRONG_TYPE);

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

// checktypescript[verify check.diagnostic+2]
#[tokio::test]
async fn run_with_a_type_error_reports_the_position_of_tsc() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", WRONG_TYPE);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(
        findings[0].location(),
        &Location::Position {
            path: "src/index.ts"
                .parse()
                .expect("the test names a relative path"),
            position: Position::builder().line(1).column(14).build(),
        }
    );
}

// checktypescript[verify check.elaboration]
#[tokio::test]
async fn run_with_an_explained_diagnostic_carries_the_explanation() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", EXPLAINED);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert!(
        findings[0]
            .message()
            .get()
            .contains("Types of parameters 'name' and 'name' are incompatible."),
        "expected the explanation of tsc, got {:?}",
        findings[0].message()
    );
}

// checktypescript[verify check.elaboration]
#[tokio::test]
async fn run_with_an_explained_diagnostic_reports_one_finding() {
    let project = Project::new();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", EXPLAINED);

    let outcome = project.run().await;

    let Outcome::Failed { findings, .. } = &outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };
    assert_eq!(findings.len(), 1);
}

// A project that holds no configuration of tsc is no TypeScript project, and
// tsc reports that it found none rather than that it found no problem.
// checktypescript[verify skip.unconfigured]
#[tokio::test]
async fn run_without_a_configuration_file_skips() {
    let project = Project::new();
    project.write("src/index.ts", WRONG_TYPE);

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert!(
        reason.get().contains("tsconfig.json"),
        "expected the reason to name the configuration file, got {reason:?}"
    );
}

// checktypescript[verify skip.uninhabited]
#[tokio::test]
async fn run_whose_configuration_selected_nothing_skips() {
    let project = Project::new();
    project.write("tsconfig.json", ELSEWHERE);
    project.write("src/index.ts", WRONG_TYPE);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Skipped { .. }),
        "expected the run to skip, got {outcome:?}"
    );
}

// checktypescript[verify tool.missing]
#[tokio::test]
async fn run_without_a_tsc_stops() {
    let project = Project::without_tsc();
    project.write("tsconfig.json", CONFIGURATION);
    project.write("src/index.ts", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Errored { .. }),
        "expected the run to stop, got {outcome:?}"
    );
}
