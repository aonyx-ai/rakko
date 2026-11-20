//! The judgment of a pull request against its release units
//!
//! This module holds no input and no output. It takes the listing of the
//! units that Otto gave, the files that Git compared, and the opt-out lines of the body.
//! It returns the outcome of the check.

use std::path::Path;

use rakko::{DirectoryPath, Finding, Location, Outcome, Summary};

use crate::change::ChangeKind;
use crate::comparison::Comparison;
use crate::opt_out::{OptOutLine, OptOuts, Reason, UnitOptOut};
use crate::release_unit::ReleaseUnit;
use crate::unit_listing::UnitListing;

/// The summary of a pull request that changes no release unit
const NO_UNIT: &str = "the pull request changes no release unit";

/// Returns the outcome of a pull request with the given changes and body
///
/// A unit counts as changed when the pull request changes a file that the
/// unit owns, as the listing decides. A changed unit passes when the pull request adds a changeset to it,
/// or when the body opts it out with a reason. The check fails with a finding
/// for each changed unit that does neither. It also fails for each opt-out
/// line that gives no reason, and for each line that opts out a unit that
/// Otto did not list. A passing check says in its summary how many units add
/// a changeset, and which units it accepted without one, with their reasons.
pub(crate) fn judge(listing: &UnitListing, comparison: &Comparison, opt_outs: &OptOuts) -> Outcome {
    let units = listing.units();
    let mut findings: Vec<Finding> = opt_outs.blanks().iter().map(blank).collect();

    findings.extend(
        opt_outs
            .units()
            .iter()
            .filter(|opt_out| !units.iter().any(|unit| unit.name() == opt_out.unit()))
            .map(unknown),
    );

    let mut described = 0_usize;
    let mut accepted = Vec::new();

    let changes = comparison.changes();
    let changed = units.iter().filter(|unit| {
        changes
            .iter()
            .any(|change| listing.owner(change.path()) == Some(*unit))
    });

    for unit in changed {
        let adds_changeset = changes
            .iter()
            .any(|change| change.kind() == ChangeKind::Added && unit.is_changeset(change.path()));

        if adds_changeset {
            described += 1;
            continue;
        }

        match opt_outs.reason_for(unit.name().get()) {
            Some(reason) => accepted.push((unit, reason)),
            None => findings.push(missing(unit, comparison.prefix())),
        }
    }

    if !findings.is_empty() {
        return Outcome::Failed {
            findings,
            repairs: Vec::new(),
        };
    }

    Outcome::Passed {
        summary: Some(summary(described, &accepted)),
    }
}

/// Returns the finding of an opt-out line that gives no reason
fn blank(line: &OptOutLine) -> Finding {
    Finding::builder()
        .message(format!(
            "the line `{line}` of the body of the pull request gives no reason, so it opts nothing out"
        ))
        .location(Location::Project)
        .build()
}

/// Returns the location of the root of a unit in the project
///
/// The root is relative to the top of the repository, and the prefix is the
/// path from that top to the root of the project. A unit at the root of the
/// project, or above it, has the whole project as its location.
fn location(root: &Path, prefix: &Path) -> Location {
    root.strip_prefix(prefix)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .and_then(|path| DirectoryPath::try_from(path).ok())
        .map_or(Location::Project, |path| Location::Directory { path })
}

/// Returns the finding of a changed unit without a changeset and without an
/// opt-out
fn missing(unit: &ReleaseUnit, prefix: &Path) -> Finding {
    let name = unit.name();
    let directory = if unit.root().as_os_str().is_empty() {
        ".changeset/".to_owned()
    } else {
        format!("{}/.changeset/", unit.root().display())
    };

    Finding::builder()
        .message(format!(
            "the pull request changes the release unit `{name}` and adds no changeset to \
             `{directory}`. Add one, or opt the unit out with the line \
             `No changeset for {name}: <reason>` in the body of the pull request"
        ))
        .location(location(unit.root(), prefix))
        .build()
}

/// Returns the summary of a passing check
fn summary(described: usize, accepted: &[(&ReleaseUnit, &Reason)]) -> Summary {
    let mut parts = Vec::new();

    match described {
        0 => {}
        1 => parts.push("1 changed release unit adds a changeset".to_owned()),
        count => parts.push(format!("{count} changed release units add a changeset")),
    }

    if !accepted.is_empty() {
        let units: Vec<String> = accepted
            .iter()
            .map(|(unit, reason)| format!("{} ({reason})", unit.name()))
            .collect();

        parts.push(format!(
            "accepted without a changeset: {}",
            units.join(", ")
        ));
    }

    if parts.is_empty() {
        return Summary::new(NO_UNIT);
    }

    Summary::new(parts.join("; "))
}

/// Returns the finding of a line that opts out a unit that Otto did not list
fn unknown(opt_out: &UnitOptOut) -> Finding {
    Finding::builder()
        .message(format!(
            "the body of the pull request opts out `{}`, which is no release unit of the repository",
            opt_out.unit()
        ))
        .location(Location::Project)
        .build()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::PathBuf;

    use super::*;
    use crate::change::Change;

    /// Returns a change of the given kind to a file
    fn change(path: &str, kind: ChangeKind) -> Change {
        Change::builder().path(path).kind(kind).build()
    }

    /// Returns a comparison of the given changes in a project at the top of
    /// its repository
    fn comparison(changes: Vec<Change>) -> Comparison {
        Comparison::builder().changes(changes).build()
    }

    /// Returns the messages of the findings of an outcome, or the summary of a
    /// passing outcome
    fn messages(outcome: &Outcome) -> Vec<String> {
        match outcome {
            Outcome::Failed { findings, .. } => findings
                .iter()
                .map(|finding| finding.message().get().to_owned())
                .collect(),
            Outcome::Passed {
                summary: Some(summary),
            } => vec![summary.get().to_owned()],
            other => vec![format!("{other:?}")],
        }
    }

    /// Returns the listing of the tests, with the units `alpha` and `beta`
    fn units() -> UnitListing {
        UnitListing::builder()
            .units(vec![unit("alpha"), unit("beta")])
            .build()
    }

    /// Returns an independent unit in `crates/<name>`
    fn unit(name: &str) -> ReleaseUnit {
        let root = format!("crates/{name}");

        ReleaseUnit::builder()
            .name(name)
            .manifest(format!("{root}/Cargo.toml"))
            .crates(vec![PathBuf::from(&root)])
            .root(root)
            .build()
    }

    #[test]
    fn judge_of_a_blank_opt_out_line_fails() {
        let changes = comparison(vec![change("README.md", ChangeKind::Other)]);

        let outcome = judge(&units(), &changes, &OptOuts::read("No changeset:\n"));

        assert_eq!(
            messages(&outcome),
            [
                "the line `No changeset:` of the body of the pull request gives no reason, so it opts nothing out"
            ]
        );
    }

    #[test]
    fn judge_of_a_change_in_an_opted_out_unit_shows_the_reason() {
        let changes = comparison(vec![change("crates/alpha/src/lib.rs", ChangeKind::Other)]);

        let outcome = judge(
            &units(),
            &changes,
            &OptOuts::read("No changeset for alpha: only a refactor\n"),
        );

        assert_eq!(
            messages(&outcome),
            ["accepted without a changeset: alpha (only a refactor)"]
        );
    }

    #[test]
    fn judge_of_a_change_in_an_unowned_directory_passes() {
        let listing = UnitListing::builder()
            .units(vec![unit("alpha")])
            .unowned(vec![PathBuf::from("crates/alpha/tests/fixture")])
            .build();
        let changes = comparison(vec![change(
            "crates/alpha/tests/fixture/src/lib.rs",
            ChangeKind::Other,
        )]);

        let outcome = judge(&listing, &changes, &OptOuts::default());

        assert_eq!(messages(&outcome), [NO_UNIT]);
    }

    #[test]
    fn judge_of_a_change_outside_every_unit_passes() {
        let changes = comparison(vec![
            change(".github/workflows/ci.yml", ChangeKind::Other),
            change("Cargo.lock", ChangeKind::Other),
        ]);

        let outcome = judge(&units(), &changes, &OptOuts::default());

        assert_eq!(messages(&outcome), [NO_UNIT]);
    }

    #[test]
    fn judge_of_a_change_to_the_workflows_of_a_lockstep_workspace_at_the_top_passes() {
        let listing = UnitListing::builder()
            .units(vec![
                ReleaseUnit::builder()
                    .name("demo")
                    .root("")
                    .manifest("Cargo.toml")
                    .crates(vec![PathBuf::from("crates/demo")])
                    .build(),
            ])
            .build();
        let changes = comparison(vec![change(".github/workflows/ci.yml", ChangeKind::Other)]);

        let outcome = judge(&listing, &changes, &OptOuts::default());

        assert_eq!(messages(&outcome), [NO_UNIT]);
    }

    #[test]
    fn judge_of_a_change_with_a_changeset_passes() {
        let changes = comparison(vec![
            change("crates/alpha/src/lib.rs", ChangeKind::Other),
            change("crates/alpha/.changeset/add-it.md", ChangeKind::Added),
        ]);

        let outcome = judge(&units(), &changes, &OptOuts::default());

        assert_eq!(
            messages(&outcome),
            ["1 changed release unit adds a changeset"]
        );
    }

    #[test]
    fn judge_of_a_change_with_an_edited_changeset_fails() {
        let changes = comparison(vec![change(
            "crates/alpha/.changeset/add-it.md",
            ChangeKind::Other,
        )]);

        let outcome = judge(&units(), &changes, &OptOuts::default());

        assert!(
            matches!(outcome, Outcome::Failed { .. }),
            "expected a failure, got {outcome:?}"
        );
    }

    #[test]
    fn judge_of_a_change_without_a_changeset_fails() {
        let changes = comparison(vec![change("crates/alpha/src/lib.rs", ChangeKind::Other)]);

        let outcome = judge(&units(), &changes, &OptOuts::default());

        assert_eq!(
            messages(&outcome),
            [
                "the pull request changes the release unit `alpha` and adds no changeset to \
              `crates/alpha/.changeset/`. Add one, or opt the unit out with the line \
              `No changeset for alpha: <reason>` in the body of the pull request"
            ]
        );
    }

    #[test]
    fn judge_of_a_change_without_a_changeset_locates_the_unit_in_the_project() {
        let changes = Comparison::builder()
            .prefix("crates")
            .changes(vec![change("crates/alpha/src/lib.rs", ChangeKind::Other)])
            .build();

        let outcome = judge(&units(), &changes, &OptOuts::default());

        let Outcome::Failed { findings, .. } = outcome else {
            panic!("expected a failure, got {outcome:?}");
        };
        assert_eq!(
            findings[0].location(),
            &Location::Directory {
                path: DirectoryPath::try_from("alpha").unwrap()
            }
        );
    }

    #[test]
    fn judge_of_a_changeset_in_another_unit_fails() {
        let changes = comparison(vec![
            change("crates/alpha/src/lib.rs", ChangeKind::Other),
            change("crates/beta/.changeset/add-it.md", ChangeKind::Added),
        ]);

        let outcome = judge(&units(), &changes, &OptOuts::default());

        assert!(
            matches!(&outcome, Outcome::Failed { findings, .. } if findings.len() == 1),
            "expected one finding for alpha, got {outcome:?}"
        );
    }

    #[test]
    fn judge_of_an_opt_out_of_an_unknown_unit_fails() {
        let changes = comparison(vec![change("crates/alpha/src/lib.rs", ChangeKind::Other)]);

        let outcome = judge(
            &units(),
            &changes,
            &OptOuts::read("No changeset: a refactor\nNo changeset for alpah: a typo\n"),
        );

        assert_eq!(
            messages(&outcome),
            [
                "the body of the pull request opts out `alpah`, which is no release unit of the repository"
            ]
        );
    }

    #[test]
    fn judge_of_changes_in_two_units_opted_out_as_a_whole_lists_both() {
        let changes = comparison(vec![
            change("crates/alpha/src/lib.rs", ChangeKind::Other),
            change("crates/beta/src/lib.rs", ChangeKind::Other),
        ]);

        let outcome = judge(
            &units(),
            &changes,
            &OptOuts::read("No changeset: a refactor across crates\n"),
        );

        assert_eq!(
            messages(&outcome),
            [
                "accepted without a changeset: alpha (a refactor across crates), beta (a refactor across crates)"
            ]
        );
    }

    #[test]
    fn judge_of_two_units_with_changesets_counts_both() {
        let changes = comparison(vec![
            change("crates/alpha/.changeset/add-it.md", ChangeKind::Added),
            change("crates/beta/.changeset/add-it.md", ChangeKind::Added),
        ]);

        let outcome = judge(&units(), &changes, &OptOuts::default());

        assert_eq!(
            messages(&outcome),
            ["2 changed release units add a changeset"]
        );
    }

    #[test]
    fn location_of_a_unit_at_the_root_of_the_project_is_the_project() {
        let location = location(Path::new("crates/alpha"), Path::new("crates/alpha"));

        assert_eq!(location, Location::Project);
    }

    #[test]
    fn location_of_a_unit_outside_the_project_is_the_project() {
        let location = location(Path::new("crates/alpha"), Path::new("tools"));

        assert_eq!(location, Location::Project);
    }

    #[test]
    fn missing_of_a_unit_at_the_top_names_the_changeset_directory_at_the_top() {
        let unit = ReleaseUnit::builder()
            .name("demo")
            .root("")
            .manifest("Cargo.toml")
            .crates(vec![PathBuf::from("crates/demo")])
            .build();

        let finding = missing(&unit, Path::new(""));

        assert!(
            finding
                .message()
                .get()
                .contains("adds no changeset to `.changeset/`"),
            "expected the directory at the top, got {}",
            finding.message().get()
        );
    }

    #[test]
    fn summary_of_a_unit_with_a_changeset_and_an_accepted_unit_names_both() {
        let unit = unit("beta");
        let reason = Reason::new("a test");

        let summary = summary(1, &[(&unit, &reason)]);

        assert_eq!(
            summary.get(),
            "1 changed release unit adds a changeset; accepted without a changeset: beta (a test)"
        );
    }
}
