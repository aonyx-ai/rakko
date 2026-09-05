//! The level that yamllint gave a problem
//!
//! Yamllint reports a problem at one of two levels, and the configuration of
//! a project decides which level each of its rules carries. This module holds
//! the level and says nothing about what it means for a run.

use std::fmt;

/// The level that yamllint gave a problem
///
/// The level belongs to the rule and not to the file: a project that wants a
/// rule to speak without stopping anyone sets that rule to a warning, and the
/// rules that yamllint enables by default carry both levels.
///
/// A reader of a finding sees the level, because a rule that a project called
/// a warning still points at a problem, and the level says how the project
/// weighed it.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ProblemLevel {
    /// The problem breaks a rule that the project treats as an error
    Error,

    /// The problem breaks a rule that the project treats as a warning
    Warning,
}

/// The word that yamllint writes for a problem that breaks an error rule
const ERROR: &str = "error";

/// The word that yamllint writes for a problem that breaks a warning rule
const WARNING: &str = "warning";

impl ProblemLevel {
    /// Returns the level that yamllint named, or `None` for another word
    ///
    /// Yamllint writes one of two words, and a third word belongs to a
    /// version of yamllint that this crate does not know. The caller stops the
    /// run in that case, instead of guessing what the word means.
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            ERROR => Some(Self::Error),
            WARNING => Some(Self::Warning),
            _ => None,
        }
    }
}

impl fmt::Display for ProblemLevel {
    /// Writes the level with the word that yamllint uses for it
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Self::Error => ERROR,
            Self::Warning => WARNING,
        };

        write!(f, "{word}")
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn display_of_an_error_writes_the_word_of_yamllint() {
        let word = ProblemLevel::Error.to_string();

        assert_eq!(word, "error");
    }

    #[test]
    fn display_of_a_warning_writes_the_word_of_yamllint() {
        let word = ProblemLevel::Warning.to_string();

        assert_eq!(word, "warning");
    }

    #[test]
    fn parse_of_an_error_names_the_error_level() {
        let level = ProblemLevel::parse("error");

        assert_eq!(level, Some(ProblemLevel::Error));
    }

    #[test]
    fn parse_of_another_word_names_no_level() {
        let level = ProblemLevel::parse("notice");

        assert_eq!(level, None);
    }

    #[test]
    fn parse_of_a_warning_names_the_warning_level() {
        let level = ProblemLevel::parse("warning");

        assert_eq!(level, Some(ProblemLevel::Warning));
    }
}
