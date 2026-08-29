/// The error type for name parsing
mod error;
/// What kebab case holds, and the names that hold only that
pub(crate) mod kebab_case;

pub use self::error::ParseNameError;
pub(crate) use self::kebab_case::kebab_case_name;

/// Constructs a [`Name`] from a literal string at compile time
///
/// The macro evaluates [`Name::from_static`] in a constant context, so the
/// build validates the literal. A literal that does not satisfy the rules
/// for a name fails the build.
///
/// # Examples
///
/// ```
/// use rakko_action::action_name;
///
/// let name = action_name!("format-toml");
///
/// assert_eq!(name.get(), "format-toml");
/// ```
///
/// A literal that does not satisfy the rules fails the build:
///
/// ```compile_fail
/// use rakko_action::action_name;
///
/// let name = action_name!("format--toml");
/// ```
// action[impl name.literal]
#[macro_export]
macro_rules! action_name {
    ($input:expr) => {
        const { $crate::Name::from_static($input) }
    };
}

// action[impl name.accepts]
// action[impl name.text]
kebab_case_name! {
    /// The name of an action
    ///
    /// A name identifies an action. It holds only lowercase ASCII letters,
    /// ASCII digits, and hyphens. It starts with a lowercase ASCII letter, it
    /// does not hold two consecutive hyphens, and it does not end with a
    /// hyphen.
    ///
    /// Construct a name through [`FromStr`], [`TryFrom<&str>`], or
    /// [`TryFrom<String>`]. Every constructor validates the input and returns
    /// a [`ParseNameError`] when the input does not meet the rules. The
    /// [`action_name!`] macro constructs a name from a literal string at
    /// compile time, so an invalid literal fails the build instead of the
    /// run.
    ///
    /// [`FromStr`]: std::str::FromStr
    Name
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // action[verify name.literal]
    #[test]
    fn action_name_with_valid_literal_returns_name() {
        let name = action_name!("format-toml");

        assert_eq!(name.get(), "format-toml");
    }

    // action[verify name.text]
    #[test]
    fn display_shows_the_text_that_the_name_was_made_from() {
        let name: Name = "fmt".parse().unwrap();

        assert_eq!(name.to_string(), "fmt");
    }

    // action[verify name.literal]
    #[test]
    fn from_static_accepts_valid_name() {
        let name = Name::from_static("format-toml");

        assert_eq!(name.get(), "format-toml");
    }

    // action[verify name.literal]
    #[test]
    #[should_panic(expected = "name contains consecutive hyphens")]
    fn from_static_with_consecutive_hyphens_panics() {
        Name::from_static("bad--name");
    }

    // action[verify name.literal]
    #[test]
    #[should_panic(expected = "name is empty")]
    fn from_static_with_empty_string_panics() {
        Name::from_static("");
    }

    // action[verify name.literal]
    #[test]
    #[should_panic(expected = "name contains an invalid character")]
    fn from_static_with_invalid_character_panics() {
        Name::from_static("hello.world");
    }

    // action[verify name.literal]
    #[test]
    #[should_panic(expected = "name starts with an invalid character")]
    fn from_static_with_leading_digit_panics() {
        Name::from_static("1check");
    }

    // action[verify name.literal]
    #[test]
    #[should_panic(expected = "name ends with a hyphen")]
    fn from_static_with_trailing_hyphen_panics() {
        Name::from_static("trailing-");
    }

    #[test]
    fn from_str_accepts_name_with_digits() {
        let name: Name = "check2".parse().unwrap();

        assert_eq!(name.get(), "check2");
    }

    #[test]
    fn from_str_accepts_name_with_hyphen() {
        let name: Name = "format-rust".parse().unwrap();

        assert_eq!(name.get(), "format-rust");
    }

    #[test]
    fn from_str_accepts_single_letter() {
        let name: Name = "a".parse().unwrap();

        assert_eq!(name.get(), "a");
    }

    // action[verify name.accepts]
    #[test]
    fn from_str_accepts_valid_name() {
        let name: Name = "validate".parse().unwrap();

        assert_eq!(name.get(), "validate");
    }

    // action[verify name.hyphens]
    #[test]
    fn from_str_with_consecutive_hyphens_returns_error() {
        let err = "bad--name".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::ConsecutiveHyphens { position: 4 });
    }

    #[test]
    fn from_str_with_consecutive_hyphens_takes_precedence_over_trailing() {
        let err = "a--".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::ConsecutiveHyphens { position: 2 });
    }

    // action[verify name.empty]
    #[test]
    fn from_str_with_empty_string_returns_error() {
        let err = "".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::Empty);
    }

    #[test]
    fn from_str_with_invalid_character_before_consecutive_hyphens_returns_invalid_character() {
        let err = "a!-b".parse::<Name>().unwrap_err();

        assert_eq!(
            err,
            ParseNameError::InvalidCharacter {
                character: '!',
                position: 1,
            },
        );
    }

    // action[verify name.character]
    #[test]
    fn from_str_with_invalid_character_in_middle_returns_error() {
        let err = "hello.world".parse::<Name>().unwrap_err();

        assert_eq!(
            err,
            ParseNameError::InvalidCharacter {
                character: '.',
                position: 5,
            },
        );
    }

    // action[verify name.start]
    #[test]
    fn from_str_with_leading_digit_returns_error() {
        let err = "1check".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::InvalidStart { character: '1' });
    }

    #[test]
    fn from_str_with_leading_hyphen_returns_error() {
        let err = "-check".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::InvalidStart { character: '-' });
    }

    #[test]
    fn from_str_with_leading_consecutive_hyphens_returns_invalid_start() {
        let err = "--name".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::InvalidStart { character: '-' });
    }

    #[test]
    fn from_str_with_leading_uppercase_returns_error() {
        let err = "Check".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::InvalidStart { character: 'C' });
    }

    // action[verify name.end]
    #[test]
    fn from_str_with_trailing_hyphen_returns_error() {
        let err = "trailing-".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::TrailingHyphen);
    }

    #[test]
    fn from_str_with_uppercase_in_middle_returns_error() {
        let err = "helloWorld".parse::<Name>().unwrap_err();

        assert_eq!(
            err,
            ParseNameError::InvalidCharacter {
                character: 'W',
                position: 5,
            },
        );
    }

    // action[verify name.text]
    #[test]
    fn get_returns_the_text_that_the_name_was_made_from() {
        let name: Name = "fmt".parse().unwrap();

        assert_eq!(name.get(), "fmt");
    }

    #[test]
    fn try_from_str_accepts_valid_name() {
        let name = Name::try_from("fmt").unwrap();

        assert_eq!(name.get(), "fmt");
    }

    #[test]
    fn try_from_str_with_empty_returns_error() {
        let err = Name::try_from("").unwrap_err();

        assert_eq!(err, ParseNameError::Empty);
    }

    #[test]
    fn try_from_string_accepts_valid_name() {
        let name = Name::try_from("fmt".to_string()).unwrap();

        assert_eq!(name.get(), "fmt");
    }

    #[test]
    fn try_from_string_with_empty_returns_error() {
        let err = Name::try_from(String::new()).unwrap_err();

        assert_eq!(err, ParseNameError::Empty);
    }

    #[test]
    fn validate_rejects_invalid_start_before_invalid_character() {
        let err = "A!".parse::<Name>().unwrap_err();

        assert_eq!(err, ParseNameError::InvalidStart { character: 'A' });
    }
}
