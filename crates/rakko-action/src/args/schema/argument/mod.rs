/// The name of one argument
mod name;

use getset::Getters;
pub use name::ArgumentName;

/// The description of one argument that an action reads
///
/// An argument describes itself as data. Today it gives the name that
/// identifies it, and it describes no syntax, because the shape of a command
/// line belongs to the projection that renders it.
///
/// The description is a struct rather than a bare name, so that it can carry
/// more once an action needs to say more.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct Argument {
    /// The name that identifies the argument
    #[getset(get = "pub")]
    name: ArgumentName,
}

impl Argument {
    /// Creates the description of an argument from its name
    // action[impl args.argument]
    pub fn new(name: impl Into<ArgumentName>) -> Self {
        Self { name: name.into() }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // action[verify args.argument]
    #[test]
    fn name_returns_given_name() {
        let argument = Argument::new("fix");

        let name = argument.name();

        assert_eq!(name.get(), "fix");
    }
}
