//! One diagnostic that oxlint reported about a project
//!
//! A run of oxlint reports one diagnostic for each rule that a file broke, and
//! one for each problem that it found with the project itself, such as a
//! suppressions file that no longer agrees with the code. This module holds
//! one of them: where it belongs, how the project weighs it, and what oxlint
//! said about it.

/// Where a diagnostic of oxlint belongs
mod place;
/// The severity that oxlint gave a diagnostic
mod severity;

use bon::Builder;
use getset::{CopyGetters, Getters};

pub use self::place::Place;
pub use self::severity::Severity;

/// The character that opens the severity of a diagnostic in a message
const SEVERITY_OPEN: char = '[';

/// The text that closes the severity of a diagnostic in a message
const SEVERITY_CLOSE: &str = "] ";

/// The text that separates the rule of a diagnostic from its description
const RULE_CLOSE: &str = ": ";

/// The text that introduces what oxlint suggests about a diagnostic
///
/// Oxlint writes the suggestion behind this word in its own report, and a
/// message that carries one reads the same way.
const HELP_OPEN: &str = " help: ";

/// One diagnostic that oxlint reported about a project
///
/// A diagnostic belongs to a position in a file, to a whole file, or to the
/// project, and the [place][place] says which. A diagnostic about the code of
/// a file marks one or more places in it, and this value carries the first of
/// them, because oxlint reports one diagnostic however many places it marks.
///
/// The description is the sentence that oxlint wrote for a reader, the code is
/// the rule that the diagnostic reports, and the severity is the one that
/// oxlint gave it, which the configuration of the project decides for a rule.
/// A diagnostic that no rule reports, such as a syntax error or a diagnostic
/// about the project, has no code. Oxlint suggests what to do about most
/// diagnostics, and the suggestion travels where oxlint wrote one. A reader of
/// a finding therefore reads the answer of the tool, and not one that this
/// crate wrote about it.
///
/// [place]: OxlintProblem::place
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct OxlintProblem {
    /// Where the diagnostic belongs
    #[getset(get = "pub")]
    place: Place,

    /// The severity that oxlint gave the diagnostic
    #[getset(get_copy = "pub")]
    severity: Severity,

    /// The rule that the diagnostic reports, as oxlint names it, where it
    /// names one
    #[builder(into)]
    #[getset(get = "pub")]
    code: Option<String>,

    /// What oxlint said about the diagnostic
    #[builder(into)]
    #[getset(get = "pub")]
    description: String,

    /// What oxlint suggests about the diagnostic, where it suggests anything
    #[builder(into)]
    #[getset(get = "pub")]
    help: Option<String>,
}

impl OxlintProblem {
    /// Returns the sentence that oxlint wrote about the diagnostic
    ///
    /// The sentence holds the severity, the rule, the description, and the
    /// suggestion, in the order that oxlint writes them, so that a finding
    /// reads like the line that a contributor sees when they run oxlint
    /// themselves. A diagnostic without a rule leaves the rule and its
    /// separator out, so that the sentence holds only the words of oxlint.
    pub fn message(&self) -> String {
        let rule = match &self.code {
            Some(code) => format!("{code}{RULE_CLOSE}"),
            None => String::new(),
        };

        let help = match &self.help {
            Some(help) => format!("{HELP_OPEN}{help}"),
            None => String::new(),
        };

        format!(
            "{SEVERITY_OPEN}{}{SEVERITY_CLOSE}{rule}{}{help}",
            self.severity, self.description
        )
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    #[test]
    fn message_of_a_diagnostic_reads_like_the_line_of_oxlint() {
        let problem = OxlintProblem::builder()
            .place(Place::File {
                path: PathBuf::from("index.ts"),
            })
            .severity(Severity::Warning)
            .code("eslint(no-debugger)")
            .description("`debugger` statement is not allowed")
            .help("Remove the debugger statement")
            .build();

        assert_eq!(
            problem.message(),
            "[warning] eslint(no-debugger): `debugger` statement is not allowed \
             help: Remove the debugger statement"
        );
    }

    #[test]
    fn message_of_a_diagnostic_without_a_rule_holds_the_words_of_oxlint_alone() {
        let problem = OxlintProblem::builder()
            .place(Place::Project)
            .severity(Severity::Error)
            .description("There are suppressions that do not occur anymore.")
            .help("Run `oxlint --prune-suppressions` to remove unused suppressions.")
            .build();

        assert_eq!(
            problem.message(),
            "[error] There are suppressions that do not occur anymore. \
             help: Run `oxlint --prune-suppressions` to remove unused suppressions."
        );
    }

    #[test]
    fn message_of_a_diagnostic_without_a_sentence_holds_the_rule_alone() {
        let problem = OxlintProblem::builder()
            .place(Place::File {
                path: PathBuf::from("index.ts"),
            })
            .severity(Severity::Error)
            .code("eslint(id-length)")
            .description("Identifier name `x` is too short")
            .build();

        assert_eq!(
            problem.message(),
            "[error] eslint(id-length): Identifier name `x` is too short"
        );
    }
}
