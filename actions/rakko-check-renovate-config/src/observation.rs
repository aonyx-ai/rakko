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
pub struct Observation {
    /// The problems that the validator reported, in the order of its log
    #[getset(get = "pub")]
    problems: Vec<RenovateProblem>,

    /// How many configurations the validator counted in a run without a
    /// problem
    #[getset(get_copy = "pub")]
    validated: u32,

    /// What the validator wrote when it found no configuration to validate
    #[getset(get = "pub")]
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
    pub fn new(
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
}
