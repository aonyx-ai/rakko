/// The value of one argument
mod value;

use std::collections::BTreeMap;

pub use self::value::ArgumentValue;
use crate::args::schema::ArgumentName;

/// The values that a run reads for the arguments of an action
///
/// The machinery parses what a user asked for and collects the result here,
/// keyed by the name of each argument. An argument set turns these values into
/// its own type, so this collection is the boundary between the machinery and
/// the action: the machinery fills it without knowing the type of the
/// arguments, and the action reads it without knowing the command line.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct ArgsValues {
    /// The value that the machinery parsed for each argument, by name
    values: BTreeMap<ArgumentName, ArgumentValue>,
}

impl ArgsValues {
    /// Returns the values of a run that reads no arguments
    // action[impl args.empty]
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns the value that the run holds for an argument
    ///
    /// The method returns [`None`] when the run holds no value for the name,
    /// which is what an argument that the user left out looks like.
    // action[impl args.values]
    #[must_use]
    pub fn get(&self, name: &ArgumentName) -> Option<&ArgumentValue> {
        self.values.get(name)
    }

    /// Creates the values of a run from a name and a value for each argument
    ///
    /// When the same name appears more than once, the last value wins.
    // action[impl args.values]
    pub fn new(values: impl IntoIterator<Item = (ArgumentName, ArgumentValue)>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // action[verify args.empty]
    #[test]
    fn empty_holds_no_value() {
        let values = ArgsValues::empty();

        assert_eq!(values.get(&ArgumentName::new("fix")), None);
    }

    // action[verify args.values]
    #[test]
    fn get_returns_none_for_an_unknown_argument() {
        let values = ArgsValues::new([(ArgumentName::new("fix"), ArgumentValue::new("true"))]);

        let value = values.get(&ArgumentName::new("check"));

        assert_eq!(value, None);
    }

    // action[verify args.values]
    #[test]
    fn get_returns_the_value_of_an_argument() {
        let values = ArgsValues::new([(ArgumentName::new("fix"), ArgumentValue::new("true"))]);

        let value = values.get(&ArgumentName::new("fix"));

        assert_eq!(value, Some(&ArgumentValue::new("true")));
    }
}
