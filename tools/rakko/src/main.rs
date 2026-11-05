//! The maintenance commands of this repository
//!
//! This binary is the harness of the Rakko repository: the one place that
//! states which maintenance actions run here. It mounts the bundles and the
//! actions that this repository uses, and the command line that it builds
//! turns each of them into a command. A bundle carries a set of actions that
//! projects adopt together, so the harness names the bundle instead of each
//! action in it.
//!
//! It also mounts commands, for a maintenance activity that no single action
//! describes. A command comes from a crate, as an action does, and this file
//! names it. When a command runs actions, this file names those actions as
//! well and gives them to the command.
//!
//! It also mounts groups, such as pre-commit. A group is a name and a list of
//! actions that the command line runs as one command. This file gives each
//! group its list. The command line takes the flags of the group from those
//! actions, and it runs them.
//!
//! Run it with `mise run rakko`, or with `rakko` where the environment
//! supplies the shortcut.

use rakko::Bundle;
use rakko_check_msrv::CheckMsrv;
use rakko_format_json::FormatJson;
use rakko_format_markdown::FormatMarkdown;
use rakko_format_rust::FormatRust;
use rakko_format_toml::FormatToml;
use rakko_format_yaml::FormatYaml;
use rakko_lint_github_actions::LintGitHubActions;
use rakko_lint_markdown::LintMarkdown;
use rakko_lint_rust::LintRust;
use rakko_lint_stiles::LintStiles;
use rakko_lint_yaml::LintYaml;
use rakko_pre_commit::PreCommit;
use rakko_set_msrv::SetMsrv;
use rakko_test_rust::TestRust;
use rakko_test_rust_docs::TestRustDocs;

/// Builds the command line of this repository and runs it
fn main() -> std::process::ExitCode {
    let pre_commit = PreCommit::new(actions_that_guard_a_commit());

    rakko_cli::builder()
        .mount(rakko_baseline::bundle())
        .mount(rakko_rust_library::bundle())
        .mount(LintStiles)
        .mount(SetMsrv::new(CheckMsrv))
        .mount(pre_commit)
        .run()
}

/// Returns the actions that guard a commit
///
/// The list is shorter than the list that the harness mounts. An action such
/// as `check-latest-deps`, which resolves the dependencies of the project
/// again, belongs in a scheduled job and not in front of every commit.
///
/// The pre-commit command runs the actions that write before the actions that
/// read, so the list needs no order between the two. The order among the
/// formatters is the one in which this repository formatted before the
/// pre-commit command existed. The formatters overlap in the files that they
/// write, so the last formatter that writes a file decides its content.
///
/// The command takes its flags from these actions. The formatters declare
/// `fix`, so `pre-commit --fix` reaches them and no other action.
fn actions_that_guard_a_commit() -> Bundle {
    Bundle::default()
        .with(FormatJson)
        .with(FormatMarkdown)
        .with(FormatYaml)
        .with(FormatToml)
        .with(FormatRust)
        .with(LintGitHubActions)
        .with(LintMarkdown)
        .with(LintRust)
        .with(LintStiles)
        .with(LintYaml)
        .with(TestRust)
        .with(TestRustDocs)
}
