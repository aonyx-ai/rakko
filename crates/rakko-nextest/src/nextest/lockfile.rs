/// What a run does with the lockfile of the workspace
///
/// Cargo resolves the dependencies of a build and records the versions that
/// it chose in a lockfile. A run that answers for the project as it stands
/// lets cargo do that, because the resolution is part of the build that a
/// contributor gets.
///
/// A run that answers for a resolution which another job produced needs the
/// opposite. The versions of that resolution are the subject of the run, so a
/// build that quietly chose another version would answer a question that
/// nobody asked. Cargo then refuses the build instead, and the caller reads a
/// run that reported nothing as a run it cannot answer from.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Lockfile {
    /// Cargo resolves what the build needs and writes the lockfile
    Writable,

    /// Cargo builds the versions that the lockfile holds, and the run ends
    /// without success when the build would need another one
    Locked,
}
