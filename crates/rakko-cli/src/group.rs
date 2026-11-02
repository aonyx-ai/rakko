use rakko_action::{ErasedAction, Name};

/// A named list of actions that the command line runs as one command
///
/// Some maintenance activities of a project run a list of actions, such as the
/// actions that guard a commit. These actions are the members of the group. A
/// group gives the command line its name and its members, and nothing else.
/// The command line gives the group a command with the name of the group,
/// derives the flags of that command from the members, and runs the members
/// itself.
///
/// The command line derives the flags of the group from the arguments that
/// the members declare. A flag reaches every member that declares its
/// argument, and no other member. A group has no arguments of its own, so it
/// defines no flag. The help of a flag shows one line for each member that
/// declares the argument, in the order of the members. Each line starts with
/// the name of the member and continues with the sentence that the member
/// wrote for the argument.
///
/// A run of the group binds every member to the values of the run first. When
/// a member cannot bind its values, no member runs, and the error gives the
/// member and the argument. Then the run asks every member how its run uses
/// the project, as [`Access`] declares it. When a member declares a kind of
/// access that the command line does not know, no member runs either, and the
/// error tells the user to update Rakko.
///
/// Then the members that write run, one at a time and in the order of the
/// list. A member that declares nothing writes. The members that read run
/// after them, so that they examine the project as the writers left it. They
/// also run one at a time and in the order of the list. Every member runs to
/// its end, also after another member found problems or stopped, so one run
/// reports every problem. The run reports each member when the member
/// finishes, as a run of that action alone reports. The run fails when a
/// member found problems or stopped, and the error counts these members.
///
/// A command with arguments of its own, or a command that runs its actions in
/// a way of its own, is a [`Command`]. It calls the actions that it runs as
/// libraries.
///
/// The framework runs a group on a runtime of its own and can move the run to
/// a different thread. The trait therefore requires [`Send`] and [`Sync`] of
/// the group.
///
/// # Examples
///
/// A harness forms a group from a name and a list of actions, and mounts it
/// beside its actions:
///
/// ```
/// use rakko_action::{ErasedAction, Name, action_name};
/// use rakko_cli::Group;
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
/// let command_line = rakko_cli::builder().mount_groups([Box::new(check_all) as Box<dyn Group>]);
/// ```
///
/// [`Access`]: rakko_action::Access
/// [`Command`]: crate::Command
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
    /// order decides the order of the flags and of the lines in their help,
    /// and the order in which the members that write run, and the members
    /// that read. Two members of the list must not have the same name. The
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
