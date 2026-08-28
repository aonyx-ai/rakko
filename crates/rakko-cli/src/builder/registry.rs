use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt;

use rakko_action::{ErasedAction, Name};

/// The actions that a harness mounted
///
/// A registry holds one action for each name. A harness mounts lists that come
/// from different crates, and two of those lists can carry the same action, so
/// the registry is where a name gets one meaning.
#[derive(Default)]
pub(super) struct Registry {
    /// The actions, by the name that identifies each of them
    actions: BTreeMap<Name, Box<dyn ErasedAction>>,
}

impl Registry {
    /// Adds actions to the registry
    ///
    /// # Panics
    ///
    /// Panics when an action carries the name of an action that the registry
    /// already holds, and reports that name. A harness that mounts one name
    /// twice has a defect that only a change of its own code corrects, so the
    /// failure happens where the harness mounts and not where a user runs.
    pub(super) fn add(&mut self, actions: impl IntoIterator<Item = Box<dyn ErasedAction>>) {
        for action in actions {
            match self.actions.entry(action.name()) {
                Entry::Vacant(entry) => {
                    entry.insert(action);
                }
                Entry::Occupied(entry) => {
                    panic!(
                        "the harness mounts two actions with the name '{}'",
                        entry.key()
                    )
                }
            }
        }
    }

    /// Returns the actions of the registry, by name
    pub(super) fn actions(&self) -> impl Iterator<Item = &dyn ErasedAction> {
        self.actions.values().map(Box::as_ref)
    }

    /// Removes the action with the given name and returns it
    ///
    /// A run drives one action, and the run owns it for as long as it lasts,
    /// so the registry gives the action away instead of lending it. The method
    /// returns [`None`] when the registry holds no action for the name.
    pub(super) fn take(&mut self, name: &str) -> Option<Box<dyn ErasedAction>> {
        let name: Name = name.parse().ok()?;

        self.actions.remove(&name)
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Registry")
            .field("actions", &self.actions.keys().collect::<Vec<_>>())
            .finish()
    }
}
