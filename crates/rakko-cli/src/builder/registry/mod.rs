/// One thing that a harness mounted: an action or a command
mod mounted;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt;

use rakko_action::Name;

pub(crate) use self::mounted::Mounted;

/// The actions and the commands that a harness mounted
///
/// A registry holds one entry for each name. A harness mounts lists that come
/// from different crates, two of those lists can carry the same action, and a
/// command that the harness wrote can take the name of an action, so the
/// registry is where a name gets one meaning.
#[derive(Default)]
pub(super) struct Registry {
    /// The entries, by the name that identifies each of them
    entries: BTreeMap<Name, Mounted>,
}

impl Registry {
    /// Adds actions and commands to the registry
    ///
    /// # Panics
    ///
    /// Panics when an entry carries the name of an entry that the registry
    /// already holds, and reports that name. A harness that mounts one name
    /// twice has a defect that only a change of its own code corrects, so the
    /// failure happens where the harness mounts and not where a user runs.
    // cli[impl mount.collision+2]
    pub(super) fn add(&mut self, entries: impl IntoIterator<Item = Mounted>) {
        for mounted in entries {
            match self.entries.entry(mounted.name()) {
                Entry::Vacant(entry) => {
                    entry.insert(mounted);
                }
                Entry::Occupied(entry) => {
                    panic!(
                        "the harness mounts two actions or commands with the name '{}'",
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
    /// A run drives one action or one command, and the run owns it for as
    /// long as it lasts, so the registry gives the entry away instead of
    /// lending it. The method returns [`None`] when the registry holds no
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
