//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it, so no fixture with a deliberately broken configuration sits in
//! this repository, where Renovate and the checks of the repository itself
//! would read it.
//!
//! The tests run the validator that this repository pins. A project copies the
//! `mise.toml` of the repository and trusts it, so the version that answers is
//! the version that the repository installs, and a new pin reaches the tests
//! without a change to them.

// An assertion in a test panics by design, and the helpers of this file exist
// only for tests. The lints that guard production code do not apply.
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko_action::{
    Action, Args, Context, Finding, Location, Outcome, Position, ProjectRoot, Summary,
};
use rakko_check_renovate_config::{CheckRenovateConfig, CheckRenovateConfigError};
use rakko_test_utils::path_text;
use tempfile::TempDir;

/// A configuration that Renovate accepts
const VALID: &str = "{\"extends\": [\"config:recommended\"]}\n";

/// A configuration with an option that Renovate does not know
const UNKNOWN_OPTION: &str = "{\"foo\": 1}\n";

/// A configuration that Renovate accepts with a warning
///
/// Renovate warns about a registry at the top of a configuration, because it
/// applies to every manager and every datasource.
const WARNING: &str = "{\"registryUrls\": [\"https://registry.example.invalid\"]}\n";

/// A configuration that sends a header that Renovate does not allow by default
const DENIED_HEADER: &str = "{\"hostRules\": [{\"matchHost\": \"example.com\", \"headers\": \
                             {\"Authorization\": \"token\"}}]}\n";

/// A configuration that extends a preset that a later release renamed
const MIGRATION: &str = "{\"extends\": [\"config:base\"]}\n";

/// A configuration that ends in the middle of an array
const SYNTAX_ERROR: &str = "{\"extends\": [\n";

/// A manifest of Node that carries a configuration of Renovate in a key
const PACKAGE_WITH_UNKNOWN_OPTION: &str = "{\"name\": \"otter\", \"renovate\": {\"foo\": 1}}\n";

/// A manifest of Node that carries two presets of Renovate in a key
const PACKAGE_WITH_PRESETS: &str = "{\"name\": \"otter\", \"renovate-config\": {\"default\": \
                                    {\"extends\": [\"config:recommended\"]}, \"strict\": \
                                    {\"automerge\": true}}}\n";

/// A file that holds an unrelated configuration under the name that Renovate
/// reads its global configuration from
const UNRELATED_GLOBAL_CONFIGURATION: &str = "module.exports = { port: 3000 };\n";

/// A global configuration of Renovate that extends a preset that a later
/// release renamed
const GLOBAL_MIGRATION: &str = "module.exports = { extends: ['config:base'] };\n";

/// A global configuration of Renovate that Node cannot parse
const BROKEN_GLOBAL_CONFIGURATION: &str = "module.exports = {\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

impl Project {
    /// Creates a project without a validator to resolve
    ///
    /// The project holds no mise configuration, so nothing in it reaches a
    /// tool. A test uses this shape when the run must end before the tool
    /// runs.
    fn bare() -> Self {
        let directory = tempfile::tempdir().expect("the test creates a temporary directory");

        Self { directory }
    }

    /// Creates a project with the validator of this repository
    ///
    /// The project copies the `mise.toml` of this repository, so the validator
    /// that mise resolves for it is the validator that the repository pins and
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

    /// Creates a project that pins a validator that nothing installed
    ///
    /// The pin names a version that no machine installs, so mise reports no
    /// location for it, whatever the global configuration of the machine says
    /// about Renovate.
    fn without_validator() -> Self {
        let project = Self::bare();

        let pins = project.directory.path().join("mise.toml");
        std::fs::write(&pins, "[tools]\n\"npm:renovate\" = \"0.0.1\"\n")
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

    /// Returns the content of a file of the project
    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.directory.path().join(path))
            .expect("the test reads a file that it wrote")
    }

    /// Runs the action against this project
    async fn run(&self) -> Outcome {
        CheckRenovateConfig.run(&self.context(), &()).await
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

/// Returns the error of a run that stopped
fn error(outcome: &Outcome) -> &CheckRenovateConfigError {
    let Outcome::Errored { source } = outcome else {
        panic!("expected the run to stop, got {outcome:?}");
    };

    source
        .downcast_ref()
        .expect("the action stops with its own error")
}

/// Returns the findings of a run that failed
fn findings(outcome: &Outcome) -> &[Finding] {
    let Outcome::Failed { findings, .. } = outcome else {
        panic!("expected the run to fail, got {outcome:?}");
    };

    findings
}

/// Returns the messages of the given findings
fn messages(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .map(|finding| finding.message().get())
        .collect()
}

/// Returns the paths that the findings of an outcome name
fn paths(findings: &[Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| match finding.location() {
            Location::File { path } | Location::Position { path, .. } => path.to_string(),
            other => panic!("expected a finding in a file, got {other:?}"),
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

// checkrenovateconfig[verify args.none]
#[test]
fn action_declares_no_argument() {
    let schema = <<CheckRenovateConfig as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

// checkrenovateconfig[verify name]
#[test]
fn action_identifies_itself_as_check_renovate_config() {
    let name = CheckRenovateConfig.name();

    assert_eq!(name.get(), "check-renovate-config");
}

// checkrenovateconfig[verify check.read]
#[tokio::test]
async fn run_leaves_a_configuration_with_a_problem_unchanged() {
    let project = Project::new();
    project.write("renovate.json", MIGRATION);

    project.run().await;

    assert_eq!(project.read("renovate.json"), MIGRATION);
}

// checkrenovateconfig[verify check.error]
#[tokio::test]
async fn run_with_a_denied_header_reports_the_error_at_the_file() {
    let project = Project::new();
    project.write("renovate.json", DENIED_HEADER);

    let outcome = project.run().await;

    assert_eq!(paths(findings(&outcome)), [path_text("renovate.json")]);
}

// checkrenovateconfig[verify check.error]
// checkrenovateconfig[verify run.project]
#[tokio::test]
async fn run_with_a_file_named_like_the_global_configuration_reports_it() {
    let project = Project::new();
    project.write("config.js", UNRELATED_GLOBAL_CONFIGURATION);

    let outcome = project.run().await;

    assert_eq!(
        messages(findings(&outcome)),
        ["Configuration Error: Invalid configuration option: port"]
    );
}

// checkrenovateconfig[verify check.aborted]
#[tokio::test]
async fn run_with_a_global_configuration_that_does_not_parse_stops() {
    let project = Project::new();
    project.write("renovate.json", VALID);
    project.write("config.js", BROKEN_GLOBAL_CONFIGURATION);

    let outcome = project.run().await;

    assert!(
        matches!(
            error(&outcome),
            CheckRenovateConfigError::AbortedValidation { .. }
        ),
        "expected the validator to stop, got {outcome:?}"
    );
}

// The validator migrates the global configuration before it validates it, and
// it fails no run for the migration, so the action passes as the validator
// does. The specification states this limit.
#[tokio::test]
async fn run_with_a_global_configuration_that_needs_a_migration_passes() {
    let project = Project::new();
    project.write("config.js", GLOBAL_MIGRATION);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass as the validator does, got {outcome:?}"
    );
}

// checkrenovateconfig[verify check.migration]
#[tokio::test]
async fn run_with_a_needed_migration_names_the_option_that_it_changes() {
    let project = Project::new();
    project.write("renovate.json", MIGRATION);

    let outcome = project.run().await;

    assert_eq!(
        messages(findings(&outcome)),
        ["Config migration necessary: extends"]
    );
}

// checkrenovateconfig[verify check.embedded]
#[tokio::test]
async fn run_with_a_problem_in_package_json_reports_package_json() {
    let project = Project::new();
    project.write("package.json", PACKAGE_WITH_UNKNOWN_OPTION);

    let outcome = project.run().await;

    assert_eq!(paths(findings(&outcome)), [path_text("package.json")]);
}

// checkrenovateconfig[verify check.unparsable]
#[tokio::test]
async fn run_with_a_syntax_error_reports_the_position_of_the_error() {
    let project = Project::new();
    project.write("renovate.json", SYNTAX_ERROR);

    let outcome = project.run().await;

    assert_eq!(
        findings(&outcome)
            .iter()
            .map(Finding::location)
            .collect::<Vec<_>>(),
        [&Location::Position {
            path: "renovate.json"
                .parse()
                .expect("the test names a relative path"),
            position: Position::builder().line(2).column(1).build(),
        }]
    );
}

// checkrenovateconfig[verify check.passed]
// checkrenovateconfig[verify tool.validator]
#[tokio::test]
async fn run_with_a_valid_configuration_passes() {
    let project = Project::new();
    project.write(".github/renovate.json", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

// checkrenovateconfig[verify check.error]
#[tokio::test]
async fn run_with_an_unknown_option_reports_the_error_at_the_file() {
    let project = Project::new();
    project.write("renovate.json", UNKNOWN_OPTION);

    let outcome = project.run().await;

    assert_eq!(
        findings(&outcome)
            .iter()
            .map(|finding| (finding.message().get(), finding.location()))
            .collect::<Vec<_>>(),
        [(
            "Configuration Error: Invalid configuration option: foo",
            &Location::File {
                path: "renovate.json"
                    .parse()
                    .expect("the test names a relative path"),
            }
        )]
    );
}

// checkrenovateconfig[verify check.warning]
#[tokio::test]
async fn run_with_only_a_warning_fails_with_the_warning() {
    let project = Project::new();
    project.write("renovate.json", WARNING);

    let outcome = project.run().await;

    assert_eq!(
        findings(&outcome)
            .iter()
            .map(|finding| (
                finding
                    .message()
                    .get()
                    .starts_with("Configuration Warning: Setting `registryUrls`"),
                finding.location()
            ))
            .collect::<Vec<_>>(),
        [(
            true,
            &Location::File {
                path: "renovate.json"
                    .parse()
                    .expect("the test names a relative path"),
            }
        )]
    );
}

// checkrenovateconfig[verify run.project]
#[tokio::test]
async fn run_with_two_broken_configurations_reports_both_files() {
    let project = Project::new();
    project.write(".github/renovate.json", UNKNOWN_OPTION);
    project.write("renovate.json5", UNKNOWN_OPTION);

    let outcome = project.run().await;

    let mut paths = paths(findings(&outcome));
    paths.sort();
    assert_eq!(
        paths,
        [
            path_text(".github/renovate.json"),
            path_text("renovate.json5")
        ]
    );
}

// checkrenovateconfig[verify check.summary]
#[tokio::test]
async fn run_with_two_presets_in_package_json_counts_both() {
    let project = Project::new();
    project.write("package.json", PACKAGE_WITH_PRESETS);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("validated 2 configurations")
    );
}

// checkrenovateconfig[verify check.summary]
// checkrenovateconfig[verify run.project]
#[tokio::test]
async fn run_with_two_valid_configurations_names_both_in_the_summary() {
    let project = Project::new();
    project.write(".github/renovate.json", VALID);
    project.write("renovate.json5", VALID);

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("validated 2 configurations")
    );
}

// checkrenovateconfig[verify skip.unconfigured]
#[tokio::test]
async fn run_without_a_configuration_skips_with_the_words_of_the_validator() {
    let project = Project::new();

    let outcome = project.run().await;

    let Outcome::Skipped { reason } = &outcome else {
        panic!("expected the run to skip, got {outcome:?}");
    };
    assert_eq!(
        reason.get(),
        "No files to perform configuration validation against"
    );
}

// checkrenovateconfig[verify tool.missing]
#[tokio::test]
async fn run_without_a_validator_stops() {
    let project = Project::without_validator();
    project.write("renovate.json", VALID);

    let outcome = project.run().await;

    assert!(
        matches!(
            error(&outcome),
            CheckRenovateConfigError::UnresolvedTool { .. }
        ),
        "expected no validator, got {outcome:?}"
    );
}
