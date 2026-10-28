use std::future::Future;

use clawless::CommandResult;
use clawless::context::Context as ClawlessContext;
use rakko_action::{Context, ErasedAction, Name};

use crate::bound_member::BoundMember;

/// A command that runs a list of actions and takes its flags from them
///
/// Some maintenance activities of a project run a list of actions, such as the
/// actions that guard a commit. These actions are the members of the group. A
/// user who passes a flag to such a command wants the flag to reach the
/// members. A group therefore gives its members to the projection, and the
/// projection derives the flags of the group from the arguments that the
/// members declare. A flag reaches every member that declares its argument,
/// and no other member. A group has no arguments of its own, so it defines no
/// flag.
///
/// The help of a flag shows one line for each member that declares the
/// argument, in the order of the members. Each line starts with the name of
/// the member and continues with the sentence that the member wrote for the
/// argument.
///
/// The projection binds every member to the values of the run before the run
/// of the group starts. When a member cannot bind its values, the group does
/// not run, and the error gives the member and the argument. Otherwise the
/// group receives one [`BoundMember`] for each member, in the order of the
/// members. The group decides how it runs them and what it reports, and it
/// can ask each of them how its run uses the project before it runs any of
/// them.
///
/// A command with arguments of its own is a [`Command`], and it calls the
/// actions that it runs as libraries.
///
/// The framework drives a group on a runtime of its own and can move the run
/// to a different thread. The trait therefore requires [`Send`] and [`Sync`]
/// of the group, and [`run`] returns a future that is [`Send`].
///
/// # Examples
///
/// The trait declares [`run`] with an explicit return type, but an
/// implementation writes a plain `async fn`:
///
/// ```
/// use rakko_action::{Context, ErasedAction, Name, action_name};
/// use rakko_cli::clawless::CommandResult;
/// use rakko_cli::clawless::context::Context as ClawlessContext;
/// use rakko_cli::{BoundMember, Group, Report};
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
///
///     async fn run(
///         &self,
///         project: &Context,
///         clawless: &ClawlessContext,
///         members: Vec<BoundMember<'_>>,
///     ) -> CommandResult {
///         for member in members {
///             let outcome = member.action().run(project).await;
///             let report = Report::new(member.name().clone(), outcome);
///
///             clawless.output().artifact(report).await?;
///         }
///
///         Ok(())
///     }
/// }
/// ```
///
/// [`Command`]: crate::Command
/// [`run`]: Group::run
pub trait Group: Send + Sync {
    /// Returns the name that identifies the group
    fn name(&self) -> Name;

    /// Returns the actions that the group runs, in the order of its list
    ///
    /// The projection reads the list when the harness mounts the group, to
    /// derive the flags of the group, and again at each run, to bind the
    /// members. An implementation returns the same list each time. The
    /// command line carries only the flags of the list at the mount, so a list
    /// that changes later can panic the run or leave members out of it. The
    /// order decides the order of the flags and of the lines in their help,
    /// and the order of the members that a run receives.
    fn members(&self) -> &[Box<dyn ErasedAction>];

    /// Runs the members of the group against the project that the context
    /// gives
    ///
    /// The run receives one bound member for each member of the group, in the
    /// order of [`members`]. Each of them holds the values of the arguments
    /// that the member declares, and no other value. The method returns a
    /// future, and the framework drives that future to its end.
    ///
    /// The group writes what it has to say through the output of the Clawless
    /// context, and it returns an error when it fails. The command line turns
    /// success into a clean exit code and an error into the code of a run that
    /// could not answer. It shows the error and then every cause of it from
    /// [`source`], so the message of an error states what failed and does not
    /// repeat its cause.
    ///
    /// [`members`]: Group::members
    /// [`source`]: std::error::Error::source
    fn run(
        &self,
        project: &Context,
        clawless: &ClawlessContext,
        members: Vec<BoundMember<'_>>,
    ) -> impl Future<Output = CommandResult> + Send;
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use clawless::event::{EventReceiver, event_channel};
    use clawless::output::Output;

    use super::*;

    /// A group without members that succeeds
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

        async fn run(
            &self,
            _project: &Context,
            _clawless: &ClawlessContext,
            _members: Vec<BoundMember<'_>>,
        ) -> CommandResult {
            Ok(())
        }
    }

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    /// Returns the context of Clawless that a test hands to a run
    ///
    /// The receiver travels with the context, because a run that writes
    /// output fails as soon as nothing reads it.
    fn clawless() -> (ClawlessContext, EventReceiver) {
        let (sender, receiver) = event_channel();
        let context = ClawlessContext::builder()
            .current_working_directory(Path::new("/tmp/project"))
            .output(Output::new(sender))
            .build()
            .expect("the test gives a working directory");

        (context, receiver)
    }

    /// Compiles only when the trait makes every group `Send`
    fn require_group_send<G: Group>() {
        assert_send::<G>();
    }

    /// Compiles only when the trait makes every group `Sync`
    fn require_group_sync<G: Group>() {
        assert_sync::<G>();
    }

    /// Compiles only when the trait makes the future of every run `Send`
    fn require_run_send<G: Group>(group: &G, project: &Context, clawless: &ClawlessContext) {
        fn require_send<T: Send>(_: T) {}
        require_send(group.run(project, clawless, Vec::new()));
    }

    #[test]
    fn group_is_send() {
        require_group_send::<Empty>();
    }

    #[test]
    fn group_is_sync() {
        require_group_sync::<Empty>();
    }

    #[test]
    fn run_is_send() {
        let project = Context::builder().root("/tmp/project").build();
        let (clawless, _receiver) = clawless();

        require_run_send(&Empty, &project, &clawless);
    }
}
