//! The tests of the group of the guide "Form a Group"

// A test reports a failure with a panic.
#![allow(clippy::missing_panics_doc)]

use otter_check_project::CheckProject;
use rakko::{Action, Bundle, Context, Documented, Group, Name, Outcome, action_name};

/// An action that passes, as a member of the group under test
#[derive(Documented)]
struct CheckNothing;

impl Action for CheckNothing {
    type Args = ();

    fn name(&self) -> Name {
        action_name!("check-nothing")
    }

    async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
        Outcome::Passed { summary: None }
    }
}

#[test]
fn description_is_the_first_line_of_the_doc_comment() {
    let group = CheckProject::new(Bundle::default());

    assert_eq!(
        group.description().get(),
        "Runs the checks that every project of the team passes"
    );
}

#[test]
fn members_returns_the_given_actions() {
    let group = CheckProject::new(Bundle::default().with(CheckNothing));

    assert_eq!(group.members().len(), 1);
}

#[test]
fn name_is_check_project() {
    let group = CheckProject::new(Bundle::default());

    assert_eq!(group.name().get(), "check-project");
}
