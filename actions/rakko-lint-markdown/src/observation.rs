//! What one run of markdownlint produced
//!
//! Markdownlint answers over both of its streams. The report travels on the
//! standard error stream. A run that has no file to examine answers on one
//! stream or the other, as its version selects. This module turns one run into
//! data and judges nothing: the action that asked for the run decides what the
//! answer means for its outcome.

use bon::bon;
use getset::{CopyGetters, Getters};

use crate::problem::MarkdownlintProblem;

/// Whether markdownlint examined a file of the project
///
/// # Examples
///
/// A run that found no file to examine reports `Skipped`, and a caller tells
/// it apart from a run that examined the project:
///
/// ```
/// use rakko_lint_markdown::observation::Examination;
///
/// let examination = Examination::Skipped;
///
/// let label = match examination {
///     Examination::Examined => "examined the project",
///     Examination::Skipped => "found no file to examine",
/// };
///
/// assert_eq!(label, "found no file to examine");
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum Examination {
    /// Markdownlint examined a file
    #[default]
    Examined,
    /// Markdownlint examined no file
    Skipped,
}

/// Whether markdownlint ended with success
///
/// # Examples
///
/// A run that ends without success is a failure of the run, and a caller
/// checks this before it trusts the report:
///
/// ```
/// use rakko_lint_markdown::observation::Exit;
///
/// let exit = Exit::Failed;
///
/// assert_ne!(exit, Exit::Succeeded);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum Exit {
    /// Markdownlint ended with success
    #[default]
    Succeeded,
    /// Markdownlint ended without success
    Failed,
}

/// What one run of markdownlint produced
///
/// The value holds the answer of one run. It includes the rules markdownlint
/// reported, whether a file was examined, the text written, and whether it
/// ended with success.
///
/// A run that found nothing to examine is not a run that found nothing wrong.
/// Markdownlint answers an empty selection without a report and ends with
/// success. An older version writes its usage text on the standard output
/// stream. A newer version writes a sentence on the standard error stream
/// instead. Without this distinction, both runs would look alike.
///
/// # Examples
///
/// A run that is left at its defaults examined the project, ended with
/// success, and found nothing:
///
/// ```
/// use rakko_lint_markdown::Observation;
/// use rakko_lint_markdown::observation::{Examination, Exit};
///
/// let observation = Observation::builder().build();
///
/// assert!(observation.problems().is_empty());
/// assert_eq!(observation.examined(), Examination::Examined);
/// assert_eq!(observation.exit(), Exit::Succeeded);
/// ```
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The rules that markdownlint reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<MarkdownlintProblem>,

    /// Whether markdownlint examined a file of the project
    #[getset(get_copy = "pub")]
    examined: Examination,

    /// What markdownlint wrote to its standard error stream
    ///
    /// The report of a run travels here, and so does the diagnosis of a run
    /// that ended without one.
    #[getset(get = "pub")]
    stderr: String,

    /// Whether markdownlint ended with success
    #[getset(get_copy = "pub")]
    exit: Exit,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of markdownlint builds one through [`Markdownlint::observe`], and
    /// a caller builds one where a test stands in for a markdownlint that
    /// nobody started. Every part is optional, so a test names what its case
    /// is about. The rest defaults to the answer of a run that examined the
    /// project and found nothing.
    ///
    /// # Examples
    ///
    /// A test names the one part that its case is about:
    ///
    /// ```
    /// use rakko_lint_markdown::Observation;
    /// use rakko_lint_markdown::observation::Exit;
    ///
    /// let observation = Observation::builder()
    ///     .exit(Exit::Failed)
    ///     .stderr("Error: cannot open the file")
    ///     .build();
    ///
    /// assert_eq!(observation.exit(), Exit::Failed);
    /// assert_eq!(observation.stderr(), "Error: cannot open the file");
    /// ```
    ///
    /// [`Markdownlint::observe`]: crate::markdownlint::Markdownlint::observe
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<MarkdownlintProblem>,
        #[builder(default)] examined: Examination,
        #[builder(into, default)] stderr: String,
        #[builder(default)] exit: Exit,
    ) -> Self {
        Self {
            problems,
            examined,
            stderr,
            exit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examination_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Examination>();
    }

    #[test]
    fn exit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Exit>();
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }
}
