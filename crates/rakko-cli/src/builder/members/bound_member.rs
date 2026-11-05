//! One member of a group, bound to the values of one run

use std::fmt;

use rakko_action::{ArgsValues, BoundAction, ErasedAction, Name, ReadArgsError};

/// One member of a group, bound to the values of one run
///
/// A group lists actions, its members. The command line binds every member to
/// the values of the run before any member runs, and the scheduler receives
/// one bound member for each member of the group. A bound member keeps the
/// name of the action beside the [`BoundAction`], so that the scheduler can
/// report what each member found under the name of the member.
///
/// The values are bound once. The bound action declares how its run uses the
/// project and then runs, and both see the same arguments. So the scheduler
/// asks every member for its access before it runs the first one.
pub(crate) struct BoundMember<'a> {
    /// The name of the action
    name: Name,
    /// The action with the arguments of the run
    action: Box<dyn BoundAction + 'a>,
}

#[bon::bon]
impl<'a> BoundMember<'a> {
    /// Binds the values of a run to the arguments of an action
    ///
    /// The command line calls this function for every member of a group, with
    /// the values of the arguments that the member declares.
    ///
    /// # Errors
    ///
    /// Returns a [`ReadArgsError`] when the values do not convert into the
    /// arguments of the action.
    #[builder]
    pub(crate) fn bind(
        action: &'a dyn ErasedAction,
        values: &ArgsValues,
    ) -> Result<Self, ReadArgsError> {
        Ok(Self {
            name: action.name(),
            action: action.bind(values)?,
        })
    }

    /// Returns the name of the action
    #[must_use]
    pub(crate) fn name(&self) -> &Name {
        &self.name
    }

    /// Returns the action with the arguments of the run
    ///
    /// The bound action declares how its run uses the project and runs with
    /// the arguments that the command line bound.
    #[must_use]
    pub(crate) fn action(&self) -> &(dyn BoundAction + 'a) {
        self.action.as_ref()
    }
}

/// Shows the name of the action, because a bound action shows nothing
impl fmt::Debug for BoundMember<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundMember")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::{
        Access, Action, Args, ArgsSchema, Argument, ArgumentShape, ArgumentValue, Context, Outcome,
        action_name, argument_name,
    };

    use super::*;

    /// Whether the formatter should write
    #[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
    enum Operation {
        /// Only report what the formatter found
        #[default]
        Report,
        /// Rewrite the files that the formatter can format
        Rewrite,
    }

    /// The arguments of the formatter: whether the run repairs what it finds
    struct Fix {
        /// Whether the run rewrites the files that it can format
        operation: Operation,
    }

    impl Args for Fix {
        fn schema() -> ArgsSchema {
            ArgsSchema::new([Argument::builder()
                .name(argument_name!("fix"))
                .shape(ArgumentShape::Boolean)
                .documentation("Rewrite the files that the tool can format")
                .build()])
        }

        fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
            match values.get(&argument_name!("fix")) {
                Some(value) if value.get() == "true" => Ok(Self {
                    operation: Operation::Rewrite,
                }),
                Some(value) => Err(ReadArgsError::UnreadableValue {
                    name: argument_name!("fix"),
                    value: value.clone(),
                }),
                None => Ok(Self {
                    operation: Operation::Report,
                }),
            }
        }
    }

    /// An action that writes only when its arguments ask for a repair
    struct Formatter;

    impl Action for Formatter {
        type Args = Fix;

        fn name(&self) -> Name {
            action_name!("formatter")
        }

        fn access(&self, _context: &Context, args: &Self::Args) -> Access {
            match args.operation {
                Operation::Rewrite => Access::Write,
                Operation::Report => Access::Read,
            }
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// Returns the values of a run that gives `fix` the given text
    fn fix(value: &str) -> ArgsValues {
        ArgsValues::new([(argument_name!("fix"), ArgumentValue::new(value))])
    }

    #[test]
    fn action_declares_the_access_of_the_values_that_were_bound() {
        let member = BoundMember::bind()
            .action(&Formatter)
            .values(&fix("true"))
            .call()
            .unwrap();
        let context = Context::builder().root("/tmp/project").build();

        let access = member.action().access(&context);

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn bind_reports_values_that_do_not_convert() {
        let member = BoundMember::bind()
            .action(&Formatter)
            .values(&fix("maybe"))
            .call();

        assert!(matches!(
            member,
            Err(ReadArgsError::UnreadableValue { name, .. }) if name.get() == "fix"
        ));
    }

    #[test]
    fn bound_member_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<BoundMember<'static>>();
    }

    #[test]
    fn name_returns_the_name_of_the_action() {
        let member = BoundMember::bind()
            .action(&Formatter)
            .values(&ArgsValues::empty())
            .call()
            .unwrap();

        let name = member.name();

        assert_eq!(name.get(), "formatter");
    }
}
