//! The severity that stiles gave a diagnostic
//!
//! A rule of stiles weighs each diagnostic that it reports, and the weight
//! travels with the diagnostic in the report. This module holds the weight and
//! says what it means for a run.

use std::fmt;

use serde::Deserialize;

/// The severity that stiles gave a diagnostic
///
/// The rule that reports a diagnostic decides its severity. Stiles fails a run
/// on an error by default, and on a warning as well when a run asks it to. A
/// note and a help add context to the code, and stiles fails a run on neither
/// of them, whatever a run asks.
///
/// A reader of a finding sees the severity, because an error and a warning
/// both fail a run of the action, and the severity tells the reader which one
/// to read first.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The diagnostic suggests a change to the code
    Help,

    /// The diagnostic tells the reader something about the code
    Note,

    /// The diagnostic is a problem that the rule weighs as a warning
    Warning,

    /// The diagnostic is a problem that the rule weighs as an error
    Error,
}

impl Severity {
    /// Returns whether a diagnostic of this severity fails a run of the action
    ///
    /// An error and a warning fail a run, because both are a rule that the
    /// project asked stiles to run, and a project that wants a rule to stay
    /// quiet turns that rule off. A note and a help never fail a run, which is
    /// how stiles treats them in its strictest mode as well.
    pub fn fails(self) -> bool {
        matches!(self, Self::Error | Self::Warning)
    }
}

/// The word that stiles writes for a diagnostic that suggests a change
const HELP: &str = "help";

/// The word that stiles writes for a diagnostic that adds context
const NOTE: &str = "note";

/// The word that stiles writes for a diagnostic that it weighs as a warning
const WARNING: &str = "warning";

/// The word that stiles writes for a diagnostic that it weighs as an error
const ERROR: &str = "error";

impl fmt::Display for Severity {
    /// Writes the severity in the word that stiles uses for it
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Self::Help => HELP,
            Self::Note => NOTE,
            Self::Warning => WARNING,
            Self::Error => ERROR,
        };

        f.write_str(word)
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn fails_of_a_help_is_false() {
        assert!(!Severity::Help.fails());
    }

    #[test]
    fn fails_of_a_note_is_false() {
        assert!(!Severity::Note.fails());
    }

    #[test]
    fn fails_of_a_warning_is_true() {
        assert!(Severity::Warning.fails());
    }

    #[test]
    fn fails_of_an_error_is_true() {
        assert!(Severity::Error.fails());
    }

    #[test]
    fn to_string_of_a_warning_is_the_word_of_stiles() {
        assert_eq!(Severity::Warning.to_string(), "warning");
    }
}
