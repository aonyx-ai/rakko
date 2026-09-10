//! The maintenance commands of this repository
//!
//! This binary is the harness of the Rakko repository: the one place that
//! states which maintenance actions run here. It mounts the bundles and the
//! actions that this repository uses, and the command line that it builds
//! turns each of them into a command. A bundle carries a set of actions that
//! projects adopt together, so the harness names the bundle instead of each
//! action in it.
//!
//! It also mounts the commands that this repository writes for itself, for a
//! maintenance activity that no single action describes. Each of them lives in
//! a module of this package, and this file names it.
//!
//! Run it with `mise run rakko`, or with `rakko` where the environment
//! supplies the shortcut.

/// The command that runs the actions that guard a commit
mod pre_commit;

use rakko_action::ErasedAction;
use rakko_build_internal_docs::BuildInternalDocs;
use rakko_check_dependencies::CheckDependencies;
use rakko_check_latest_deps::CheckLatestDeps;
use rakko_check_minimal_deps::CheckMinimalDeps;
use rakko_check_msrv::CheckMsrv;
use rakko_check_specs::CheckSpecs;
use rakko_check_unused_deps::CheckUnusedDeps;
use rakko_cli::ErasedCommand;
use rakko_format_rust::FormatRust;
use rakko_lint_rust::LintRust;
use rakko_test_rust::TestRust;
use rakko_test_rust_docs::TestRustDocs;

use self::pre_commit::PreCommit;

/// Builds the command line of this repository and runs it
///
/// The call ends the process itself, so a harness stays a `main` that names
/// what the repository mounts and returns nothing.
fn main() {
    rakko_cli::builder()
        .mount(rakko_baseline::bundle())
        .mount([
            Box::new(BuildInternalDocs) as Box<dyn ErasedAction>,
            Box::new(CheckDependencies),
            Box::new(CheckLatestDeps),
            Box::new(CheckMinimalDeps),
            Box::new(CheckMsrv),
            Box::new(CheckSpecs),
            Box::new(CheckUnusedDeps),
            Box::new(FormatRust),
            Box::new(LintRust),
            Box::new(TestRust),
            Box::new(TestRustDocs),
        ])
        .mount_commands([Box::new(PreCommit) as Box<dyn ErasedCommand>])
        .run();
}
