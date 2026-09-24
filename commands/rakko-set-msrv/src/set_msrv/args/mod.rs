/// A minimum supported Rust version
mod msrv;
/// Why a project needs its minimum supported Rust version
mod reason;

use getset::Getters;
use rakko_action::{
    Args, ArgsSchema, ArgsValues, Argument, ArgumentName, ArgumentShape, ArgumentValue,
    ReadArgsError, argument_name,
};

pub use self::msrv::Msrv;
pub use self::reason::Reason;

/// The documentation of the msrv argument
const MSRV_DOCUMENTATION: &str = "The oldest Rust version that the project compiles on, as 1.88.0";

/// The documentation of the reason argument
const REASON_DOCUMENTATION: &str = "Why the project needs this version, for the comment above it";

/// The number of parts in a complete version
const PARTS: usize = 3;

/// The arguments that a run of the set-msrv command reads
///
/// Both arguments are required. The version must have three parts, because
/// mise resolves a pin of `1.88` to the newest `1.88.x` while Cargo reads the
/// same text as `1.88.0`. A version with two parts would therefore check
/// another toolchain than the one that the manifest promises. No part starts
/// with a zero, because Cargo refuses such a version in a manifest.
///
/// The reason is one line that says something. It becomes the comment above
/// the version, and a blank reason would only delete that comment. A comment
/// in TOML ends at the line break and holds no other control character.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Getters)]
pub struct SetMsrvArgs {
    /// The version that the project promises to compile on after the run
    #[getset(get = "pub")]
    msrv: Msrv,

    /// Why the project needs the version
    #[getset(get = "pub")]
    reason: Reason,
}

impl Args for SetMsrvArgs {
    // setmsrv[impl args.declare]
    fn schema() -> ArgsSchema {
        ArgsSchema::new([
            Argument::builder()
                .name(argument_name!("msrv"))
                .shape(ArgumentShape::Text)
                .documentation(MSRV_DOCUMENTATION)
                .build(),
            Argument::builder()
                .name(argument_name!("reason"))
                .shape(ArgumentShape::Text)
                .documentation(REASON_DOCUMENTATION)
                .build(),
        ])
    }

    // setmsrv[impl args.missing]
    // setmsrv[impl args.form]
    // setmsrv[impl args.reason]
    fn from_values(values: &ArgsValues) -> Result<Self, ReadArgsError> {
        let msrv = required(values, argument_name!("msrv"))?;
        if !complete(msrv.get()) {
            return Err(ReadArgsError::UnreadableValue {
                name: argument_name!("msrv"),
                value: msrv.clone(),
            });
        }

        let reason = required(values, argument_name!("reason"))?;
        if reason.get().trim().is_empty()
            || reason
                .get()
                .chars()
                .any(|character| character.is_control() && character != '\t')
        {
            return Err(ReadArgsError::UnreadableValue {
                name: argument_name!("reason"),
                value: reason.clone(),
            });
        }

        Ok(Self {
            msrv: Msrv::new(msrv.get()),
            reason: Reason::new(reason.get()),
        })
    }
}

/// Returns the value of an argument that a run must give
///
/// # Errors
///
/// Returns [`ReadArgsError::MissingValue`] when the run gave the argument no
/// value.
fn required(values: &ArgsValues, name: ArgumentName) -> Result<&ArgumentValue, ReadArgsError> {
    values
        .get(&name)
        .ok_or(ReadArgsError::MissingValue { name })
}

/// Returns whether a version has three parts that are numbers
///
/// A number is `0` or starts with another digit, as Cargo reads it.
fn complete(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();

    parts.len() == PARTS
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (*part == "0" || !part.starts_with('0'))
        })
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns the values of a run that gives the arguments the given text
    fn values(msrv: &str, reason: &str) -> ArgsValues {
        ArgsValues::new([
            (argument_name!("msrv"), ArgumentValue::new(msrv)),
            (argument_name!("reason"), ArgumentValue::new(reason)),
        ])
    }

    // setmsrv[verify args.reason]
    #[test]
    fn from_values_with_a_blank_reason_reports_the_argument() {
        let error = SetMsrvArgs::from_values(&values("1.89.0", "  ")).unwrap_err();

        assert_eq!(
            error,
            ReadArgsError::UnreadableValue {
                name: argument_name!("reason"),
                value: ArgumentValue::new("  "),
            }
        );
    }

    // setmsrv[verify args.form]
    #[test]
    fn from_values_with_a_complete_version_reads_it() {
        let args = SetMsrvArgs::from_values(&values("1.89.0", "bon needs it")).unwrap();

        assert_eq!(args.msrv(), &Msrv::new("1.89.0"));
    }

    // setmsrv[verify args.form]
    #[test]
    fn from_values_with_a_leading_zero_reports_the_argument() {
        let error = SetMsrvArgs::from_values(&values("1.089.0", "bon needs it")).unwrap_err();

        assert_eq!(
            error,
            ReadArgsError::UnreadableValue {
                name: argument_name!("msrv"),
                value: ArgumentValue::new("1.089.0"),
            }
        );
    }

    // setmsrv[verify args.reason]
    #[test]
    fn from_values_with_a_line_break_in_the_reason_reports_the_argument() {
        let error = SetMsrvArgs::from_values(&values("1.89.0", "bon\nneeds it")).unwrap_err();

        assert_eq!(
            error,
            ReadArgsError::UnreadableValue {
                name: argument_name!("reason"),
                value: ArgumentValue::new("bon\nneeds it"),
            }
        );
    }

    // setmsrv[verify args.form]
    #[test]
    fn from_values_with_a_version_of_two_parts_reports_the_argument() {
        let error = SetMsrvArgs::from_values(&values("1.89", "bon needs it")).unwrap_err();

        assert_eq!(
            error,
            ReadArgsError::UnreadableValue {
                name: argument_name!("msrv"),
                value: ArgumentValue::new("1.89"),
            }
        );
    }

    // setmsrv[verify args.form]
    #[test]
    fn from_values_with_a_version_that_is_no_number_reports_the_argument() {
        let error = SetMsrvArgs::from_values(&values("1.89.x", "bon needs it")).unwrap_err();

        assert_eq!(
            error,
            ReadArgsError::UnreadableValue {
                name: argument_name!("msrv"),
                value: ArgumentValue::new("1.89.x"),
            }
        );
    }

    // setmsrv[verify args.missing]
    #[test]
    fn from_values_without_a_reason_reports_the_argument() {
        let values = ArgsValues::new([(argument_name!("msrv"), ArgumentValue::new("1.89.0"))]);

        let error = SetMsrvArgs::from_values(&values).unwrap_err();

        assert_eq!(
            error,
            ReadArgsError::MissingValue {
                name: argument_name!("reason")
            }
        );
    }

    // setmsrv[verify args.missing]
    #[test]
    fn from_values_without_a_version_reports_the_argument() {
        let values = ArgsValues::new([(argument_name!("reason"), ArgumentValue::new("why"))]);

        let error = SetMsrvArgs::from_values(&values).unwrap_err();

        assert_eq!(
            error,
            ReadArgsError::MissingValue {
                name: argument_name!("msrv")
            }
        );
    }

    // setmsrv[verify args.declare]
    #[test]
    fn schema_declares_two_documented_texts() {
        let schema = SetMsrvArgs::schema();

        let arguments: Vec<(&str, &ArgumentShape, bool)> = schema
            .arguments()
            .iter()
            .map(|argument| {
                (
                    argument.name().get(),
                    argument.shape(),
                    argument.documentation().get().is_empty(),
                )
            })
            .collect();

        assert_eq!(
            arguments,
            [
                ("msrv", &ArgumentShape::Text, false),
                ("reason", &ArgumentShape::Text, false),
            ]
        );
    }
}
