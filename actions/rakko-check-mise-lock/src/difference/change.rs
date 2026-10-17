/// What `mise lock` did to one part of a lockfile
///
/// The variant says whether the part was missing before the run, is missing
/// after it, or is there on both sides with other content.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Change {
    /// `mise lock` wrote a part that the lockfile did not hold
    Added,

    /// `mise lock` wrote other content for a part that the lockfile held
    Changed,

    /// `mise lock` took away a part that the lockfile held
    Removed,
}
