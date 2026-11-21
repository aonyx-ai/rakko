//! The lines of the body of a pull request that opt units out of the check
//!
//! A change that does not reach users, such as a refactor, needs no
//! changeset. The author then says so in the body of the pull request, with a
//! line that gives a reason. The line `No changeset for <unit>: <reason>` opts
//! one unit out. The line `No changeset: <reason>` opts out the whole pull
//! request, so that a refactor across many crates needs no line per crate.

mod line;
mod reason;
mod unit_opt_out;

use getset::Getters;

pub(crate) use self::line::OptOutLine;
pub(crate) use self::reason::Reason;
pub(crate) use self::unit_opt_out::UnitOptOut;

/// The start of every opt-out line
const START: &str = "No changeset";

/// The text after the start of a line that opts one unit out
const FOR: &str = " for ";

/// The mark between the subject of a line and its reason
const COLON: char = ':';

/// The mark that Markdown puts around code, which an author can put around
/// the name of a unit
const BACKTICK: char = '`';

/// The opt-out lines of the body of a pull request
///
/// Each line of the body counts on its own, without the spaces at its ends.
/// A line opts out only when it starts with `No changeset` in exactly this
/// case. Every other line is text for the reviewer and changes nothing.
#[derive(Clone, Eq, PartialEq, Debug, Default, Getters)]
pub(crate) struct OptOuts {
    /// The reason of the line that opts out the whole pull request, or `None`
    /// when no line does
    ///
    /// When several lines opt out the whole pull request, the first one counts.
    #[getset(get = "pub(crate)")]
    everything: Option<Reason>,

    /// The lines that opt out one unit each, in the order of the body
    #[getset(get = "pub(crate)")]
    units: Vec<UnitOptOut>,

    /// The lines that start an opt-out and give no reason, in the order of the
    /// body
    #[getset(get = "pub(crate)")]
    blanks: Vec<OptOutLine>,
}

impl OptOuts {
    /// Reads the opt-out lines of the body of a pull request
    ///
    /// A line for one unit names the unit before the colon. The name can stand
    /// in backticks, as Markdown writes code. A line of one of the two forms
    /// whose reason is empty opts nothing out, and the reader keeps it among
    /// the blanks. So does a line for one unit that has no colon. A line that
    /// continues `No changeset` in another way is text for the reviewer.
    pub(crate) fn read(body: &str) -> Self {
        let mut everything = None;
        let mut units = Vec::new();
        let mut blanks = Vec::new();

        for line in body.lines().map(str::trim) {
            let Some(rest) = line.strip_prefix(START) else {
                continue;
            };

            if let Some(reason) = rest.strip_prefix(COLON) {
                match reason_of(reason) {
                    Some(reason) => {
                        everything.get_or_insert(reason);
                    }
                    None => blanks.push(OptOutLine::new(line)),
                }
                continue;
            }

            let Some(subject) = rest.strip_prefix(FOR) else {
                continue;
            };

            let opt_out = subject
                .split_once(COLON)
                .and_then(|(unit, reason)| Some((name_of(unit)?, reason_of(reason)?)));

            match opt_out {
                Some((unit, reason)) => {
                    units.push(UnitOptOut::builder().unit(unit).reason(reason).build());
                }
                None => blanks.push(OptOutLine::new(line)),
            }
        }

        Self {
            everything,
            units,
            blanks,
        }
    }

    /// Returns the reason that opts out the unit, or `None` when nothing does
    ///
    /// A line for the unit itself comes before the line for the whole pull
    /// request, because it says more about that unit.
    pub(crate) fn reason_for(&self, unit: &str) -> Option<&Reason> {
        let Self {
            everything, units, ..
        } = self;

        units
            .iter()
            .find(|opt_out| opt_out.unit().get() == unit)
            .map(UnitOptOut::reason)
            .or(everything.as_ref())
    }
}

/// Returns the name of a unit before the colon of a line, or `None` when it
/// is empty
fn name_of(text: &str) -> Option<&str> {
    let name = text.trim().trim_matches(BACKTICK).trim();

    (!name.is_empty()).then_some(name)
}

/// Returns the reason after the colon of a line, or `None` when it is empty
fn reason_of(text: &str) -> Option<Reason> {
    let reason = text.trim();

    (!reason.is_empty()).then(|| Reason::new(reason))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Returns the units and the reasons of the unit lines of a body
    fn units_of(body: &str) -> Vec<(String, String)> {
        OptOuts::read(body)
            .units()
            .iter()
            .map(|opt_out| {
                (
                    opt_out.unit().get().to_owned(),
                    opt_out.reason().get().to_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn opt_outs_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<OptOuts>();
    }

    #[test]
    fn read_of_a_body_without_opt_outs_opts_nothing_out() {
        let opt_outs = OptOuts::read("Refactor the parser.\n\nNo changes to users.\n");

        assert_eq!(opt_outs, OptOuts::default());
    }

    #[test]
    fn read_of_a_line_for_a_unit_in_backticks_reads_the_name() {
        let units = units_of("No changeset for `rakko`: only a refactor");

        assert_eq!(units, [("rakko".to_owned(), "only a refactor".to_owned())]);
    }

    #[test]
    fn read_of_a_line_for_a_unit_reads_the_unit_and_the_reason() {
        let units = units_of("No changeset for rakko-cli: only a refactor\n");

        assert_eq!(
            units,
            [("rakko-cli".to_owned(), "only a refactor".to_owned())]
        );
    }

    #[test]
    fn read_of_a_line_for_a_unit_without_a_colon_keeps_it_as_a_blank() {
        let opt_outs = OptOuts::read("No changeset for rakko\n");

        assert_eq!(
            opt_outs.blanks(),
            &[OptOutLine::new("No changeset for rakko")]
        );
    }

    #[test]
    fn read_of_a_line_for_a_unit_without_a_name_keeps_it_as_a_blank() {
        let opt_outs = OptOuts::read("No changeset for : only a refactor\n");

        assert_eq!(
            opt_outs.blanks(),
            &[OptOutLine::new("No changeset for : only a refactor")]
        );
    }

    #[test]
    fn read_of_a_line_for_a_unit_without_a_reason_keeps_it_as_a_blank() {
        let opt_outs = OptOuts::read("No changeset for rakko:  \n");

        assert_eq!(
            opt_outs.blanks(),
            &[OptOutLine::new("No changeset for rakko:")]
        );
    }

    #[test]
    fn read_of_a_line_for_everything_reads_the_reason() {
        let opt_outs = OptOuts::read("Some text\r\n  No changeset: a refactor across crates  \r\n");

        assert_eq!(
            opt_outs.everything(),
            &Some(Reason::new("a refactor across crates"))
        );
    }

    #[test]
    fn read_of_a_line_for_everything_without_a_reason_keeps_it_as_a_blank() {
        let opt_outs = OptOuts::read("No changeset:\n");

        assert_eq!(opt_outs.blanks(), &[OptOutLine::new("No changeset:")]);
    }

    #[test]
    fn read_of_a_line_in_another_case_opts_nothing_out() {
        let opt_outs = OptOuts::read("no changeset: a refactor\n");

        assert_eq!(opt_outs, OptOuts::default());
    }

    #[test]
    fn read_of_a_line_that_continues_the_start_opts_nothing_out() {
        let opt_outs = OptOuts::read("No changesets were needed: a refactor\n");

        assert_eq!(opt_outs, OptOuts::default());
    }

    #[test]
    fn read_of_two_lines_for_everything_keeps_the_first_reason() {
        let opt_outs = OptOuts::read("No changeset: the first\nNo changeset: the second\n");

        assert_eq!(opt_outs.everything(), &Some(Reason::new("the first")));
    }

    #[test]
    fn reason_for_a_unit_of_its_own_line_prefers_that_line() {
        let opt_outs = OptOuts::read("No changeset: a refactor\nNo changeset for rakko: a test\n");

        let reason = opt_outs.reason_for("rakko");

        assert_eq!(reason, Some(&Reason::new("a test")));
    }

    #[test]
    fn reason_for_a_unit_without_a_line_is_the_reason_for_everything() {
        let opt_outs = OptOuts::read("No changeset: a refactor\n");

        let reason = opt_outs.reason_for("rakko");

        assert_eq!(reason, Some(&Reason::new("a refactor")));
    }

    #[test]
    fn reason_for_a_unit_without_any_line_is_none() {
        let opt_outs = OptOuts::read("No changeset for rakko-cli: a refactor\n");

        let reason = opt_outs.reason_for("rakko");

        assert_eq!(reason, None);
    }
}
