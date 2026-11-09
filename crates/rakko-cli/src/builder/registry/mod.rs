//! The actions that a harness mounted

mod mounted;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt;

use rakko::Name;

pub(crate) use self::mounted::Mounted;

/// The names of the commands that the command line holds or will hold itself
///
/// The parser adds `help` to every command line with commands. The
/// projection adds `doctor` to every harness, to report and repair the tool
/// pins that a correct run needs. A command line cannot hold two commands
/// with one name. So the command line refuses an action, a command, or a
/// group with one of these names.
const RESERVED: [&str; 2] = ["doctor", "help"];

/// The actions, the commands, and the groups that a harness mounted
///
/// A registry holds one entry for each name. A harness mounts lists from
/// different crates. Two lists can carry the same action, and a command that
/// the harness wrote can take the name of an action. So the registry is where
/// a name gets one meaning.
#[derive(Default)]
pub(super) struct Registry {
    /// The entries, by the name that identifies each of them
    entries: BTreeMap<Name, Mounted>,
}

impl Registry {
    /// Adds actions, commands, and groups to the registry
    ///
    /// # Panics
    ///
    /// Panics when an entry carries the name of an entry that the registry
    /// already holds, and reports that name. A harness that mounts one name
    /// twice has a defect that only a change of its own code corrects. So the
    /// failure happens where the harness mounts and not where a user runs.
    ///
    /// Panics when an entry carries a name that the command line reserves for
    /// itself, and reports the entry.
    pub(super) fn add(&mut self, entries: impl IntoIterator<Item = Mounted>) {
        for mounted in entries {
            assert!(
                !RESERVED.contains(&mounted.name().get()),
                "the harness mounts the {mounted}, whose name the command line reserves for itself"
            );

            match self.entries.entry(mounted.name()) {
                Entry::Vacant(entry) => {
                    entry.insert(mounted);
                }
                Entry::Occupied(entry) => {
                    panic!(
                        "the harness mounts two actions, commands, or groups with the name '{}'",
                        entry.key()
                    )
                }
            }
        }
    }

    /// Returns the entries of the registry, by name
    pub(super) fn entries(&self) -> impl Iterator<Item = &Mounted> {
        self.entries.values()
    }

    /// Removes the entry with the given name and returns it
    ///
    /// A run drives one action, one command, or one group. The run owns it
    /// for as long as it lasts, so the registry gives the entry away instead
    /// of lending it. The method returns [`None`] when the registry holds no
    /// entry for the name.
    pub(super) fn take(&mut self, name: &str) -> Option<Mounted> {
        let name: Name = name.parse().ok()?;

        self.entries.remove(&name)
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Registry")
            .field("entries", &self.entries.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Registry>();
    }
}
