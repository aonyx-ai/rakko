//! One issue that TFLint reported about a module
//!
//! A run of TFLint reports one issue for each problem that a rule found. This
//! module holds one of them. It says where the problem is, which rule found
//! it, how much it matters, and what TFLint said about it.

use std::fmt;
use std::path::PathBuf;

use bon::Builder;
use getset::{CopyGetters, Getters};
use rakko::Span;
use serde::Deserialize;

/// How much a rule of TFLint thinks that an issue matters
///
/// The configuration of a project can change the severity of a rule. TFLint
/// fails a bare run for an issue of every severity, so the level only informs
/// the reader.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Severity {
    /// The configuration cannot work as it is
    Error,

    /// The configuration works, but probably not as its author wants
    Warning,

    /// The configuration works, and another form is better
    Notice,
}

impl fmt::Display for Severity {
    /// Writes the severity with the word that TFLint uses for it
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Notice => "notice",
        };

        write!(f, "{word}")
    }
}

/// The part of the project that an issue is about
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Place {
    /// The issue is about a module as a whole, and TFLint gives no file
    Module,

    /// The issue covers a range of a file
    Range {
        /// The path of the file, as TFLint wrote it
        path: PathBuf,

        /// The range of the file that the issue covers
        span: Span,
    },
}

/// One issue that TFLint reported about a module
///
/// The message is the sentence that TFLint wrote for a reader. A reader of a
/// finding therefore reads the answer of the tool, and not one that this crate
/// wrote about it.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub(crate) struct TflintIssue {
    /// The part of the project that the issue is about
    #[getset(get = "pub(crate)")]
    place: Place,

    /// How much the rule thinks that the issue matters
    #[getset(get_copy = "pub(crate)")]
    severity: Severity,

    /// The name of the rule, which the configuration of TFLint uses as well
    #[getset(get = "pub(crate)")]
    rule: String,

    /// What the rule said about the problem
    #[getset(get = "pub(crate)")]
    description: String,
}

impl TflintIssue {
    /// Returns the sentence that TFLint wrote about the issue
    ///
    /// The sentence holds the severity, the description, and the name of the
    /// rule. The name is what a project gives to turn the rule off, and the
    /// documentation of the rule uses it too.
    pub(crate) fn message(&self) -> String {
        format!("[{}] {} ({})", self.severity, self.description, self.rule)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn display_of_a_notice_writes_the_word_of_tflint() {
        let word = Severity::Notice.to_string();

        assert_eq!(word, "notice");
    }

    #[test]
    fn message_holds_the_severity_the_description_and_the_rule() {
        let issue = TflintIssue::builder()
            .place(Place::Module)
            .severity(Severity::Warning)
            .rule("terraform_typed_variables".to_owned())
            .description("`name` variable has no type".to_owned())
            .build();

        assert_eq!(
            issue.message(),
            "[warning] `name` variable has no type (terraform_typed_variables)"
        );
    }

    #[test]
    fn place_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Place>();
    }

    #[test]
    fn severity_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Severity>();
    }

    #[test]
    fn tflint_issue_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<TflintIssue>();
    }
}
