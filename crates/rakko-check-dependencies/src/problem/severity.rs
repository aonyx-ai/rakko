//! The weight that cargo-deny gave a report
//!
//! Cargo-deny weighs everything that it reports, and the weight comes from
//! the configuration of the project. This module holds the weight and says
//! nothing about what it means for a run.

use serde::Deserialize;

/// The weight that cargo-deny gave a report
///
/// The weight is the answer that the project already gave. Every check of
/// cargo-deny carries a level in the configuration: `deny` for a shape that
/// must not appear, `warn` for one that a maintainer wants to read about, and
/// `allow` for one that the project does not care about. A check that the
/// project turned off reports nothing, and the other two arrive here.
///
/// Cargo-deny counts four weights in the summary that it ends a run with, and
/// all four are held here. The two below a warning carry the detail that a
/// report adds about a crate, and cargo-deny writes neither of them while it
/// reports at its own default level. A report that carries one is read all the
/// same, because a run that stopped over a word would say nothing about the
/// project.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// The report tells a reader something about the crate
    Help,

    /// The report adds detail to another one
    Note,

    /// The project asked to read about the shape, and not to fail over it
    Warning,

    /// The project said that the shape must not appear
    Error,
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design, and a test that reads a word
    // which cargo-deny writes expects the reading to succeed. A `# Panics`
    // section on every test would repeat that and give the reader no
    // information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns the severity that cargo-deny writes as the given word
    fn severity(word: &str) -> Severity {
        serde_json::from_str(&format!("\"{word}\""))
            .expect("the test reads a word that cargo-deny could write")
    }

    #[test]
    fn severity_of_an_error_reads_the_word_of_cargo_deny() {
        let severity = severity("error");

        assert_eq!(severity, Severity::Error);
    }

    #[test]
    fn severity_of_a_help_reads_the_word_of_cargo_deny() {
        let severity = severity("help");

        assert_eq!(severity, Severity::Help);
    }

    #[test]
    fn severity_of_a_note_reads_the_word_of_cargo_deny() {
        let severity = severity("note");

        assert_eq!(severity, Severity::Note);
    }

    #[test]
    fn severity_of_a_warning_reads_the_word_of_cargo_deny() {
        let severity = severity("warning");

        assert_eq!(severity, Severity::Warning);
    }

    // The severity arrives from the report of cargo-deny. A weight that this
    // crate does not know stops the reading, so that a new weight of a later
    // version cannot land in the wrong one.
    #[test]
    fn severity_of_an_unknown_word_is_not_read() {
        let severity = serde_json::from_str::<Severity>("\"fatal\"");

        assert!(
            severity.is_err(),
            "expected the reading to stop, got {severity:?}"
        );
    }
}
