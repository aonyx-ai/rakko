use thiserror::Error;

/// An error that occurs when parsing a name
///
/// The name of an action and the name of an argument satisfy one set of
/// rules, and both report this error. The variant identifies the rule that
/// the input does not satisfy.
#[derive(Clone, Eq, PartialEq, Debug, Error)]
pub enum ParseNameError {
    /// The input was empty
    #[error("name is empty")]
    Empty,

    /// The first character is not a lowercase ASCII letter
    #[error("name starts with an invalid character: '{character}'")]
    InvalidStart {
        /// The character that the name started with
        character: char,
    },

    /// A character is not a lowercase ASCII letter, an ASCII digit, or a
    /// hyphen
    #[error(
        "name contains an invalid character '{character}' at position \
         {position}"
    )]
    InvalidCharacter {
        /// The character that is not valid in a name
        character: char,
        /// The position of the invalid character in the input
        position: usize,
    },

    /// Two hyphens appear next to each other
    #[error("name contains consecutive hyphens at position {position}")]
    ConsecutiveHyphens {
        /// The position of the second hyphen in the pair
        position: usize,
    },

    /// The last character of the input is a hyphen
    #[error("name ends with a hyphen")]
    TrailingHyphen,
}
