//! The arguments that a run reads

use getset::CopyGetters;
use rakko::{Args, ArgsSchema, ArgsValues, Argument, ArgumentShape, ReadArgsError, argument_name};

use super::operation::Operation;

/// The documentation of the fix argument
const FIX_DOCUMENTATION: &str = "Add each missing owner on crates.io, as a user owner of the crate";

/// The arguments that a run of the check-crate-owners action reads
///
/// The action reads one argument. A run reports by default, and `fix` adds
/// each missing owner on crates.io. Reporting is the safe default. A run
/// that was started in order to look must not change the owners of a crate.
///
/// # Examples
///
/// ```
/// use rakko::{Args, ArgsValues, ArgumentValue, argument_name};
/// use aonyx_rakko_check_crate_owners::{CheckCrateOwnersArgs, Operation};
///
/// let values = ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);
/// let args = CheckCrateOwnersArgs::from_values(&values).unwrap();
///
/// assert_eq!(args.operation(), Operation::Repair);
/// assert_eq!(CheckCrateOwnersArgs::default().operation(), Operation::Report);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default, CopyGetters)]
pub struct CheckCrateOwnersArgs {
    /// What the run does about a missing owner
    #[getset(get_copy = "pub")]
    operation: Operation,
}

impl Args for CheckCrateOwnersArgs {
    fn schema() -> ArgsSchema {
        ArgsSchema::new([Argument::builder()
            .name(argument_name!("fix"))
            .shape(ArgumentShape::Boolean)
            .documentation(FIX_DOCUMENTATION)
            .build()])
    }

    fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
        let name = argument_name!("fix");
        let fix = match values.get(&name) {
            Some(value) => value
                .get()
                .parse()
                .map_err(|_| ReadArgsError::UnreadableValue {
                    name,
                    value: value.clone(),
                })?,
            None => false,
        };

        let operation = if fix {
            Operation::Repair
        } else {
            Operation::Report
        };

        Ok(Self { operation })
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko::ArgumentValue;

    use super::*;

    /// Returns the values of a run that gives `fix` the given text
    fn values(fix: &str) -> ArgsValues {
        ArgsValues::new([(argument_name!("fix"), ArgumentValue::new(fix))])
    }

    #[test]
    fn check_crate_owners_args_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CheckCrateOwnersArgs>();
    }

    #[test]
    fn from_values_with_an_unreadable_value_reports_the_argument() {
        let error = CheckCrateOwnersArgs::from_values(&values("maybe")).unwrap_err();

        assert!(matches!(
            error,
            ReadArgsError::UnreadableValue { name, .. } if name.get() == "fix"
        ));
    }

    #[test]
    fn from_values_with_true_asks_for_a_repair() {
        let args = CheckCrateOwnersArgs::from_values(&values("true")).unwrap();

        assert_eq!(args.operation(), Operation::Repair);
    }

    #[test]
    fn from_values_without_a_value_asks_for_a_report() {
        let args = CheckCrateOwnersArgs::from_values(&ArgsValues::empty()).unwrap();

        assert_eq!(args.operation(), Operation::Report);
    }

    #[test]
    fn schema_declares_the_fix_argument() {
        let schema = CheckCrateOwnersArgs::schema();

        let names: Vec<&str> = schema
            .arguments()
            .iter()
            .map(|argument| argument.name().get())
            .collect();

        assert_eq!(names, ["fix"]);
    }

    #[test]
    fn schema_documents_the_fix_argument() {
        let schema = CheckCrateOwnersArgs::schema();

        let documentation = schema
            .arguments()
            .first()
            .map(|argument| argument.documentation().get());

        assert_eq!(documentation, Some(FIX_DOCUMENTATION));
    }

    #[test]
    fn schema_gives_fix_a_boolean_shape() {
        let schema = CheckCrateOwnersArgs::schema();

        let shape = schema.arguments().first().map(Argument::shape);

        assert_eq!(shape, Some(&ArgumentShape::Boolean));
    }
}
