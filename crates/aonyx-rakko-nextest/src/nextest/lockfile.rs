//! What a run does with the lockfile of the workspace

/// What a run does with the lockfile of the workspace
///
/// Cargo resolves the dependencies of a build and records the versions that it
/// chose in a lockfile. A run that only examines the project holds cargo to the
/// versions that the lockfile records. Otherwise a build that chose another
/// version would answer a question that nobody asked, and it would change a
/// file of the project as well. Two examples are a run that tests the code as
/// the project committed it and a run that tests a resolution which another job
/// produced.
///
/// Cargo refuses such a build instead, and it refuses a workspace without a
/// lockfile too. The run then reports nothing that the crate can answer
/// from, and the observation stops with a report that it does not
/// recognize. The standard error stream of that report carries the refusal
/// of cargo, which [`refused`][refused] recognizes.
///
/// A run that may change the project can let cargo resolve the dependencies
/// and write the lockfile instead.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_cargo::lockfile;
/// use aonyx_rakko_nextest::Lockfile;
///
/// let flag = match Lockfile::Locked {
///     Lockfile::Locked => Some(lockfile::LOCKED),
///     Lockfile::Writable => None,
/// };
///
/// assert_eq!(flag, Some("--locked"));
/// ```
///
/// [refused]: aonyx_rakko_cargo::lockfile::refused
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Lockfile {
    /// Cargo resolves what the build needs and writes the lockfile
    Writable,

    /// Cargo builds the versions that the lockfile holds, and the run ends
    /// without success when the build would need another one
    Locked,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Lockfile>();
    }
}
