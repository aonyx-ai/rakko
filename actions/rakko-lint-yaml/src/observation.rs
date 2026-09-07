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

/// What one run of yamllint produced
///
/// The value holds the answer of one run: the rules that yamllint reported,
/// whether yamllint examined every file that it had collected, and the text
/// that it wrote about a run that it could not finish.
///
/// A run that stopped early is not a run that found nothing. Yamllint stops
/// at the first file that it cannot open, and the problems that it had
/// already reported still arrive, so a caller that ignored this would report
/// a part of the project as though it were the whole.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The rules that yamllint reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<YamllintProblem>,

    /// Whether yamllint examined every file that it had collected
    #[getset(get_copy = "pub")]
    finished: bool,

    /// What yamllint wrote to its standard error stream
    ///
    /// A run that examined every file writes nothing here. A run that stopped
    /// early writes why.
    #[getset(get = "pub")]
    stderr: String,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of yamllint builds one through [`Yamllint::observe`], and a
    /// caller builds one where a test stands in for a yamllint that nobody
    /// started. Every part is optional, so a test names what its case is
    /// about and leaves the rest at the answer of a run that examined the
    /// project and found nothing.
    ///
    /// [`Yamllint::observe`]: crate::yamllint::Yamllint::observe
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<YamllintProblem>,
        #[builder(default = true)] finished: bool,
        #[builder(into, default)] stderr: String,
    ) -> Self {
        Self {
            problems,
            finished,
            stderr,
        }
    }
}
