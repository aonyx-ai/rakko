//! The text that a run of a group emits for a reader at a terminal

use std::fmt;

use rakko::Outcome;

use super::GroupSummary;
use crate::report::text::render_count;

/// Writes the summary of a run of a group as the text that a reader at a
/// terminal gets
///
/// Each member that did not pass takes one line, in the order of the list of
/// the group. The line gives the name of the member and whether it found
/// problems or stopped. The report of the member already showed what it
/// found, so the line only tells the reader where to look. That is all that a
/// reader learns when the flags that ask for less output hide the reports.
///
/// The last line gives the name of the group and counts the members that did
/// not pass. So the line that sums up the run sits at the end of the output.
/// The report of one action ends in the same way.
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what the
/// summary writes.
pub(super) fn render(summary: &GroupSummary, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut problems = 0;
    let mut stops = 0;

    for member in &summary.members {
        match member.outcome {
            Outcome::Failed { .. } => {
                problems += 1;
                writeln!(formatter, "{}: found problems", member.action)?;
            }
            Outcome::Errored { .. } => {
                stops += 1;
                writeln!(formatter, "{}: stopped", member.action)?;
            }
            Outcome::Passed { .. } | Outcome::Changed { .. } | Outcome::Skipped { .. } => {}
        }
    }

    let total = summary.members.len();
    write!(formatter, "{}: ", summary.group)?;

    match (problems, stops) {
        (0, 0) => render_share(total, total, "passed", formatter),
        (0, stops) => render_share(stops, total, "stopped", formatter),
        (problems, 0) => render_share(problems, total, "found problems", formatter),
        (problems, stops) => {
            render_share(problems, total, "found problems", formatter)?;
            write!(formatter, ", {stops} stopped")
        }
    }
}

/// Writes how many of the members of a run ended as the verdict says, such
/// as `2 of 11 actions found problems`
///
/// # Errors
///
/// Returns the error of the formatter when the formatter cannot take what the
/// share writes.
fn render_share(
    count: usize,
    total: usize,
    verdict: &str,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    write!(formatter, "{count} of ")?;
    render_count(total, "action", formatter)?;
    write!(formatter, " {verdict}")
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use rakko::{Name, SkipReason, action_name};

    use super::*;
    use crate::report::Report;

    /// Returns the report of a member that found a problem
    fn failed(name: Name) -> Report {
        Report::new(
            name,
            Outcome::Failed {
                findings: Vec::new(),
                repairs: Vec::new(),
            },
        )
    }

    /// Returns the report of a member that passed
    fn passed(name: Name) -> Report {
        Report::new(name, Outcome::Passed { summary: None })
    }

    /// Returns the text of the summary of a run of the group `check`
    fn rendered(members: Vec<Report>) -> String {
        GroupSummary::new(action_name!("check"), members).to_string()
    }

    /// Returns the report of a member that stopped
    fn stopped(name: Name) -> Report {
        Report::new(
            name,
            Outcome::Errored {
                source: Box::new(std::io::Error::other("the tool is missing")),
            },
        )
    }

    #[test]
    fn render_with_a_member_that_found_problems_gives_the_member() {
        let text = rendered(vec![
            passed(action_name!("lint-toml")),
            failed(action_name!("lint-yaml")),
        ]);

        assert_eq!(
            text,
            "lint-yaml: found problems\n\
             check: 1 of 2 actions found problems"
        );
    }

    #[test]
    fn render_with_a_member_that_stopped_gives_the_member() {
        let text = rendered(vec![
            stopped(action_name!("test-rust")),
            passed(action_name!("lint-toml")),
        ]);

        assert_eq!(
            text,
            "test-rust: stopped\n\
             check: 1 of 2 actions stopped"
        );
    }

    #[test]
    fn render_with_members_that_changed_or_skipped_counts_them_as_passed() {
        let text = rendered(vec![
            Report::new(
                action_name!("format-yaml"),
                Outcome::Changed {
                    repairs: Vec::new(),
                },
            ),
            Report::new(
                action_name!("lint-yaml"),
                Outcome::Skipped {
                    reason: SkipReason::new("the project has no YAML file"),
                },
            ),
        ]);

        assert_eq!(text, "check: 2 of 2 actions passed");
    }

    #[test]
    fn render_with_members_that_did_not_pass_gives_them_in_the_order_of_the_list() {
        let text = rendered(vec![
            stopped(action_name!("test-rust")),
            failed(action_name!("lint-yaml")),
        ]);

        assert!(text.starts_with("test-rust: stopped\nlint-yaml: found problems\n"));
    }

    #[test]
    fn render_with_members_that_found_problems_and_stopped_counts_both() {
        let text = rendered(vec![
            failed(action_name!("lint-yaml")),
            passed(action_name!("lint-toml")),
            stopped(action_name!("test-rust")),
        ]);

        assert_eq!(
            text,
            "lint-yaml: found problems\n\
             test-rust: stopped\n\
             check: 1 of 3 actions found problems, 1 stopped"
        );
    }

    #[test]
    fn render_with_members_that_passed_counts_them() {
        let text = rendered(vec![
            passed(action_name!("lint-yaml")),
            passed(action_name!("lint-toml")),
        ]);

        assert_eq!(text, "check: 2 of 2 actions passed");
    }

    #[test]
    fn render_with_one_member_counts_it_in_the_singular() {
        let text = rendered(vec![passed(action_name!("lint-yaml"))]);

        assert_eq!(text, "check: 1 of 1 action passed");
    }

    #[test]
    fn render_without_members_counts_none() {
        let text = rendered(Vec::new());

        assert_eq!(text, "check: 0 of 0 actions passed");
    }
}
