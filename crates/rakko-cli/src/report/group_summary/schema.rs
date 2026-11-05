//! The JSON that a run of a group emits for a machine

use rakko_action::Outcome;
use serde::Serialize;

use super::GroupSummary;
use crate::report::Report;
use crate::report::schema::UNSTABLE;

/// The JSON that a run of a group emits for a machine
///
/// The payload is the one document of the run. It holds the report of every
/// member in the shape that a run of that action alone emits. So a consumer
/// reads a member with the code that reads one action. The counts let a
/// consumer judge the run without a pass over the members.
#[derive(Serialize)]
pub(super) struct Payload<'a> {
    /// Whether the shape of this payload is stable
    schema: &'static str,
    /// The name of the group that the run drove
    group: &'a str,
    /// How many members ended in each state
    counts: Counts,
    /// The report of each member, in the order of the list of the group
    members: &'a [Report],
}

impl<'a> Payload<'a> {
    /// Returns the payload that describes the given summary
    pub(super) fn of(summary: &'a GroupSummary) -> Self {
        let GroupSummary { group, members } = summary;
        Self {
            schema: UNSTABLE,
            group: group.get(),
            counts: Counts::of(members),
            members,
        }
    }
}

/// How many members of a run ended in each state
///
/// Each field takes the name of a state as the report of a member gives it.
/// A state that no member reached carries zero, so a consumer reads the same
/// fields whatever the run found.
#[derive(Default, Serialize)]
struct Counts {
    /// The members that examined the project and found no problem
    passed: usize,
    /// The members that repaired every problem that they found
    changed: usize,
    /// The members that found problems that remain in the project
    failed: usize,
    /// The members that do not apply to the project
    skipped: usize,
    /// The members that stopped before they had an answer
    errored: usize,
}

impl Counts {
    /// Returns the number of the given reports in each state
    fn of(reports: &[Report]) -> Self {
        let mut counts = Self::default();

        for report in reports {
            match report.outcome {
                Outcome::Passed { .. } => counts.passed += 1,
                Outcome::Changed { .. } => counts.changed += 1,
                Outcome::Failed { .. } => counts.failed += 1,
                Outcome::Skipped { .. } => counts.skipped += 1,
                Outcome::Errored { .. } => counts.errored += 1,
            }
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use rakko_action::{SkipReason, action_name};

    use super::*;

    /// Returns the JSON that a run of the group `check` emits
    ///
    /// The assertions compare the JSON as text, and not as a parsed value, so
    /// that they pin the bytes that a consumer reads.
    fn payload(members: Vec<Report>) -> String {
        let summary = GroupSummary::builder()
            .group(action_name!("check"))
            .members(members)
            .build();

        serde_json::to_string(&summary).expect("the summary serializes")
    }

    #[test]
    fn counts_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Counts>();
    }

    #[test]
    fn payload_carries_the_group_and_the_report_of_every_member() {
        let json = payload(vec![
            Report::builder()
                .action(action_name!("lint-toml"))
                .outcome(Outcome::Passed { summary: None })
                .build(),
            Report::builder()
                .action(action_name!("lint-yaml"))
                .outcome(Outcome::Failed {
                    findings: Vec::new(),
                    repairs: Vec::new(),
                })
                .build(),
        ]);

        assert_eq!(
            json,
            r#"{"schema":"unstable","group":"check","counts":{"passed":1,"changed":0,"failed":1,"skipped":0,"errored":0},"members":[{"schema":"unstable","action":"lint-toml","outcome":"passed","findings":[],"repairs":[]},{"schema":"unstable","action":"lint-yaml","outcome":"failed","findings":[],"repairs":[]}]}"#
        );
    }

    #[test]
    fn payload_counts_the_members_in_each_state() {
        let json = payload(vec![
            Report::builder()
                .action(action_name!("lint-toml"))
                .outcome(Outcome::Passed { summary: None })
                .build(),
            Report::builder()
                .action(action_name!("format-toml"))
                .outcome(Outcome::Changed {
                    repairs: Vec::new(),
                })
                .build(),
            Report::builder()
                .action(action_name!("format-yaml"))
                .outcome(Outcome::Changed {
                    repairs: Vec::new(),
                })
                .build(),
            Report::builder()
                .action(action_name!("lint-yaml"))
                .outcome(Outcome::Failed {
                    findings: Vec::new(),
                    repairs: Vec::new(),
                })
                .build(),
            Report::builder()
                .action(action_name!("lint-json"))
                .outcome(Outcome::Skipped {
                    reason: SkipReason::new("the project has no JSON file"),
                })
                .build(),
            Report::builder()
                .action(action_name!("test-rust"))
                .outcome(Outcome::Errored {
                    source: Box::new(std::io::Error::other("the tool is missing")),
                })
                .build(),
        ]);

        assert!(
            json.contains(
                r#""counts":{"passed":1,"changed":2,"failed":1,"skipped":1,"errored":1}"#
            )
        );
    }

    #[test]
    fn payload_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Payload<'static>>();
    }

    #[test]
    fn payload_without_members_carries_no_report() {
        let json = payload(Vec::new());

        assert_eq!(
            json,
            r#"{"schema":"unstable","group":"check","counts":{"passed":0,"changed":0,"failed":0,"skipped":0,"errored":0},"members":[]}"#
        );
    }
}
