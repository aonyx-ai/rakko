use std::io;
use std::path::PathBuf;

use rakko_tool::RunCommandError;
use rakko_worktree::CreateWorktreeError;
use thiserror::Error;

use crate::lockfile::ReadLockfileError;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a gap in a lockfile. An entry that `mise lock` adds,
/// changes, or removes travels as a finding in the outcome of the run. The
/// variants of this error describe a run whose answer cannot be trusted, and
/// such a run stops instead of reporting one.
#[derive(Debug, Error)]
pub enum CheckMiseLockError {
    /// The copy of the project could not be created
    ///
    /// A run without `fix` locks the project in a copy, so that the checkout
    /// of a contributor keeps what they left in it. A run without that copy
    /// would have to write where the contributor works, so it stops here
    /// instead.
    #[error("failed to create a copy of the project to work in")]
    CopyUnavailable {
        /// The cause of the failure
        source: CreateWorktreeError,
    },

    /// Mise named a lockfile outside the directory that it ran in
    ///
    /// A finding names its lockfile relative to the project root, and a path
    /// outside the root has no such name. A copy of the project also holds no
    /// file outside the project, so the run could not compare the lockfile.
    #[error("mise named a lockfile outside the project: {}", path.display())]
    ForeignLockfile {
        /// The path that mise named
        path: PathBuf,
    },

    /// A lockfile that mise named could not be read from the disk
    #[error("failed to read the lockfile {}", path.display())]
    InaccessibleLockfile {
        /// The path of the lockfile, relative to the project root
        path: PathBuf,

        /// The cause of the failure
        source: io::Error,
    },

    /// Mise did not start
    ///
    /// The program did not start, or it started and its output could not be
    /// read. Nothing of the project was locked.
    #[error("failed to run mise")]
    MiseUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Mise ended without success when it was asked for the lockfiles
    ///
    /// A configuration that nobody trusted, a configuration that mise cannot
    /// read, and a lockfile that mise cannot read all end here.
    #[error("mise did not name the lockfiles of the project: {details}")]
    UnlistedLockfiles {
        /// What mise wrote
        details: String,
    },

    /// Mise ended without success when it was asked to lock the project
    #[error("mise did not lock the project: {details}")]
    UnlockedProject {
        /// What mise wrote
        details: String,
    },

    /// A lockfile records a targeted platform without the address of its
    /// download after the run
    ///
    /// An install in locked mode refuses such a platform, and `mise lock`
    /// writes or keeps one when it could not resolve the tool, even when it
    /// gives no warning, such as for a release that does not exist. The run
    /// cannot tell what the project lacks from what the network failed to
    /// give, so it stops. An entry that a second run does not repair belongs
    /// to a release or a platform without a download. A contributor then
    /// checks that the release and its download exist, removes the table of
    /// that platform, or removes the platform from `lockfile_platforms`.
    #[error(
        "a lockfile records a platform without the address of its download after mise lock: {parts}"
    )]
    UnaddressedPlatforms {
        /// The parts without an address, each with its lockfile
        parts: String,
    },

    /// Mise named no platform that it targeted for a lockfile with platforms
    ///
    /// Mise names the platforms that it targets for each lockfile in its
    /// report of the run, and the run needs them to tell an entry of a
    /// targeted platform from an old one. A report in a shape that the action
    /// does not read points at a version of mise that this crate does not
    /// know, so the run stops rather than pass over an entry without an
    /// address.
    #[error("mise named no platform that it targeted for the lockfile {}", path.display())]
    UntargetedLockfile {
        /// The path of the lockfile, relative to the project root
        path: PathBuf,
    },

    /// Mise gave a warning that is not one of the harmless kinds
    ///
    /// Mise ends with success when it could not resolve a tool, such as one
    /// that it failed to fetch what it needs for, or one for which an API
    /// limited the rate of its requests, and it leaves the entry of that
    /// platform as the lockfile had it. A gap in that entry then stays on both
    /// sides of the comparison, and a run that went on would pass over it. Any
    /// warning that is not one of the harmless kinds, which include the notice
    /// of an older format of the lockfile, can be such a sign, a new kind of
    /// warning of a later version of mise among them. A failure of the network
    /// is temporary, and a second run answers the question. A cause such as a
    /// release without the attestations that mise checks lasts, and the words
    /// of mise say which of the two it is.
    #[error(
        "mise warned while it locked the project, so it may not have resolved every platform: {details}"
    )]
    UnexpectedWarnings {
        /// What mise wrote
        details: String,
    },

    /// Mise wrote a list of lockfiles that the action cannot read
    ///
    /// The shape of the list belongs to a version of mise. A list that the
    /// action cannot read points at a version that this crate does not know,
    /// and a comparison of the lockfiles that the reading happened to find
    /// would hide the others.
    #[error("failed to read the list of lockfiles that mise wrote: {listing}")]
    UnreadableListing {
        /// What mise wrote in place of a list that the action reads
        listing: String,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// A lockfile has a shape that the action cannot read
    ///
    /// A comparison of the entries that the reading happened to understand
    /// would hide gaps behind a green result.
    #[error("failed to read the entries of the lockfile {}", path.display())]
    UnreadableLockfile {
        /// The path of the lockfile, relative to the project root
        path: PathBuf,

        /// The cause of the failure
        source: ReadLockfileError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // An action puts the error in the outcome of a run, and that outcome holds
    // an error that another thread can read. This test holds the error to the
    // auto traits that make this possible, because a field of a later version
    // could take them away without a word from the compiler.
    #[test]
    fn check_mise_lock_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<CheckMiseLockError>();
    }
}
