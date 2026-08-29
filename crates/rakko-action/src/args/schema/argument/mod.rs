/// The documentation of one argument
mod documentation;
/// The name of one argument
mod name;
/// The kind of value that one argument holds
mod shape;

use bon::bon;
use getset::Getters;

pub use self::documentation::Documentation;
pub use self::name::ArgumentName;
pub use self::shape::ArgumentShape;

/// The description of one argument that an action reads
///
/// An argument describes itself as data. It gives the name that identifies
/// it, the shape of the value that it holds, and the sentence that tells a
/// user what it does. A projection builds a command from those three, and an
/// action names no flag.
///
/// All three are mandatory. An argument without a shape is one that no
/// projection can render, and an argument without documentation reaches a
/// user who cannot tell what it does.
///
/// The description carries no default and does not say that the argument is
/// required. The argument set of an action decides what an absent value
/// means, so a run from a command line and a run from a test read the same
/// rules.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Argument {
    /// The name that identifies the argument
    #[getset(get = "pub")]
    name: ArgumentName,
    /// The kind of value that the argument holds
    #[getset(get = "pub")]
    shape: ArgumentShape,
    /// The sentence that tells a user what the argument does
    #[getset(get = "pub")]
    documentation: Documentation,
}

#[bon]
impl Argument {
    /// Creates the description of an argument
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_action::{Argument, ArgumentShape};
    ///
    /// let argument = Argument::builder()
    ///     .name("fix")
    ///     .shape(ArgumentShape::Boolean)
    ///     .documentation("Rewrite the files that the tool can format")
    ///     .build();
    ///
    /// assert_eq!(argument.name().get(), "fix");
    /// ```
    // action[impl args.argument]
    // action[impl args.shape]
    // action[impl args.documentation]
    #[builder]
    pub fn new(
        #[builder(into)] name: ArgumentName,
        shape: ArgumentShape,
        #[builder(into)] documentation: Documentation,
    ) -> Self {
        Self {
            name,
            shape,
            documentation,
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns the description of an argument that holds the given shape
    fn argument(shape: ArgumentShape) -> Argument {
        Argument::builder()
            .name("fix")
            .shape(shape)
            .documentation("Rewrite the files that the tool can format")
            .build()
    }

    // action[verify args.documentation]
    #[test]
    fn documentation_returns_given_documentation() {
        let argument = Argument::builder()
            .name("fix")
            .shape(ArgumentShape::Boolean)
            .documentation("Rewrite the files that the tool can format")
            .build();

        let documentation = argument.documentation();

        assert_eq!(
            documentation.get(),
            "Rewrite the files that the tool can format"
        );
    }

    // action[verify args.argument]
    #[test]
    fn name_returns_given_name() {
        let argument = argument(ArgumentShape::Boolean);

        let name = argument.name();

        assert_eq!(name.get(), "fix");
    }

    // action[verify args.shape]
    #[test]
    fn shape_returns_a_boolean() {
        let argument = argument(ArgumentShape::Boolean);

        let shape = argument.shape();

        assert_eq!(shape, &ArgumentShape::Boolean);
    }

    // action[verify args.shape]
    #[test]
    fn shape_returns_a_path() {
        let argument = argument(ArgumentShape::Path);

        let shape = argument.shape();

        assert_eq!(shape, &ArgumentShape::Path);
    }

    // action[verify args.shape]
    #[test]
    fn shape_returns_a_text() {
        let argument = argument(ArgumentShape::Text);

        let shape = argument.shape();

        assert_eq!(shape, &ArgumentShape::Text);
    }

    // action[verify args.shape]
    #[test]
    fn shape_returns_a_whole_number() {
        let argument = argument(ArgumentShape::Integer);

        let shape = argument.shape();

        assert_eq!(shape, &ArgumentShape::Integer);
    }
}
