//! The example group of the guide "Form a Group"
//!
//! A group is a named list of actions, its members, that the command line of
//! a harness runs as one command. This crate holds a group whose members the
//! harness chooses, so the same group fits projects with other actions.
//!
//! The site shows the regions of this file between the `// region:` and the
//! `// endregion:` comments. Cargo compiles and tests the file, so the code on
//! the site always compiles against the current `rakko` crate.

// region: group
use rakko::{Bundle, Documented, ErasedAction, Group, Name, action_name};

/// Runs the checks that every project of the team passes
///
/// The harness gives the group its members. The command line runs them as one
/// command, `check-project`.
#[derive(Debug, Documented)]
pub struct CheckProject {
    /// The actions that the group runs, in the order of the list
    members: Bundle,
}

impl CheckProject {
    /// Returns the group with the given members
    pub fn new(members: Bundle) -> Self {
        Self { members }
    }
}

impl Group for CheckProject {
    fn name(&self) -> Name {
        action_name!("check-project")
    }

    fn members(&self) -> &[Box<dyn ErasedAction>] {
        self.members.actions()
    }
}
// endregion: group
