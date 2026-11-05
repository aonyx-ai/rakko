//! What one check of stiles produced
//!
//! Stiles answers over both of its streams. The diagnostics travel on the
//! standard output stream, and the standard error stream carries the reason
//! of a run that stopped. This module turns one check into data and judges
//! nothing: the action that asked for the check decides what the answer means
//! for its outcome.

use bon::bon;
use getset::{CopyGetters, Getters};

use crate::problem::StilesProblem;

/// What one check of stiles produced
///
/// The value holds the answer of one check: the diagnostics that stiles
/// reported, and whether stiles found a file to check at all.
///
/// A check that found no file is not a check that found nothing. Stiles
/// reports no diagnostic in both cases, and the two mean opposite things for
/// the outcome of an action: one project is clean, and the other was never
/// looked at.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The diagnostics that stiles reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<StilesProblem>,

    /// Whether stiles found a file to check
    #[getset(get_copy = "pub")]
    examined: bool,
}

#[bon]
impl Observation {
    /// Creates the observation of a check
    ///
    /// A run of stiles builds one through [`Stiles::observe`], and a caller
    /// builds one where a test stands in for a stiles that nobody started.
    /// Every part is optional, so a test names what its case is about and
    /// leaves the rest at the answer of a check that examined the project and
    /// found nothing.
    ///
    /// [`Stiles::observe`]: crate::stiles::Stiles::observe
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<StilesProblem>,
        #[builder(default = true)] examined: bool,
    ) -> Self {
        Self { problems, examined }
    }
}
