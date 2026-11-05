//! What one run of zizmor produced
//!
//! Zizmor answers over both of its streams. The report travels on the
//! standard output stream, and the standard error stream carries the log of
//! the run and the reason of a run that stopped. This module turns one run
//! into data and judges nothing: the action that asked for the run decides
//! what the answer means for its outcome.

use bon::bon;
use getset::{CopyGetters, Getters};

use crate::problem::ZizmorProblem;

/// Whether zizmor collected a file to audit
///
/// # Examples
///
/// ```
/// use rakko_lint_github_actions::observation::Collection;
///
/// let collected = Collection::default();
///
/// assert_eq!(collected, Collection::Collected);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub enum Collection {
    /// Zizmor collected a file
    #[default]
    Collected,
    /// Zizmor collected no file
    Empty,
}

/// What one run of zizmor produced
///
/// The value holds the answer of one run: the places that zizmor reported,
/// and whether zizmor collected a file to audit at all.
///
/// A run that collected nothing is not a run that found nothing. Zizmor
/// reports an empty array in both cases, and the two mean opposite things for
/// the outcome of an action: one project is clean, and the other was never
/// looked at.
///
/// # Examples
///
/// ```
/// use rakko_lint_github_actions::Observation;
///
/// let observation = Observation::builder().build();
///
/// assert!(observation.problems().is_empty());
/// ```
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The places that zizmor reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<ZizmorProblem>,

    /// Whether zizmor collected a file to audit
    #[getset(get_copy = "pub")]
    collected: Collection,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of zizmor builds one through [`Zizmor::observe`], and a caller
    /// builds one where a test stands in for a zizmor that nobody started.
    /// Every part is optional, so a test names what its case is about and
    /// leaves the rest at the answer of a run that audited the project and
    /// found nothing.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_lint_github_actions::Observation;
    /// use rakko_lint_github_actions::observation::Collection;
    ///
    /// let observation = Observation::builder()
    ///     .collected(Collection::default())
    ///     .build();
    ///
    /// assert!(observation.problems().is_empty());
    /// assert_eq!(observation.collected(), Collection::Collected);
    /// ```
    ///
    /// [`Zizmor::observe`]: crate::zizmor::Zizmor::observe
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<ZizmorProblem>,
        #[builder(default)] collected: Collection,
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
    fn collection_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Collection>();
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }
}
