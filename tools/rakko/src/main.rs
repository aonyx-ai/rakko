//! The maintenance commands of this repository
//!
//! This binary is the harness of the Rakko repository: the one place that
//! states which maintenance actions run here. It mounts the actions that this
//! repository uses, and the command line that it builds turns each of them
//! into a command. It also names the actions that guard a commit, and the
//! `pre-commit` command drives every one of them.
//!
//! Run it with `mise run rakko`, or with `rakko` where the environment
//! supplies the shortcut.

use rakko_action::ErasedAction;
use rakko_build_internal_docs::BuildInternalDocs;
use rakko_check_dependencies::CheckDependencies;
use rakko_check_latest_deps::CheckLatestDeps;
use rakko_check_minimal_deps::CheckMinimalDeps;
use rakko_check_msrv::CheckMsrv;
use rakko_check_specs::CheckSpecs;
use rakko_check_unused_deps::CheckUnusedDeps;
use rakko_cli::Step;
use rakko_format_json::FormatJson;
use rakko_format_markdown::FormatMarkdown;
use rakko_format_rust::FormatRust;
use rakko_format_toml::FormatToml;
use rakko_format_yaml::FormatYaml;
use rakko_lint_github_actions::LintGitHubActions;
use rakko_lint_markdown::LintMarkdown;
use rakko_lint_rust::LintRust;
use rakko_lint_toml::LintToml;
use rakko_lint_yaml::LintYaml;
use rakko_test_rust::TestRust;

/// Builds the command line of this repository and runs it
///
/// The call ends the process itself, so a harness stays a `main` that names
/// what the repository mounts and returns nothing.
///
/// The steps that guard a commit run in the order below. The formatters go
/// first, because they rewrite the tree, and they run one after another,
/// because their files overlap and nothing may read a file while another
/// formatter writes it. They are the steps that get the `fix` flag of the
/// command. The checks follow, so that what they read is what the commit will
/// hold. The slow checks, which resolve dependencies or build on another
/// toolchain, have no step and run on demand.
fn main() {
    rakko_cli::builder()
        .mount([
            Box::new(BuildInternalDocs) as Box<dyn ErasedAction>,
            Box::new(CheckDependencies),
            Box::new(CheckLatestDeps),
            Box::new(CheckMinimalDeps),
            Box::new(CheckMsrv),
            Box::new(CheckSpecs),
            Box::new(CheckUnusedDeps),
            Box::new(FormatJson),
            Box::new(FormatMarkdown),
            Box::new(FormatRust),
            Box::new(FormatToml),
            Box::new(FormatYaml),
            Box::new(LintGitHubActions),
            Box::new(LintMarkdown),
            Box::new(LintRust),
            Box::new(LintToml),
            Box::new(LintYaml),
            Box::new(TestRust),
        ])
        .pre_commit([
            Step::with_fix(Box::new(FormatJson)),
            Step::with_fix(Box::new(FormatMarkdown)),
            Step::with_fix(Box::new(FormatYaml)),
            Step::with_fix(Box::new(FormatToml)),
            Step::with_fix(Box::new(FormatRust)),
            Step::new(Box::new(CheckSpecs)),
            Step::new(Box::new(LintGitHubActions)),
            Step::new(Box::new(LintMarkdown)),
            Step::new(Box::new(LintRust)),
            Step::new(Box::new(LintYaml)),
            Step::new(Box::new(TestRust)),
        ])
        .run();
}
