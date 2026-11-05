//! One difference between a lockfile before and after `mise lock`
//!
//! The comparison of a lockfile produces one difference for each part that
//! `mise lock` added, changed, or removed. A difference becomes a finding of
//! a run that only reports, and a repair of a run that fixes.

mod change;

use bon::Builder;
use getset::{CopyGetters, Getters};

pub use self::change::Change;
use crate::part::Part;

/// One part of a lockfile that `mise lock` added, changed, or removed
///
/// The value names the part and what happened to it, and it gives the text
/// that a finding and a repair carry.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct Difference {
    /// The part that `mise lock` touched
    #[getset(get = "pub")]
    part: Part,

    /// What `mise lock` did to the part
    #[getset(get_copy = "pub")]
    change: Change,
}

impl Difference {
    /// Returns the message of the finding that reports the difference
    ///
    /// The message says what `mise lock` would do to the lockfile, and names
    /// the tool, the version, and the platform of the part, such as
    /// `mise lock adds the entry of jq 1.8.1 for linux-x64`.
    pub fn finding(&self) -> String {
        let verb = match self.change {
            Change::Added => "adds",
            Change::Changed => "changes",
            Change::Removed => "removes",
        };

        format!("mise lock {verb} the entry of {}", self.part)
    }

    /// Returns the message of the repair that reports the difference
    ///
    /// The message says what `mise lock` did to the lockfile, such as
    /// `mise lock added the entry of jq 1.8.1 for linux-x64`.
    pub fn repair(&self) -> String {
        let verb = match self.change {
            Change::Added => "added",
            Change::Changed => "changed",
            Change::Removed => "removed",
        };

        format!("mise lock {verb} the entry of {}", self.part)
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;
    use crate::part::{Platform, ToolName, Version};

    /// Returns the part of jq 1.8.1 for linux-x64
    fn part() -> Part {
        Part::builder()
            .tool(ToolName::new("jq"))
            .version(Version::new("1.8.1"))
            .platform(Platform::new("linux-x64"))
            .build()
    }

    #[test]
    fn difference_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Difference>();
    }

    #[test]
    fn finding_of_a_changed_part_says_that_mise_changes_it() {
        let difference = Difference::builder()
            .part(part())
            .change(Change::Changed)
            .build();

        assert_eq!(
            difference.finding(),
            "mise lock changes the entry of jq 1.8.1 for linux-x64"
        );
    }

    #[test]
    fn finding_of_a_removed_part_says_that_mise_removes_it() {
        let difference = Difference::builder()
            .part(part())
            .change(Change::Removed)
            .build();

        assert_eq!(
            difference.finding(),
            "mise lock removes the entry of jq 1.8.1 for linux-x64"
        );
    }

    #[test]
    fn finding_of_an_added_part_says_that_mise_adds_it() {
        let difference = Difference::builder()
            .part(part())
            .change(Change::Added)
            .build();

        assert_eq!(
            difference.finding(),
            "mise lock adds the entry of jq 1.8.1 for linux-x64"
        );
    }

    #[test]
    fn repair_of_a_changed_part_says_that_mise_changed_it() {
        let difference = Difference::builder()
            .part(part())
            .change(Change::Changed)
            .build();

        assert_eq!(
            difference.repair(),
            "mise lock changed the entry of jq 1.8.1 for linux-x64"
        );
    }

    #[test]
    fn repair_of_a_removed_part_says_that_mise_removed_it() {
        let difference = Difference::builder()
            .part(part())
            .change(Change::Removed)
            .build();

        assert_eq!(
            difference.repair(),
            "mise lock removed the entry of jq 1.8.1 for linux-x64"
        );
    }

    #[test]
    fn repair_of_an_added_part_says_that_mise_added_it() {
        let difference = Difference::builder()
            .part(part())
            .change(Change::Added)
            .build();

        assert_eq!(
            difference.repair(),
            "mise lock added the entry of jq 1.8.1 for linux-x64"
        );
    }
}
