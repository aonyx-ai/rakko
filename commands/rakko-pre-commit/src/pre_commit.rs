//! The group of the actions that guard a commit

use std::fmt;

use rakko_action::{ErasedAction, Group, Name, action_name};

/// The group of the actions that guard a commit
///
/// A commit of a project has to pass a list of actions, and no action
/// describes that list. The list repairs the tree first and then examines what
/// it repaired. Whoever reads it wants what each action said, where an action
/// answers with one outcome. The hook that Git runs before a commit therefore
/// calls one command that runs the whole list.
///
/// The group contains no action of its own. Which activities guard a commit is
/// a decision of the project, so the harness gives the actions to the group in
/// one list. The group is a [`Group`], and the actions in its list are its
/// members. The command line runs the members: it asks each member whether a
/// run reads or writes the project. It orders the run from these declarations.
///
/// The command line also derives the flags of the group from the arguments
/// that the members declare. So the group defines no flag of its own. A flag
/// such as `--fix` reaches every member that declares its argument, and no
/// other member.
///
/// # Examples
///
/// ```
/// use rakko_action::Group;
/// use rakko_pre_commit::PreCommit;
///
/// let group = PreCommit::new([]);
/// assert_eq!(group.name().get(), "pre-commit");
/// ```
pub struct PreCommit {
    /// The actions that guard a commit, in the order of the list that the
    /// harness gave
    actions: Vec<Box<dyn ErasedAction>>,
}

impl PreCommit {
    /// Creates the group from the actions that guard a commit
    ///
    /// The list can hold the actions in any order. A run asks each action how
    /// it uses the project, and runs every action that writes before the first
    /// action that reads. An action that declares nothing writes. The actions
    /// that write run one at a time, in the order of the list. This order
    /// matters when two actions that write touch the same files. Two
    /// formatters can do this, and so can a generator that reads the files
    /// that a formatter writes. The actions that read then all start at the
    /// same time.
    ///
    /// The list holds each action only once. A list with two actions of the
    /// same name stops the harness when it mounts the group. The list is
    /// independent of the mount, so a harness that wants a command for one of
    /// these actions mounts that action as well.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_action::Group;
    /// use rakko_pre_commit::PreCommit;
    ///
    /// let group = PreCommit::new([]);
    /// assert!(group.members().is_empty());
    /// ```
    pub fn new(actions: impl IntoIterator<Item = Box<dyn ErasedAction>>) -> Self {
        Self {
            actions: actions.into_iter().collect(),
        }
    }
}

/// Shows the names of the actions, because an erased action shows nothing
impl fmt::Debug for PreCommit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreCommit")
            .field("actions", &names(&self.actions))
            .finish()
    }
}

impl Group for PreCommit {
    fn name(&self) -> Name {
        action_name!("pre-commit")
    }

    /// Returns the actions that guard a commit, in the order of the list that
    /// the harness gave
    fn members(&self) -> &[Box<dyn ErasedAction>] {
        &self.actions
    }
}

/// Returns the names of the actions, in the order of the list
fn names(actions: &[Box<dyn ErasedAction>]) -> Vec<Name> {
    actions.iter().map(|action| action.name()).collect()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko_action::{Action, Context, Outcome};

    use super::*;

    /// An action that passes
    struct Probe {
        /// The name that the action reports
        name: Name,
    }

    impl Action for Probe {
        type Args = ();

        fn name(&self) -> Name {
            self.name.clone()
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// Returns an action with the given name
    fn probe(name: Name) -> Box<dyn ErasedAction> {
        Box::new(Probe { name })
    }

    #[test]
    fn members_are_the_actions_in_the_order_of_the_list() {
        let group = PreCommit::new([
            probe(action_name!("lint-yaml")),
            probe(action_name!("format-yaml")),
        ]);

        let members: Vec<Name> = group.members().iter().map(|member| member.name()).collect();

        assert_eq!(
            members,
            [action_name!("lint-yaml"), action_name!("format-yaml")]
        );
    }

    #[test]
    fn name_is_the_name_of_the_hook() {
        let group = PreCommit::new([]);

        assert_eq!(group.name().get(), "pre-commit");
    }

    #[test]
    fn pre_commit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<PreCommit>();
    }
}
