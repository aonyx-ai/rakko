//! The severity that oxlint gave a diagnostic
//!
//! Oxlint reports a diagnostic at one of two severities, and the
//! configuration of a project decides which severity each of its rules
//! carries. This module holds the severity and says nothing about what it
//! means for a run.

use std::fmt;

use serde::Deserialize;

/// The severity that oxlint gave a diagnostic
///
/// The severity belongs to the rule and not to the file: a project that wants
/// a rule to speak without stopping anyone weighs that rule as a warning, and
/// the rules that oxlint enables by itself carry the warning severity.
///
/// The severity decides the status that oxlint ends with, and it decides
/// nothing about the outcome of an action. A rule that a project weighs as a
/// warning is still a rule that the project asked oxlint to look for, so a
/// reader of a finding sees the severity, and every severity fails a run.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The diagnostic breaks a rule that the project weighs as a warning
    Warning,

    /// The diagnostic breaks a rule that the project weighs as an error
    Error,
}

/// The word that oxlint writes for a diagnostic of a warning rule
const WARNING: &str = "warning";

/// The word that oxlint writes for a diagnostic of an error rule
const ERROR: &str = "error";

impl fmt::Display for Severity {
    /// Writes the severity with the word that oxlint uses for it
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Self::Warning => WARNING,
            Self::Error => ERROR,
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
    fn display_of_an_error_writes_the_word_of_oxlint() {
        let word = Severity::Error.to_string();

        assert_eq!(word, "error");
    }

    #[test]
    fn display_of_a_warning_writes_the_word_of_oxlint() {
        let word = Severity::Warning.to_string();

        assert_eq!(word, "warning");
    }

    // The severity arrives from the report of oxlint, which writes it in lower
    // case. A severity that this crate does not know stops the reading, so
    // that a new weight of a later version cannot land in the wrong one.
    #[test]
    fn severity_of_an_unknown_word_is_not_read() {
        let severity = serde_json::from_str::<Severity>("\"advice\"");

        assert!(
            severity.is_err(),
            "expected the reading to stop, got {severity:?}"
        );
    }
}
