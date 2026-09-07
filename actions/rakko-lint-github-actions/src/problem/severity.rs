//! The severity that zizmor gave a finding
//!
//! Zizmor weighs each of its audits, and the weight travels with every
//! finding of that audit. This module holds the weight and says nothing about
//! what it means for a run.

use std::fmt;

use serde::Deserialize;

/// The severity that zizmor gave a finding
///
/// The severity belongs to the audit and not to the workflow: a template that
/// expands attacker-controlled text into a shell is high wherever it appears,
/// and a naming habit that a reviewer wants to see is informational wherever
/// it appears. Zizmor reports every severity in one run, and a project that
/// wants an audit to stay quiet turns that audit off.
///
/// A reader of a finding sees the severity, because a finding of every
/// severity fails a run, and the severity is what tells the reader which one
/// to read first.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
pub enum Severity {
    /// The finding tells the reader something about the workflow
    Informational,

    /// The finding is a weakness that needs other things to go wrong
    Low,

    /// The finding is a weakness that a reviewer should act on
    Medium,

    /// The finding is a way into the repository
    High,
}

/// The word that zizmor writes for a finding that tells the reader something
const INFORMATIONAL: &str = "informational";

/// The word that zizmor writes for a finding of the lowest weight
const LOW: &str = "low";

/// The word that zizmor writes for a finding of the middle weight
const MEDIUM: &str = "medium";

/// The word that zizmor writes for a finding of the highest weight
const HIGH: &str = "high";

impl fmt::Display for Severity {
    /// Writes the severity with the word that zizmor uses for it
    ///
    /// Zizmor names a severity in two places, and the two disagree in case.
    /// The report capitalizes it, and the summary that zizmor writes for a
    /// reader counts the findings in lower case. A message reads next to the
    /// audit that produced it, so it takes the lower case of the summary.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Self::Informational => INFORMATIONAL,
            Self::Low => LOW,
            Self::Medium => MEDIUM,
            Self::High => HIGH,
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
    fn display_of_a_high_severity_writes_the_word_of_zizmor() {
        let word = Severity::High.to_string();

        assert_eq!(word, "high");
    }

    #[test]
    fn display_of_an_informational_severity_writes_the_word_of_zizmor() {
        let word = Severity::Informational.to_string();

        assert_eq!(word, "informational");
    }

    #[test]
    fn display_of_a_low_severity_writes_the_word_of_zizmor() {
        let word = Severity::Low.to_string();

        assert_eq!(word, "low");
    }

    #[test]
    fn display_of_a_medium_severity_writes_the_word_of_zizmor() {
        let word = Severity::Medium.to_string();

        assert_eq!(word, "medium");
    }

    // The severity arrives from the report of zizmor, which writes the name of
    // the variant. A severity that this crate does not know stops the reading,
    // so that a new weight of a later version cannot land in the wrong one.
    #[test]
    fn severity_of_an_unknown_word_is_not_read() {
        let severity = serde_json::from_str::<Severity>("\"Critical\"");

        assert!(
            severity.is_err(),
            "expected the reading to stop, got {severity:?}"
        );
    }
}
