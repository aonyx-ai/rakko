/// The reason why an action does not apply to a project
mod skip_reason;

pub use skip_reason::SkipReason;

use crate::finding::Finding;
use std::error::Error;

/// The result of one action run
///
/// An outcome has one of four states: the action passed, the action failed, the
/// action does not apply, or the action stopped. The machinery maps each state
/// to output and to an exit code.
// action[impl outcome.send]
// action[impl outcome.sync]
#[derive(Debug)]
pub enum Outcome {
    /// The action examined the project and found no problem
    // action[impl outcome.passed]
    Passed,
    /// The action found problems in the project
    ///
    /// This state holds every [`Finding`] that the action produced.
    // action[impl outcome.failed]
    Failed {
        /// The findings that the action produced
        findings: Vec<Finding>,
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
    use crate::finding::Location;

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

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
        let location = Location::builder().path("src/main.rs").build();
        let finding = Finding::builder()
            .message("error")
            .location(location)
            .build();
        let expected = vec![finding.clone()];

        let outcome = Outcome::Failed {
            findings: vec![finding],
        };

        let Outcome::Failed { findings } = outcome else {
            panic!("expected Failed variant");
        };
        assert_eq!(findings, expected);
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
        let outcome = Outcome::Passed;

        assert!(matches!(outcome, Outcome::Passed));
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
