use std::future::Future;
use std::pin::Pin;

use clawless::CommandResult;
use clawless::context::Context as ClawlessContext;
use rakko_action::{Context, ErasedAction, Name};

use crate::bound_member::BoundMember;
use crate::group::Group;

/// One group behind an interface that hides its type
///
/// The [`Group`] trait is written for the author of a group, so its run
/// returns a future of a type that the implementation chooses. The builder
/// holds many groups at once, beside the actions and the commands, and cannot
/// use a different type for each of them, so it holds erased groups instead,
/// the way it holds erased commands.
///
/// An erased group answers everything that the projection needs: the name that
/// identifies it, the members whose arguments become its flags, and a run that
/// takes the bound members.
///
/// Every group has an erased view, because this trait is implemented for all
/// of them. A harness never writes this implementation. It uses the trait
/// once, where it boxes the groups that it mounts.
///
/// # Examples
///
/// ```
/// use rakko_action::{Context, ErasedAction, Name, action_name};
/// use rakko_cli::clawless::CommandResult;
/// use rakko_cli::clawless::context::Context as ClawlessContext;
/// use rakko_cli::{BoundMember, ErasedGroup, Group};
///
/// struct CheckAll;
///
/// impl Group for CheckAll {
///     fn name(&self) -> Name {
///         action_name!("check-all")
///     }
///
///     fn members(&self) -> &[Box<dyn ErasedAction>] {
///         &[]
///     }
///
///     async fn run(
///         &self,
///         _project: &Context,
///         _clawless: &ClawlessContext,
///         _members: Vec<BoundMember<'_>>,
///     ) -> CommandResult {
///         Ok(())
///     }
/// }
///
/// let groups: Vec<Box<dyn ErasedGroup>> = vec![Box::new(CheckAll)];
///
/// assert_eq!(groups[0].name().get(), "check-all");
/// ```
pub trait ErasedGroup: Send + Sync {
    /// Returns the name that identifies the group
    fn name(&self) -> Name;

    /// Returns the actions that the group runs, in the order of its list
    fn members(&self) -> &[Box<dyn ErasedAction>];

    /// Runs the members of the group against the project that the context
    /// gives
    ///
    /// The run receives one bound member for each member of the group, in the
    /// order of [`members`][ErasedGroup::members].
    fn run<'a>(
        &'a self,
        project: &'a Context,
        clawless: &'a ClawlessContext,
        members: Vec<BoundMember<'a>>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>>;
}

impl<G: Group> ErasedGroup for G {
    fn name(&self) -> Name {
        Group::name(self)
    }

    fn members(&self) -> &[Box<dyn ErasedAction>] {
        Group::members(self)
    }

    fn run<'a>(
        &'a self,
        project: &'a Context,
        clawless: &'a ClawlessContext,
        members: Vec<BoundMember<'a>>,
    ) -> Pin<Box<dyn Future<Output = CommandResult> + Send + 'a>> {
        Box::pin(Group::run(self, project, clawless, members))
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;
    use std::pin::pin;
    use std::sync::Mutex;
    use std::task::{Context as TaskContext, Poll, Waker};

    use clawless::event::{EventReceiver, event_channel};
    use clawless::output::Output;
    use rakko_action::{Action, ArgsValues, Outcome, action_name};

    use super::*;

    /// A group that records the names of the members that its run received
    struct Roster {
        /// The actions that the group runs
        actions: Vec<Box<dyn ErasedAction>>,
        /// The names of the members that a run received
        received: Mutex<Vec<Name>>,
    }

    impl Group for Roster {
        fn name(&self) -> Name {
            action_name!("roster")
        }

        fn members(&self) -> &[Box<dyn ErasedAction>] {
            &self.actions
        }

        async fn run(
            &self,
            _project: &Context,
            _clawless: &ClawlessContext,
            members: Vec<BoundMember<'_>>,
        ) -> CommandResult {
            let mut received = self.received.lock().expect("the test holds the lock alone");
            received.extend(members.iter().map(|member| member.name().clone()));

            Ok(())
        }
    }

    /// An action that reads no arguments and passes
    struct Probe;

    impl Action for Probe {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("probe")
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
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

    /// Compiles only when every group has an erased view
    fn erase<G: Group + 'static>(group: G) -> Box<dyn ErasedGroup> {
        Box::new(group)
    }

    /// Returns a group with one member
    fn roster() -> Roster {
        Roster {
            actions: vec![Box::new(Probe)],
            received: Mutex::new(Vec::new()),
        }
    }

    #[test]
    fn erased_group_is_send() {
        assert_send::<Box<dyn ErasedGroup>>();
    }

    #[test]
    fn erased_group_is_sync() {
        assert_sync::<Box<dyn ErasedGroup>>();
    }

    #[test]
    fn every_group_has_an_erased_view() {
        let group = erase(roster());

        assert_eq!(group.name().get(), "roster");
    }

    #[test]
    fn members_returns_the_members_of_the_group() {
        let group: &dyn ErasedGroup = &roster();

        let names: Vec<Name> = group.members().iter().map(|member| member.name()).collect();

        assert_eq!(names, [action_name!("probe")]);
    }

    #[test]
    fn run_gives_the_group_the_members_that_it_receives() {
        let group = roster();
        let project = Context::builder().root("/tmp/project").build();
        let (clawless, _receiver) = clawless();
        let member =
            BoundMember::bind(&Probe, &ArgsValues::empty()).expect("the action reads no arguments");
        let erased: &dyn ErasedGroup = &group;
        let mut future = pin!(erased.run(&project, &clawless, vec![member]));

        let Poll::Ready(Ok(())) = future
            .as_mut()
            .poll(&mut TaskContext::from_waker(Waker::noop()))
        else {
            panic!("expected the run to succeed without waiting");
        };

        let received = group
            .received
            .lock()
            .expect("the test holds the lock alone")
            .clone();
        assert_eq!(received, [action_name!("probe")]);
    }
}
