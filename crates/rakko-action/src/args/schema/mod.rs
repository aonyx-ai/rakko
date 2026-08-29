/// The description of one argument
mod argument;

use getset::Getters;

pub use self::argument::{Argument, ArgumentName, ArgumentShape, Documentation};

/// The description of the arguments that an action reads
///
/// A schema lists the arguments of an action as data, in the order that the
/// action declares them. A projection reads the schema to build a command
/// before any value exists, so the schema is available without an instance of
/// the argument set.
///
/// The schema describes what an action reads, not how a user writes it. It
/// carries no flag syntax and no position in a command tree, because those
/// decisions belong to the projection and are the same for every action.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Getters)]
pub struct ArgsSchema {
    /// The arguments, in the order that the action declares them
    #[getset(get = "pub")]
    arguments: Vec<Argument>,
}

impl ArgsSchema {
    /// Returns the schema of an action that reads no arguments
    // action[impl args.empty]
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a schema from the arguments that an action declares
    ///
    /// The order of the arguments is preserved, because a projection may show
    /// them in the order that the action declares them.
    // action[impl args.schema]
    pub fn new(arguments: impl IntoIterator<Item = Argument>) -> Self {
        Self {
            arguments: arguments.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns the description of an argument with the given name
    fn argument(name: &str) -> Argument {
        Argument::builder()
            .name(name)
            .shape(ArgumentShape::Boolean)
            .documentation("Rewrite the files that the tool can format")
            .build()
    }

    // action[verify args.empty]
    #[test]
    fn empty_returns_schema_without_arguments() {
        let schema = ArgsSchema::empty();

        assert!(schema.arguments().is_empty());
    }

    // action[verify args.schema]
    #[test]
    fn new_keeps_the_order_of_the_arguments() {
        let expected = vec![argument("fix"), argument("check")];

        let schema = ArgsSchema::new(expected.clone());

        assert_eq!(schema.arguments(), &expected);
    }
}
