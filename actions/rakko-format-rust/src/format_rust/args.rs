//! The arguments that a run of the action reads

use getset::CopyGetters;
use rakko_action::{
    Args, ArgsSchema, ArgsValues, Argument, ArgumentShape, ReadArgsError, argument_name,
};

/// What the run does with files that are not formatted
///
/// # Examples
///
/// ```
/// use rakko_format_rust::Operation;
///
/// assert_eq!(Operation::default(), Operation::Report);
/// assert_ne!(Operation::Rewrite, Operation::Report);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Operation {
    /// Report files that are not formatted
    #[default]
    Report,
    /// Rewrite files that are not formatted
    Rewrite,
}

/// The documentation of the fix argument
const FIX_DOCUMENTATION: &str = "Rewrite the files that rustfmt can format";

/// The arguments that a run of the format-rust action reads
///
/// The action reads one argument. A run reports by default, and `fix` lets
/// rustfmt rewrite the files that it can format. Reporting is the safe
/// default, because a run that was started in order to look must not change
/// the tree that its caller holds.
///
/// # Examples
///
/// ```
/// use rakko_action::{Args, ArgsValues, ArgumentValue, argument_name};
/// use rakko_format_rust::{FormatRustArgs, Operation};
///
/// let values = ArgsValues::new([(argument_name!("fix"), ArgumentValue::new("true"))]);
/// let args = FormatRustArgs::from_values(&values).unwrap();
/// assert_eq!(args.operation(), Operation::Rewrite);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default, CopyGetters)]
pub struct FormatRustArgs {
    /// What the run does with files that are not formatted
    #[getset(get_copy = "pub")]
    operation: Operation,
}

impl Args for FormatRustArgs {
    fn schema() -> ArgsSchema {
        ArgsSchema::new([Argument::builder()
            .name(argument_name!("fix"))
            .shape(ArgumentShape::Boolean)
            .documentation(FIX_DOCUMENTATION)
            .build()])
    }

    fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
        let name = argument_name!("fix");
        let operation = match values.get(&name) {
            Some(value) => {
                let fix: bool =
                    value
                        .get()
                        .parse()
                        .map_err(|_| ReadArgsError::UnreadableValue {
                            name,
                            value: value.clone(),
                        })?;
                if fix {
                    Operation::Rewrite
                } else {
                    Operation::Report
                }
            }
            None => Operation::Report,
        };

        Ok(Self { operation })
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::ArgumentValue;

    use super::*;

    /// Returns the values of a run that gives `fix` the given text
    fn values(fix: &str) -> ArgsValues {
        ArgsValues::new([(argument_name!("fix"), ArgumentValue::new(fix))])
    }

    #[test]
    fn format_rust_args_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<FormatRustArgs>();
    }

    #[test]
    fn from_values_with_an_unreadable_value_reports_the_argument() {
        let error = FormatRustArgs::from_values(&values("maybe")).unwrap_err();

        assert!(matches!(
            error,
            ReadArgsError::UnreadableValue { name, .. } if name.get() == "fix"
        ));
    }

    #[test]
    fn from_values_with_true_asks_for_a_rewrite() {
        let args = FormatRustArgs::from_values(&values("true")).unwrap();

        assert_eq!(args.operation(), Operation::Rewrite);
    }

    #[test]
    fn from_values_without_a_value_asks_for_a_report() {
        let args = FormatRustArgs::from_values(&ArgsValues::empty()).unwrap();

        assert_eq!(args.operation(), Operation::Report);
    }

    #[test]
    fn operation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Operation>();
    }

    #[test]
    fn schema_declares_the_fix_argument() {
        let schema = FormatRustArgs::schema();

        let names: Vec<&str> = schema
            .arguments()
            .iter()
            .map(|argument| argument.name().get())
            .collect();

        assert_eq!(names, ["fix"]);
    }

    #[test]
    fn schema_documents_the_fix_argument() {
        let schema = FormatRustArgs::schema();

        let documentation = schema
            .arguments()
            .first()
            .map(|argument| argument.documentation().get());

        assert_eq!(documentation, Some(FIX_DOCUMENTATION));
    }

    #[test]
    fn schema_gives_fix_a_boolean_shape() {
        let schema = FormatRustArgs::schema();

        let shape = schema.arguments().first().map(Argument::shape);

        assert_eq!(shape, Some(&ArgumentShape::Boolean));
    }
}
