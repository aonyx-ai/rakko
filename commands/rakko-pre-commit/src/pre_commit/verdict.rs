use rakko_action::Outcome;

/// What the outcome of one action means for the commit
///
/// An action answers with one of five outcomes, and a command answers with
/// success or an error. The run therefore reduces the five states to these
/// two, and the reports that it wrote keep what each action actually said.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum Verdict {
    /// The action left the project ready for a commit
    Clean,

    /// The action found a problem, or it stopped before it had an answer
    Problem,
}

impl Verdict {
    /// Returns what an outcome means for the commit
    ///
    /// An action that does not apply is clean, because a skip is an answer. An
    /// action that repaired everything that it found is clean as well: the run
    /// asked for the repair and got it, and the files that it rewrote sit in
    /// the working tree, where the hook that started the run compares them
    /// against what the commit holds.
    ///
    /// An action that stopped is a problem, although it says nothing about the
    /// project. A commit that nothing examined is what the hook exists to
    /// prevent.
    // precommit[impl result.clean]
    // precommit[impl result.problem]
    pub(crate) fn of(outcome: &Outcome) -> Self {
        match outcome {
            Outcome::Passed { .. } | Outcome::Changed { .. } | Outcome::Skipped { .. } => {
                Self::Clean
            }
            Outcome::Failed { .. } | Outcome::Errored { .. } => Self::Problem,
        }
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko_action::SkipReason;

    use super::*;

    // precommit[verify result.clean]
    #[test]
    fn of_a_changed_outcome_is_clean() {
        let outcome = Outcome::Changed {
            repairs: Vec::new(),
        };

        let verdict = Verdict::of(&outcome);

        assert_eq!(verdict, Verdict::Clean);
    }

    // precommit[verify result.problem]
    #[test]
    fn of_a_failed_outcome_is_a_problem() {
        let outcome = Outcome::Failed {
            findings: Vec::new(),
            repairs: Vec::new(),
        };

        let verdict = Verdict::of(&outcome);

        assert_eq!(verdict, Verdict::Problem);
    }

    // precommit[verify result.clean]
    #[test]
    fn of_a_passed_outcome_is_clean() {
        let outcome = Outcome::Passed { summary: None };

        let verdict = Verdict::of(&outcome);

        assert_eq!(verdict, Verdict::Clean);
    }

    // precommit[verify result.clean]
    #[test]
    fn of_a_skipped_outcome_is_clean() {
        let outcome = Outcome::Skipped {
            reason: SkipReason::new("the project has no Rust code"),
        };

        let verdict = Verdict::of(&outcome);

        assert_eq!(verdict, Verdict::Clean);
    }

    // precommit[verify result.problem]
    #[test]
    fn of_an_errored_outcome_is_a_problem() {
        let outcome = Outcome::Errored {
            source: Box::new(std::io::Error::other("boom")),
        };

        let verdict = Verdict::of(&outcome);

        assert_eq!(verdict, Verdict::Problem);
    }
}
