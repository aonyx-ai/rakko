//! What one run of zizmor produced
//!
//! Zizmor answers over both of its streams. The report travels on the
//! standard output stream. The standard error stream carries the log of the
//! run and the reason of a run that stopped. This module turns one run
//! into data and judges nothing: the action that asked for the run decides
//! what the answer means for its outcome.

use bon::bon;
use getset::{CopyGetters, Getters};

use crate::problem::ZizmorProblem;

/// Whether zizmor collected a file to audit
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub(crate) enum Examination {
    /// Zizmor collected a file to audit
    #[default]
    Examined,

    /// Zizmor collected no file to audit
    Skipped,
}

/// What one run of zizmor produced
///
/// The value holds the answer of one run: the places that zizmor reported,
/// and whether zizmor collected a file to audit at all.
///
/// A run that collected nothing is not a run that found nothing. Zizmor
/// reports an empty array in both cases. The two mean opposite things for
/// the outcome of an action: one project is clean, and the other was never
/// looked at.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub(crate) struct Observation {
    /// The places that zizmor reported, in the order of the report
    #[getset(get = "pub(crate)")]
    problems: Vec<ZizmorProblem>,

    /// Whether zizmor collected a file to audit
    #[getset(get_copy = "pub(crate)")]
    collected: Examination,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of zizmor builds one through [`Zizmor::observe`], and a caller
    /// builds one where a test stands in for a zizmor that nobody started.
    /// Every part is optional. So a test names what its case is about. It
    /// leaves the rest at the answer of a run that audited the project and
    /// found nothing.
    ///
    /// [`Zizmor::observe`]: crate::zizmor::Zizmor::observe
    #[builder]
    pub(crate) fn new(
        #[builder(default)] problems: Vec<ZizmorProblem>,
        #[builder(default)] collected: Examination,
    ) -> Self {
        Self {
            problems,
            collected,
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
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }
}
