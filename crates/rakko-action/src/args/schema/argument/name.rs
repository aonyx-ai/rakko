use crate::name::kebab_case_name;

/// Constructs an [`ArgumentName`] from a literal string at compile time
///
/// The macro evaluates [`ArgumentName::from_static`] in a constant context, so
/// the build validates the literal. A literal string that does not satisfy the
/// rules for a name fails the build.
///
/// # Examples
///
/// ```
/// use rakko_action::argument_name;
///
/// let name = argument_name!("fix");
///
/// assert_eq!(name.get(), "fix");
/// ```
///
/// A literal that does not satisfy the rules fails the build:
///
/// ```compile_fail
/// use rakko_action::argument_name;
///
/// let name = argument_name!("--fix");
/// ```
// action[impl args.literal]
#[macro_export]
macro_rules! argument_name {
    ($input:expr) => {
        const { $crate::ArgumentName::from_static($input) }
    };
}

// action[impl args.name]
kebab_case_name! {
    /// The name of one argument in an argument set
    ///
    /// The name identifies the argument in a description and in the values
    /// that a run reads. A projection turns the name into the token that a
    /// user types, so the projection owns the syntax and the name owns only
    /// the identity.
    ///
    /// A name holds only what such a token can carry. It holds lowercase
    /// ASCII letters, ASCII digits, and hyphens, it starts with a lowercase
    /// ASCII letter, it holds no two hyphens next to each other, and it does
    /// not end with a hyphen. These are the rules that the name of an action
    /// satisfies, because both names reach a user in the same place.
    ///
    /// Construct a name through [`FromStr`], [`TryFrom<&str>`], or
    /// [`TryFrom<String>`]. Every constructor validates the input and returns
    /// a [`ParseNameError`] when the input does not meet the rules. The
    /// [`argument_name!`] macro constructs a name from a literal string at
    /// compile time, so an invalid literal fails the build instead of the
    /// run.
    ///
    /// [`FromStr`]: std::str::FromStr
    /// [`ParseNameError`]: crate::ParseNameError
    ArgumentName
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;
    use crate::name::ParseNameError;

    // action[verify args.literal]
    #[test]
    fn argument_name_with_a_valid_literal_returns_a_name() {
        let name = argument_name!("fix");

        assert_eq!(name.get(), "fix");
    }

    // action[verify args.literal]
    #[test]
    #[should_panic(expected = "name starts with an invalid character")]
    fn from_static_with_a_name_that_starts_with_a_hyphen_panics() {
        ArgumentName::from_static("-fix");
    }

    // action[verify args.literal]
    #[test]
    fn from_static_with_a_valid_name_returns_a_name() {
        let name = ArgumentName::from_static("format-only");

        assert_eq!(name.get(), "format-only");
    }

    // action[verify args.name]
    #[test]
    fn parse_with_a_name_that_holds_a_space_reports_the_character() {
        let result: Result<ArgumentName, _> = "fix all".parse();

        assert_eq!(
            result,
            Err(ParseNameError::InvalidCharacter {
                character: ' ',
                position: 3,
            })
        );
    }

    // action[verify args.name]
    #[test]
    fn parse_with_a_valid_name_returns_a_name() {
        let name: ArgumentName = "fix".parse().expect("the test names an argument correctly");

        assert_eq!(name.get(), "fix");
    }

    // action[verify args.name]
    #[test]
    fn parse_without_characters_reports_that_the_name_is_empty() {
        let result: Result<ArgumentName, _> = "".parse();

        assert_eq!(result, Err(ParseNameError::Empty));
    }
}
