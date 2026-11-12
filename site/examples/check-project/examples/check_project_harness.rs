//! The harness of the guide "Form a Group"
//!
//! A harness is the small binary that a project runs to maintain itself. This
//! one mounts the group `check-project` with two members, and one of the
//! members as an action of its own. The two actions are small, so that the
//! file shows the whole harness. A project takes its actions from crates.

// region: main
use otter_check_project::CheckProject;
use rakko::Bundle;

/// Builds the command line of the project and runs it
fn main() -> std::process::ExitCode {
    let check_project =
        CheckProject::new(Bundle::default().with(CheckLicense).with(CheckChangelog));

    rakko_cli::builder()
        .mount(CheckLicense)
        .mount(check_project)
        .run()
}
// endregion: main

use rakko::{Access, Action, Context, Documented, Finding, Location, Name, Outcome, action_name};

/// Checks that the project has a license
#[derive(Documented)]
struct CheckLicense;

impl Action for CheckLicense {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("check-license")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        check_file(context, "LICENSE")
    }
}

/// Checks that the project has a changelog
#[derive(Documented)]
struct CheckChangelog;

impl Action for CheckChangelog {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("check-changelog")
    }

    fn access(&self, _context: &Context, _args: &Self::Args) -> Access {
        Access::Read
    }

    async fn run(&self, context: &Context, _args: &Self::Args) -> Outcome {
        check_file(context, "CHANGELOG.md")
    }
}

/// Returns the outcome of a check for a file in the root of the project
fn check_file(context: &Context, file: &str) -> Outcome {
    match context.root().get().join(file).try_exists() {
        Ok(true) => Outcome::Passed { summary: None },
        Ok(false) => Outcome::Failed {
            findings: vec![
                Finding::builder()
                    .message(format!("the project has no {file}"))
                    .location(Location::Project)
                    .build(),
            ],
            repairs: Vec::new(),
        },
        Err(error) => Outcome::Errored {
            source: Box::new(error),
        },
    }
}
