use crate::erased_action::ErasedAction;
use crate::name::Name;

/// A named list of actions that the command line runs as one command
///
/// Some maintenance activities of a project run a list of actions, such as the
/// actions that guard a commit. These actions are the members of the group. A
/// group gives its name and its members, and nothing else. It has no run of
/// its own, and it has no arguments of its own.
///
/// The `rakko-cli` crate builds the command line of a harness. A harness
/// mounts a group there with the `mount_groups` method of the builder. The
/// command line gives the group a command with the name of the group. It
/// derives the flags of that command from the arguments that the members
/// declare, and it runs the members itself. The documentation of
/// `mount_groups` describes the flags and the run.
///
/// A command with arguments of its own, or a command that runs its actions in
/// a way of its own, is not a group. A harness writes such a command with the
/// `Command` trait of `rakko-cli`. The command calls the actions that it runs
/// as libraries.
///
/// The command line runs a group on a runtime of its own and can move the run
/// to a different thread. The trait therefore requires [`Send`] and [`Sync`]
/// of the group.
///
/// # Examples
///
/// A harness, or a crate that provides a group for a harness, forms the group
/// from a name and a list of actions:
///
/// ```
/// use rakko_action::{ErasedAction, Group, Name, action_name};
///
/// struct CheckAll {
///     actions: Vec<Box<dyn ErasedAction>>,
/// }
///
/// impl Group for CheckAll {
///     fn name(&self) -> Name {
///         action_name!("check-all")
///     }
///
///     fn members(&self) -> &[Box<dyn ErasedAction>] {
///         &self.actions
///     }
/// }
///
/// let check_all = CheckAll {
///     actions: Vec::new(),
/// };
///
/// // The `mount_groups` method of `rakko-cli` takes the group in a box.
/// let group: Box<dyn Group> = Box::new(check_all);
/// #
/// # assert_eq!(group.name().get(), "check-all");
/// ```
pub trait Group: Send + Sync {
    /// Returns the name that identifies the group
    fn name(&self) -> Name;

    /// Returns the members of the group, in the order of its list
    ///
    /// The command line reads the list when the harness mounts the group, to
    /// derive the flags of the group, and again at each run, to bind and run
    /// the members. An implementation returns the same list each time. The
    /// command line carries only the flags of the list at the mount, so a list
    /// that changes later can panic the run or leave members out of it. The
    /// order decides the order of the flags and of the lines in their help.
    /// It also orders the run, as the documentation of `mount_groups`
    /// describes. Two members of the list must not have the same name. The
    /// harness stops at the mount of a group whose list breaks this rule.
    fn members(&self) -> &[Box<dyn ErasedAction>];
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// A group without members
    struct Empty;

    impl Group for Empty {
        fn name(&self) -> Name {
            "empty"
                .parse()
                .expect("the name of the test group is valid")
        }

        fn members(&self) -> &[Box<dyn ErasedAction>] {
            &[]
        }
    }

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    /// Compiles only when the trait makes every group `Send`
    fn require_group_send<G: Group>() {
        assert_send::<G>();
    }

    /// Compiles only when the trait makes every group `Sync`
    fn require_group_sync<G: Group>() {
        assert_sync::<G>();
    }

    #[test]
    fn group_is_send() {
        require_group_send::<Empty>();
    }

    #[test]
    fn group_is_sync() {
        require_group_sync::<Empty>();
    }
}
