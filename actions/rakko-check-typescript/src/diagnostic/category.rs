use std::fmt::{Display, Formatter, Result};

/// The word that tsc writes for a diagnostic it reports as an error
const ERROR: &str = "error";

/// The word that tsc writes for a diagnostic it reports as a warning
const WARNING: &str = "warning";

/// The word that tsc writes for a diagnostic it only states
const MESSAGE: &str = "message";

/// The word that tsc writes for a diagnostic it offers as an improvement
const SUGGESTION: &str = "suggestion";

/// The kind that tsc gave a diagnostic
///
/// Tsc writes the kind in front of the number of every diagnostic, and the
/// four variants below are the words that it writes. A check of a project sees
/// the error almost always, because that is the kind of a rule of the language
/// that the code breaks. The other three exist in the compiler, and a reader
/// of a finding gets the word that tsc wrote rather than one that this crate
/// chose for it.
///
/// The action reports every kind as a finding. A project that runs the type
/// check asked the compiler to speak, and a diagnostic that the compiler holds
/// back until it is asked is not one that Rakko decides to hide.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Category {
    /// The code breaks a rule of the language
    Error,

    /// The code is accepted, and tsc warns about it
    Warning,

    /// Tsc states something about the run
    Message,

    /// Tsc offers an improvement of the code
    Suggestion,
}

impl Category {
    /// Returns the kind that tsc writes as the given word
    ///
    /// Returns `None` for a word that is none of the four, which belongs to a
    /// line that is not the start of a diagnostic.
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            ERROR => Some(Self::Error),
            WARNING => Some(Self::Warning),
            MESSAGE => Some(Self::Message),
            SUGGESTION => Some(Self::Suggestion),
            _ => None,
        }
    }
}

impl Display for Category {
    /// Writes the kind as the word that tsc writes for it
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        let word = match self {
            Self::Error => ERROR,
            Self::Warning => WARNING,
            Self::Message => MESSAGE,
            Self::Suggestion => SUGGESTION,
        };

        formatter.write_str(word)
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn category_of_a_word_that_tsc_does_not_write_is_none() {
        let category = Category::parse("notice");

        assert_eq!(category, None);
    }

    #[test]
    fn category_writes_the_word_that_tsc_writes() {
        let word = Category::Error.to_string();

        assert_eq!(word, "error");
    }

    #[test]
    fn category_of_the_word_of_tsc_is_the_kind_that_it_names() {
        let category = Category::parse("warning");

        assert_eq!(category, Some(Category::Warning));
    }
}
