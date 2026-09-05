//! What one run of markdownlint produced
//!
//! Markdownlint answers over both of its streams. The report travels on the
//! standard error stream, and the standard output stream stays empty for as
//! long as markdownlint has files to examine. This module turns one run into
//! data and judges nothing: the action that asked for the run decides what
//! the answer means for its outcome.

use bon::bon;
use getset::{CopyGetters, Getters};

use crate::problem::MarkdownlintProblem;

/// What one run of markdownlint produced
///
/// The value holds the answer of one run: the rules that markdownlint
/// reported, whether it examined a file at all, the text that it wrote about
/// the run, and whether it ended with success.
///
/// A run that found nothing to examine is not a run that found nothing wrong.
/// Markdownlint answers an empty selection with its usage text on the
/// standard output stream and ends with success, and both runs would
/// otherwise look alike.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The rules that markdownlint reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<MarkdownlintProblem>,

    /// Whether markdownlint examined a file of the project
    #[getset(get_copy = "pub")]
    examined: bool,

    /// What markdownlint wrote to its standard error stream
    ///
    /// The report of a run travels here, and so does the diagnosis of a run
    /// that ended without one.
    #[getset(get = "pub")]
    stderr: String,

    /// Whether markdownlint ended with success
    #[getset(get_copy = "pub")]
    succeeded: bool,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of markdownlint builds one through [`Markdownlint::observe`], and
    /// a caller builds one where a test stands in for a markdownlint that
    /// nobody started. Every part is optional, so a test names what its case
    /// is about and leaves the rest at the answer of a run that examined the
    /// project and found nothing.
    ///
    /// [`Markdownlint::observe`]: crate::markdownlint::Markdownlint::observe
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<MarkdownlintProblem>,
        #[builder(default = true)] examined: bool,
        #[builder(into, default)] stderr: String,
        #[builder(default = true)] succeeded: bool,
    ) -> Self {
        Self {
            problems,
            examined,
            stderr,
            succeeded,
        }
    }
}
