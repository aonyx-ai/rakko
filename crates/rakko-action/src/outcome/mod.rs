/// The reason why an action does not apply to a project
mod skip_reason;
/// What a passing run examined
mod summary;

use std::error::Error;

pub use self::skip_reason::SkipReason;
pub use self::summary::Summary;
use crate::finding::Finding;

/// The result of one action run
///
/// An outcome has one of five states: the action passed, the action changed the
/// project, the action failed, the action does not apply, or the action
/// stopped. The machinery maps each state to output and to an exit code.
///
/// An action that repairs a problem instead of reporting it ends in one of two
/// of those states. It changed the project when it repaired every problem that
/// it found, and it failed when a problem remains. Both states hold the
/// repairs, so that whoever started the run learns what it rewrote instead of
/// finding it in a diff.
// action[impl outcome.send]
// action[impl outcome.sync]
#[derive(Debug)]
pub enum Outcome {
    /// The action examined the project and found no problem
    ///
    /// The state can carry a [`Summary`] of what the run examined, such as
    /// `checked 3 files`. The summary lets a reader question a pass that
    /// examined less than they expect, and an action that has nothing useful
    /// to say gives none.
    // action[impl outcome.passed]
    // action[impl outcome.summary]
    Passed {
        /// What the run examined, or `None` when the action gave no summary
        summary: Option<Summary>,
    },
    /// The action found problems in the project and repaired all of them
    ///
    /// This state holds one [`Finding`] for each problem that the action
    /// repaired. The project is clean now, and the working tree of whoever
    /// started the run differs from what they had before it.
    // action[impl outcome.changed]
    Changed {
        /// The problems that the action repaired
        repairs: Vec<Finding>,
    },
    /// The action found problems in the project
    ///
    /// This state holds every [`Finding`] that the action produced. An action
    /// that repairs what it finds reports the problems that remain here, and
    /// the ones that it repaired alongside them.
    // action[impl outcome.failed]
    // action[impl outcome.repairs]
    Failed {
        /// The findings that the action produced
        findings: Vec<Finding>,
        /// The problems that the action repaired
        ///
        /// An action that repairs nothing leaves this list empty.
        repairs: Vec<Finding>,
    },
    /// The action does not apply to the project
    ///
    /// This state holds the reason why the action does not apply.
    // action[impl outcome.skipped]
    Skipped {
        /// The reason why the action does not apply
        reason: SkipReason,
    },
    /// The action stopped before it got a result
    ///
    /// This state holds the error that stopped the action.
    // action[impl outcome.errored]
    Errored {
        /// The error that stopped the action
        source: Box<dyn Error + Send + Sync>,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;
    use crate::finding::{FilePath, Location};

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    // action[verify outcome.changed]
    #[test]
    fn changed_variant_holds_repairs() {
        let location = Location::File {
            path: FilePath::try_from("deny.toml").unwrap(),
        };
        let repair = Finding::builder()
            .message("the file was not formatted")
            .location(location)
            .build();
        let expected = vec![repair.clone()];

        let outcome = Outcome::Changed {
            repairs: vec![repair],
        };

        let Outcome::Changed { repairs } = outcome else {
            panic!("expected Changed variant");
        };
        assert_eq!(repairs, expected);
    }

    // action[verify outcome.errored]
    #[test]
    fn errored_variant_holds_error() {
        let outcome = Outcome::Errored {
            source: Box::new(std::io::Error::other("boom")),
        };

        let Outcome::Errored { source } = &outcome else {
            panic!("expected Errored variant");
        };
        assert_eq!(source.to_string(), "boom");
    }

    // action[verify outcome.failed]
    #[test]
    fn failed_variant_holds_findings() {
        let location = Location::File {
            path: FilePath::try_from("src/main.rs").unwrap(),
        };
        let finding = Finding::builder()
            .message("error")
            .location(location)
            .build();
        let expected = vec![finding.clone()];

        let outcome = Outcome::Failed {
            findings: vec![finding],
            repairs: Vec::new(),
        };

        let Outcome::Failed { findings, .. } = outcome else {
            panic!("expected Failed variant");
        };
        assert_eq!(findings, expected);
    }

    // action[verify outcome.repairs]
    #[test]
    fn failed_variant_holds_repairs() {
        let location = Location::File {
            path: FilePath::try_from("deny.toml").unwrap(),
        };
        let repair = Finding::builder()
            .message("the file was not formatted")
            .location(location)
            .build();
        let expected = vec![repair.clone()];

        let outcome = Outcome::Failed {
            findings: Vec::new(),
            repairs: vec![repair],
        };

        let Outcome::Failed { repairs, .. } = outcome else {
            panic!("expected Failed variant");
        };
        assert_eq!(repairs, expected);
    }

    // action[verify outcome.send]
    #[test]
    fn outcome_is_send() {
        assert_send::<Outcome>();
    }

    // action[verify outcome.sync]
    #[test]
    fn outcome_is_sync() {
        assert_sync::<Outcome>();
    }

    // action[verify outcome.passed]
    #[test]
    fn passed_variant_exists() {
        let outcome = Outcome::Passed { summary: None };

        assert!(matches!(outcome, Outcome::Passed { .. }));
    }

    // action[verify outcome.summary]
    #[test]
    fn passed_variant_holds_summary() {
        let outcome = Outcome::Passed {
            summary: Some(Summary::new("checked 3 files")),
        };

        let Outcome::Passed { summary } = &outcome else {
            panic!("expected Passed variant");
        };
        assert_eq!(summary.as_ref().map(Summary::get), Some("checked 3 files"));
    }

    // action[verify outcome.summary]
    #[test]
    fn passed_variant_without_a_summary_reports_none() {
        let outcome = Outcome::Passed { summary: None };

        let Outcome::Passed { summary } = &outcome else {
            panic!("expected Passed variant");
        };
        assert_eq!(*summary, None);
    }

    // action[verify outcome.skipped]
    #[test]
    fn skipped_variant_holds_reason() {
        let outcome = Outcome::Skipped {
            reason: SkipReason::new("not applicable"),
        };

        let Outcome::Skipped { reason } = &outcome else {
            panic!("expected Skipped variant");
        };
        assert_eq!(reason.get(), "not applicable");
    }
}
