//! What a run of `mise lock` that ended with success did

use std::collections::{BTreeMap, BTreeSet};

use rakko::{FilePath, SkipReason};

use crate::part::Platform;

/// What a run of `mise lock` that ended with success did
///
/// Mise ends with success both when it wrote the lockfiles of the project and
/// when the project configures nothing to lock. The two mean opposite things
/// for the outcome of the action, so the value tells them apart.
#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) enum Lock {
    /// Mise locked the tools of the project and wrote its lockfiles
    Written {
        /// The platforms that mise targeted for each lockfile, by the path of
        /// the lockfile
        ///
        /// Mise names them in its report of the run. A lockfile that the
        /// report does not name has no entry here.
        targets: BTreeMap<FilePath, BTreeSet<Platform>>,
    },

    /// Mise found no tool to lock in the project
    Unconfigured {
        /// What mise wrote about the project
        reason: SkipReason,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Lock>();
    }
}
