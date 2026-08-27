use thiserror::Error;

/// An error that occurs when parsing an action name
///
/// A [`Name`](super::Name) validates its input against a set of rules.
/// This error identifies which rule the input violated.
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
