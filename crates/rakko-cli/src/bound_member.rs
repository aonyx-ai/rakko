use std::fmt;

use rakko_action::{ArgsValues, BoundAction, ErasedAction, Name, ReadArgsError};

/// One member of a group, bound to the values of one run
///
/// A [`Group`] runs a list of actions, its members. The projection binds every
/// member to the values of the run before the run of the group starts, and the
/// group receives one bound member for each of its members. A bound member
/// keeps the name of the action beside the [`BoundAction`], so that a group
/// can report what each member found under the name of the member.
///
/// The values are bound once. The bound action declares how its run uses the
/// project and then runs, and both see the same arguments, so a group can ask
/// every member for its access before it runs the first one.
///
/// [`Group`]: crate::Group
pub struct BoundMember<'a> {
    /// The name of the action
    name: Name,
    /// The action with the arguments of the run
    action: Box<dyn BoundAction + 'a>,
}

impl<'a> BoundMember<'a> {
    /// Binds the values of a run to the arguments of an action
    ///
    /// The projection calls this function for every member of a group, with
    /// the values of the arguments that the member declares. A test of a group
    /// calls it as well, to give the run of the group the members that it
    /// expects.
    ///
    /// # Errors
    ///
    /// Returns a [`ReadArgsError`] when the values do not convert into the
    /// arguments of the action.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_action::{Action, ArgsValues, Context, Name, Outcome, action_name};
    /// use rakko_cli::BoundMember;
    ///
    /// struct LineCount;
    ///
    /// impl Action for LineCount {
    ///     type Args = ();
    ///
    ///     fn name(&self) -> Name {
    ///         action_name!("line-count")
    ///     }
    ///
    ///     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    ///         Outcome::Passed { summary: None }
    ///     }
    /// }
    ///
    /// let action = LineCount;
    /// let member = BoundMember::bind(&action, &ArgsValues::empty())
    ///     .expect("the action reads no arguments");
    ///
    /// assert_eq!(member.name().get(), "line-count");
    /// ```
    pub fn bind(action: &'a dyn ErasedAction, values: &ArgsValues) -> Result<Self, ReadArgsError> {
        Ok(Self {
            name: action.name(),
            action: action.bind(values)?,
        })
    }

    /// Returns the name of the action
    #[must_use]
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Returns the action with the arguments of the run
    ///
    /// The bound action declares how its run uses the project and runs with
    /// the arguments that the projection bound.
    #[must_use]
    pub fn action(&self) -> &(dyn BoundAction + 'a) {
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

    /// The arguments of the formatter: whether the run repairs what it finds
    struct Fix {
        /// Whether the run rewrites the files that it can format
        fix: bool,
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
                Some(value) if value.get() == "true" => Ok(Self { fix: true }),
                Some(value) => Err(ReadArgsError::UnreadableValue {
                    name: argument_name!("fix"),
                    value: value.clone(),
                }),
                None => Ok(Self { fix: false }),
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
            if args.fix {
                Access::Write
            } else {
                Access::Read
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
        let member = BoundMember::bind(&Formatter, &fix("true")).unwrap();
        let context = Context::builder().root("/tmp/project").build();

        let access = member.action().access(&context);

        assert_eq!(access, Access::Write);
    }

    #[test]
    fn bind_reports_values_that_do_not_convert() {
        let member = BoundMember::bind(&Formatter, &fix("maybe"));

        assert!(matches!(
            member,
            Err(ReadArgsError::UnreadableValue { name, .. }) if name.get() == "fix"
        ));
    }

    #[test]
    fn name_returns_the_name_of_the_action() {
        let member = BoundMember::bind(&Formatter, &ArgsValues::empty()).unwrap();

        let name = member.name();

        assert_eq!(name.get(), "formatter");
    }
}
