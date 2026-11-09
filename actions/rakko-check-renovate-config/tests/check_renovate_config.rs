//! Tests that drive the action against real projects
//!
//! Each test builds a project in a temporary directory and runs the action
//! against it. So no fixture with a deliberately broken configuration sits in
//! this repository, where Renovate and the checks of the repository itself
//! would read it.
//!
//! The tests run the validator that the action declares. A project copies the
//! `mise.toml` of the repository and trusts it. So the Node that answers is
//! the one that the repository installs. The copy pins another Renovate that
//! nothing installs. So a test passes only when the run ignores the pin of
//! the project.

#![allow(
    clippy::expect_used,
    reason = "An assertion in a test panics by design, and the helpers of this file exist only for tests. The lints that guard production code do not apply."
)]
#![allow(clippy::missing_panics_doc)]

use std::path::Path;
use std::process::Command;

use rakko::{Action, Args, Context, Finding, Location, Outcome, Position, ProjectRoot, Summary};
use rakko_check_renovate_config::{CheckRenovateConfig, CheckRenovateConfigError};
use rakko_test_utils::{path_text, run_checking_access};
use rakko_tool::ResolveRequirementError;
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

/// The key that pins Renovate in the mise configuration of a project
const TOOL_PIN: &str = "\"npm:renovate\"";

/// The header of the table of tools in a mise configuration
const TOOLS_TABLE: &str = "[tools]\n";

/// A project that a test builds in a temporary directory
struct Project {
    /// The directory that holds the project
    directory: TempDir,
}

#[bon::bon]
impl Project {
    /// Creates a project with the Node of this repository and a Renovate that
    /// nothing installed
    ///
    /// The project copies the `mise.toml` of this repository, so the Node that
    /// mise resolves for it is the Node that the repository pins and installs.
    /// The copy pins Renovate at a version that no machine installs. The run
    /// resolves the validator that the action declares, so the pin of the
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
            format!("{head}{TOOLS_TABLE}{TOOL_PIN} = \"0.0.1\"\n{tail}"),
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
    async fn run(&self) -> Outcome {
        CheckRenovateConfig.run(&self.context(), &()).await
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

impl Drop for Project {
    /// Withdraws the trust that the project received
    ///
    /// Mise records the trust of a configuration in the user state. The
    /// temporary directory of a test never comes back. So the record would pile up
    /// there forever. A project that never wrote a configuration has nothing to
    /// withdraw. Mise resolves a path that is not there to the directory above
    /// it. So the file has to exist first. A failure stays quiet because the test
    /// already reported what it was about.
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

#[test]
fn action_declares_no_argument() {
    let schema = <<CheckRenovateConfig as Action>::Args as Args>::schema();

    assert!(
        schema.arguments().is_empty(),
        "expected no argument, got {:?}",
        schema.arguments()
    );
}

#[test]
fn action_identifies_itself_as_check_renovate_config() {
    let name = CheckRenovateConfig.name();

    assert_eq!(name.get(), "check-renovate-config");
}

#[tokio::test]
async fn run_in_a_missing_directory_stops_without_a_validator() {
    let parent = tempfile::tempdir().expect("the test creates a temporary directory");
    let missing = parent.path().join("missing");
    let context = Context::builder().root(missing.as_path()).build();

    let outcome = CheckRenovateConfig.run(&context, &()).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(source.downcast_ref(), Some(CheckRenovateConfigError::UnresolvedTool { .. }))
        ),
        "expected the run to stop without a validator, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_with_node_between_the_releases_of_renovate_stops_on_node() {
    let directory = tempfile::tempdir().expect("the test creates a temporary directory");
    let pins = directory.path().join("mise.toml");
    std::fs::write(&pins, "[tools]\nnode = \"25.0.0\"\n")
        .expect("the test writes the mise.toml of the project");
    trust(&pins);
    let context = Context::builder().root(directory.path()).build();

    let outcome = CheckRenovateConfig.run(&context, &()).await;

    let _ = Command::new("mise")
        .arg("trust")
        .arg("--quiet")
        .arg("--untrust")
        .arg(&pins)
        .status();
    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(CheckRenovateConfigError::UnresolvedTool {
                        source: ResolveRequirementError::VersionOutOfRange { tool, .. },
                    }) if tool.get() == "node"
                )
        ),
        "expected the run to stop on Node 25.0.0, which Renovate does not support, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_in_a_project_without_node_stops_on_node() {
    let directory = tempfile::tempdir().expect("the test creates a temporary directory");
    let context = Context::builder().root(directory.path()).build();

    let outcome = CheckRenovateConfig.run(&context, &()).await;

    assert!(
        matches!(
            &outcome,
            Outcome::Errored { source }
                if matches!(
                    source.downcast_ref(),
                    Some(CheckRenovateConfigError::UnresolvedTool {
                        source: ResolveRequirementError::UnresolvedToolchain { tool, .. },
                    }) if tool.get() == "node"
                )
        ),
        "expected the run to stop without a Node, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_leaves_a_configuration_with_a_problem_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("renovate.json")
        .content(MIGRATION)
        .call();

    project.run().await;

    assert_eq!(project.read("renovate.json"), MIGRATION);
}

#[tokio::test]
async fn run_with_a_denied_header_reports_the_error_at_the_file() {
    let project = Project::new();
    project
        .write()
        .path("renovate.json")
        .content(DENIED_HEADER)
        .call();

    let outcome = project.run().await;

    assert_eq!(paths(findings(&outcome)), [path_text("renovate.json")]);
}

#[tokio::test]
async fn run_with_a_file_named_like_the_global_configuration_reports_it() {
    let project = Project::new();
    project
        .write()
        .path("config.js")
        .content(UNRELATED_GLOBAL_CONFIGURATION)
        .call();

    let outcome = project.run().await;

    assert_eq!(
        messages(findings(&outcome)),
        ["Configuration Error: Invalid configuration option: port"]
    );
}

#[tokio::test]
async fn run_with_a_global_configuration_that_does_not_parse_stops() {
    let project = Project::new();
    project.write().path("renovate.json").content(VALID).call();
    project
        .write()
        .path("config.js")
        .content(BROKEN_GLOBAL_CONFIGURATION)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(
            error(&outcome),
            CheckRenovateConfigError::AbortedValidation { .. }
        ),
        "expected the validator to stop, got {outcome:?}"
    );
}

/// Shows that a run with a global configuration that needs migration passes
///
/// The validator migrates the global configuration before it validates it. It
/// fails no run for the migration. So the action passes as the validator does.
/// The action keeps this limit of the validator.
#[tokio::test]
async fn run_with_a_global_configuration_that_needs_a_migration_passes() {
    let project = Project::new();
    project
        .write()
        .path("config.js")
        .content(GLOBAL_MIGRATION)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass as the validator does, got {outcome:?}"
    );
}

/// Shows that the run leaves every file of the project as it was
///
/// The access check fails the test when the run changes a file of the project.
#[tokio::test]
async fn run_with_a_needed_migration_leaves_the_project_unchanged() {
    let project = Project::new();
    project
        .write()
        .path("renovate.json")
        .content(MIGRATION)
        .call();

    run_checking_access(&CheckRenovateConfig, &project.context(), &()).await;
}

#[tokio::test]
async fn run_with_a_needed_migration_names_the_option_that_it_changes() {
    let project = Project::new();
    project
        .write()
        .path("renovate.json")
        .content(MIGRATION)
        .call();

    let outcome = project.run().await;

    assert_eq!(
        messages(findings(&outcome)),
        ["Config migration necessary: extends"]
    );
}

#[tokio::test]
async fn run_with_a_preset_and_a_configuration_counts_both() {
    let project = Project::new();
    project
        .write()
        .path(".github/renovate.json")
        .content(VALID)
        .call();
    project.write().path("default.json").content(VALID).call();

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("validated 2 configurations")
    );
}

#[tokio::test]
async fn run_with_a_preset_that_needs_a_migration_reports_the_preset() {
    let project = Project::new();
    project
        .write()
        .path("default.json")
        .content(MIGRATION)
        .call();

    let outcome = project.run().await;

    assert_eq!(paths(findings(&outcome)), [path_text("default.json")]);
}

#[tokio::test]
async fn run_with_a_problem_in_package_json_reports_package_json() {
    let project = Project::new();
    project
        .write()
        .path("package.json")
        .content(PACKAGE_WITH_UNKNOWN_OPTION)
        .call();

    let outcome = project.run().await;

    assert_eq!(paths(findings(&outcome)), [path_text("package.json")]);
}

#[tokio::test]
async fn run_with_a_problem_in_the_preset_reports_the_preset() {
    let project = Project::new();
    project
        .write()
        .path("default.json")
        .content(UNKNOWN_OPTION)
        .call();

    let outcome = project.run().await;

    assert_eq!(paths(findings(&outcome)), [path_text("default.json")]);
}

#[tokio::test]
async fn run_with_a_syntax_error_reports_the_position_of_the_error() {
    let project = Project::new();
    project
        .write()
        .path("renovate.json")
        .content(SYNTAX_ERROR)
        .call();

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

#[tokio::test]
async fn run_with_a_valid_configuration_passes() {
    let project = Project::new();
    project
        .write()
        .path(".github/renovate.json")
        .content(VALID)
        .call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_an_unknown_option_reports_the_error_at_the_file() {
    let project = Project::new();
    project
        .write()
        .path("renovate.json")
        .content(UNKNOWN_OPTION)
        .call();

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

#[tokio::test]
async fn run_with_only_a_valid_preset_passes() {
    let project = Project::new();
    project.write().path("default.json").content(VALID).call();

    let outcome = project.run().await;

    assert!(
        matches!(outcome, Outcome::Passed { .. }),
        "expected the run to pass, got {outcome:?}"
    );
}

#[tokio::test]
async fn run_with_only_a_warning_fails_with_the_warning() {
    let project = Project::new();
    project
        .write()
        .path("renovate.json")
        .content(WARNING)
        .call();

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

#[tokio::test]
async fn run_with_two_broken_configurations_reports_both_files() {
    let project = Project::new();
    project
        .write()
        .path(".github/renovate.json")
        .content(UNKNOWN_OPTION)
        .call();
    project
        .write()
        .path("renovate.json5")
        .content(UNKNOWN_OPTION)
        .call();

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

#[tokio::test]
async fn run_with_two_presets_in_package_json_counts_both() {
    let project = Project::new();
    project
        .write()
        .path("package.json")
        .content(PACKAGE_WITH_PRESETS)
        .call();

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("validated 2 configurations")
    );
}

#[tokio::test]
async fn run_with_two_valid_configurations_names_both_in_the_summary() {
    let project = Project::new();
    project
        .write()
        .path(".github/renovate.json")
        .content(VALID)
        .call();
    project.write().path("renovate.json5").content(VALID).call();

    let outcome = project.run().await;

    let Outcome::Passed { summary } = &outcome else {
        panic!("expected the run to pass, got {outcome:?}");
    };
    assert_eq!(
        summary.as_ref().map(Summary::get),
        Some("validated 2 configurations")
    );
}

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
