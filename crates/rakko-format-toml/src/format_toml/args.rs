use getset::CopyGetters;
use rakko_action::{
    Args, ArgsSchema, ArgsValues, Argument, ArgumentShape, ReadArgsError, argument_name,
};

/// The documentation of the fix argument
const FIX_DOCUMENTATION: &str = "Rewrite the files that taplo can format";

/// The arguments that a run of the format-toml action reads
///
/// The action reads one argument. A run reports by default, and `fix` lets
/// taplo rewrite the files that it can format. Reporting is the safe
/// default, because a run that was started in order to look must not change
/// the tree that its caller holds.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default, CopyGetters)]
pub struct FormatTomlArgs {
    /// Whether the run rewrites the files that are not formatted
    #[getset(get_copy = "pub")]
    fix: bool,
}

impl Args for FormatTomlArgs {
    // formattoml[impl args.fix]
    fn schema() -> ArgsSchema {
        ArgsSchema::new([Argument::builder()
            .name(argument_name!("fix"))
            .shape(ArgumentShape::Boolean)
            .documentation(FIX_DOCUMENTATION)
            .build()])
    }

    // formattoml[impl args.value]
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

        Ok(Self { fix })
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

    // formattoml[verify args.value]
    #[test]
    fn from_values_with_an_unreadable_value_reports_the_argument() {
        let error = FormatTomlArgs::from_values(&values("maybe")).unwrap_err();

        assert!(matches!(
            error,
            ReadArgsError::UnreadableValue { name, .. } if name.get() == "fix"
        ));
    }

    // formattoml[verify args.fix]
    #[test]
    fn from_values_with_true_asks_for_a_rewrite() {
        let args = FormatTomlArgs::from_values(&values("true")).unwrap();

        assert!(args.fix());
    }

    // formattoml[verify args.fix]
    #[test]
    fn from_values_without_a_value_asks_for_a_report() {
        let args = FormatTomlArgs::from_values(&ArgsValues::empty()).unwrap();

        assert!(!args.fix());
    }

    // formattoml[verify args.fix]
    #[test]
    fn schema_declares_the_fix_argument() {
        let schema = FormatTomlArgs::schema();

        let names: Vec<&str> = schema
            .arguments()
            .iter()
            .map(|argument| argument.name().get())
            .collect();

        assert_eq!(names, ["fix"]);
    }

    // formattoml[verify args.fix]
    #[test]
    fn schema_documents_the_fix_argument() {
        let schema = FormatTomlArgs::schema();

        let documentation = schema
            .arguments()
            .first()
            .map(|argument| argument.documentation().get());

        assert_eq!(documentation, Some(FIX_DOCUMENTATION));
    }

    // formattoml[verify args.fix]
    #[test]
    fn schema_gives_fix_a_boolean_shape() {
        let schema = FormatTomlArgs::schema();

        let shape = schema.arguments().first().map(Argument::shape);

        assert_eq!(shape, Some(&ArgumentShape::Boolean));
    }
}
