use thiserror::Error;

use crate::args::schema::ArgumentName;
use crate::args::values::ArgumentValue;

/// An error that occurs when an argument set reads the values of a run
///
/// The machinery collects values without knowing the type of the arguments,
/// so a value can be absent or can fail to convert. An argument set reports
/// which argument it could not read and why.
// action[impl args.unreadable]
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum ReadArgsError {
    /// The run holds no value for an argument that the action requires
    #[error("no value for the required argument '{name}'")]
    MissingValue {
        /// The name of the argument that has no value
        name: ArgumentName,
    },

    /// The value of an argument does not convert to the type of its field
    #[error("value '{value}' does not fit the argument '{name}'")]
    UnreadableValue {
        /// The name of the argument whose value does not convert
        name: ArgumentName,
        /// The value that does not convert
        value: ArgumentValue,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;
    use crate::argument_name;

    // action[verify args.unreadable]
    #[test]
    fn missing_value_reports_the_argument() {
        let error = ReadArgsError::MissingValue {
            name: argument_name!("fix"),
        };

        let message = error.to_string();

        assert_eq!(message, "no value for the required argument 'fix'");
    }

    // action[verify args.unreadable]
    #[test]
    fn unreadable_value_reports_the_argument() {
        let error = ReadArgsError::UnreadableValue {
            name: argument_name!("fix"),
            value: ArgumentValue::new("maybe"),
        };

        let message = error.to_string();

        assert_eq!(message, "value 'maybe' does not fit the argument 'fix'");
    }
}
