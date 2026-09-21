//! What one run of oxlint produced
//!
//! Oxlint writes one JSON object per run, and it carries both answers that an
//! action needs: how many files the run examined, and which rules the files
//! broke. This module turns one run into data and judges nothing: the action
//! that asked for the run decides what the answer means for its outcome.

use bon::bon;
use getset::{CopyGetters, Getters};

use crate::problem::OxlintProblem;

/// What one run of oxlint produced
///
/// The value holds the answer of one run: the rules that oxlint reported, and
/// how many files it examined to reach that answer.
///
/// A run that examined nothing is not a run that found nothing. Oxlint reports
/// no diagnostic for a project that holds no file it lints and for a project
/// that it linted and found clean, and the two mean opposite things for the
/// outcome of an action, so the count travels with the diagnostics.
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The rules that oxlint reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<OxlintProblem>,

    /// How many files oxlint examined
    #[getset(get_copy = "pub")]
    examined: usize,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of oxlint builds one through [`Oxlint::observe`], and a caller
    /// builds one where a test stands in for an oxlint that nobody started.
    /// Every part is optional, so a test names what its case is about and
    /// leaves the rest at the answer of a run that examined nothing at all.
    ///
    /// [`Oxlint::observe`]: crate::oxlint::Oxlint::observe
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<OxlintProblem>,
        #[builder(default)] examined: usize,
    ) -> Self {
        Self { problems, examined }
    }
}
