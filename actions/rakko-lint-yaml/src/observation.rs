//! What one run of yamllint produced
//!
//! Yamllint answers over both of its streams. The report travels on the
//! standard output stream, and the standard error stream carries the reason
//! of a run that stopped early. This module turns one run into data and
//! judges nothing: the action that asked for the run decides what the answer
//! means for its outcome.

use bon::bon;
use getset::{CopyGetters, Getters};

use crate::problem::YamllintProblem;

/// Whether a run of yamllint examined every file that it collected
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub(crate) enum Completion {
    /// Yamllint examined every file that it had collected
    #[default]
    Finished,

    /// Yamllint stopped before examining all files
    Interrupted,
}

/// What one run of yamllint produced
///
/// The value holds the answer of one run: the rules yamllint reported.
/// It says whether yamllint examined every file that it had collected.
/// It carries the text that it wrote about a run that it could not finish.
///
/// A run that stopped early is not a run that found nothing. Yamllint
/// stops at the first file that it cannot open. The problems it had
/// already reported still arrive. A caller that ignored this would report
/// a part of the project as though it were the whole.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub(crate) struct Observation {
    /// The rules that yamllint reported, in the order of the report
    #[getset(get = "pub(crate)")]
    problems: Vec<YamllintProblem>,

    /// Whether yamllint examined every file that it had collected
    #[getset(get_copy = "pub(crate)")]
    finished: Completion,

    /// What yamllint wrote to its standard error stream
    ///
    /// A run that examined every file writes nothing here. A run that stopped
    /// early writes why.
    #[getset(get = "pub(crate)")]
    stderr: String,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of yamllint builds one through [`Yamllint::observe`]. A caller
    /// builds one where a test stands in for a yamllint that nobody started.
    /// Every part is optional, so a test names what its case is about.
    /// The rest defaults to the answer of a run that examined the project
    /// and found nothing.
    ///
    /// [`Yamllint::observe`]: crate::yamllint::Yamllint::observe
    #[builder]
    pub(crate) fn new(
        #[builder(default)] problems: Vec<YamllintProblem>,
        #[builder(default)] finished: Completion,
        #[builder(into, default)] stderr: String,
    ) -> Self {
        Self {
            problems,
            finished,
            stderr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Completion>();
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }
}
