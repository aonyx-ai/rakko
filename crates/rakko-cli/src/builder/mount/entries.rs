//! The entries that one value adds to the command line

use std::fmt;

use crate::builder::registry::Mounted;

/// The entries that one value adds to the command line
///
/// [`Mount`] returns this type, and no code outside the crate can build it
/// or refer to it. So no other crate can implement the trait, and the kinds
/// of value that the builder takes stay the ones that this crate defines.
///
/// [`Mount`]: super::Mount
pub struct Entries(Vec<Mounted>);

impl Entries {
    /// Creates the entries from the given list, in its order
    pub(crate) fn new(entries: Vec<Mounted>) -> Self {
        Self(entries)
    }

    /// Returns the entries, in the order in which the value listed them
    pub(crate) fn into_vec(self) -> Vec<Mounted> {
        self.0
    }
}

/// Shows the kind and the name of each entry, because an entry hides the type
/// of what it holds
impl fmt::Debug for Entries {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entries: Vec<String> = self.0.iter().map(ToString::to_string).collect();

        formatter.debug_tuple("Entries").field(&entries).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Entries>();
    }
}
