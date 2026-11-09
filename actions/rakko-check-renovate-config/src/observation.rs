//! What one run of the validator produced
//!
//! The validator writes its log as JSON records on its standard output stream,
//! and those records carry the answer of the run. This module holds that answer
//! as data and judges nothing: the action that asked for the run decides what
//! the answer means for its outcome.

use bon::bon;
use getset::{CopyGetters, Getters};

use crate::problem::RenovateProblem;

/// What one run of the validator produced
///
/// The value holds the answer of one run that the validator finished: the
/// problems that it reported, how many configurations it validated, and what
/// it wrote when it found no configuration at all.
///
/// A run that found no configuration is not a run that found nothing wrong.
/// Both report no problem, and the two mean opposite things for the outcome of
/// an action: one project is valid, and the other was never looked at.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub(crate) struct Observation {
    /// The problems that the validator reported, in the order of its log
    #[getset(get = "pub(crate)")]
    problems: Vec<RenovateProblem>,

    /// How many configurations the validator counted in a run without a
    /// problem
    #[getset(get_copy = "pub(crate)")]
    validated: u32,

    /// What the validator wrote when it found no configuration to validate
    #[getset(get = "pub(crate)")]
    unconfigured: Option<String>,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of the validator builds one through [`Validator::observe`], and a
    /// caller builds one where a test stands in for a validator that nobody
    /// started. Every part is optional, so a test names what its case is
    /// about and leaves the rest at the answer of a run that validated nothing
    /// and found nothing.
    ///
    /// [`Validator::observe`]: crate::validator::Validator::observe
    #[builder]
    pub(crate) fn new(
        #[builder(default)] problems: Vec<RenovateProblem>,
        #[builder(default)] validated: u32,
        unconfigured: Option<String>,
    ) -> Self {
        Self {
            problems,
            validated,
            unconfigured,
        }
    }

    /// Returns the observation of two runs as the answer of one
    ///
    /// The problems of this run come first, then those of the other run. The
    /// count of the configurations is the sum of both counts. The joined runs
    /// found no configuration only when neither of them found one, and they
    /// then keep the words of this run.
    pub(crate) fn join(self, other: Self) -> Self {
        let mut problems = self.problems;
        problems.extend(other.problems);

        let unconfigured = self.unconfigured.filter(|_| other.unconfigured.is_some());

        Self {
            problems,
            validated: self.validated + other.validated,
            unconfigured,
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    /// What the validator writes when it found no configuration to validate
    const UNCONFIGURED: &str = "No files to perform configuration validation against";

    /// Returns an error of the validator about the given file
    fn problem(file: &str) -> RenovateProblem {
        RenovateProblem::builder()
            .path(PathBuf::from(file))
            .topic("Configuration Error".to_owned())
            .detail("Invalid configuration option: foo".to_owned())
            .build()
    }

    /// Returns the observation of a run that found no configuration
    fn unconfigured() -> Observation {
        Observation::builder()
            .unconfigured(UNCONFIGURED.to_owned())
            .build()
    }

    #[test]
    fn join_adds_the_counts_of_both_runs() {
        let first = Observation::builder().validated(1).build();
        let second = Observation::builder().validated(2).build();

        let joined = first.join(second);

        assert_eq!(joined.validated(), 3);
    }

    #[test]
    fn join_holds_the_problems_of_both_runs_in_order() {
        let first = Observation::builder()
            .problems(vec![problem("renovate.json")])
            .build();
        let second = Observation::builder()
            .problems(vec![problem("default.json")])
            .build();

        let joined = first.join(second);

        assert_eq!(
            joined.problems(),
            &[problem("renovate.json"), problem("default.json")]
        );
    }

    #[test]
    fn join_of_a_run_with_a_configuration_and_a_run_without_one_is_configured() {
        let first = Observation::builder().validated(1).build();

        let joined = first.join(unconfigured());

        assert_eq!(joined.unconfigured(), &None);
    }

    #[test]
    fn join_of_a_run_without_a_configuration_and_a_run_with_one_is_configured() {
        let second = Observation::builder().validated(1).build();

        let joined = unconfigured().join(second);

        assert_eq!(joined.unconfigured(), &None);
    }

    #[test]
    fn join_of_two_runs_without_a_configuration_keeps_the_words_of_the_validator() {
        let joined = unconfigured().join(unconfigured());

        assert_eq!(joined.unconfigured().as_deref(), Some(UNCONFIGURED));
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }
}
