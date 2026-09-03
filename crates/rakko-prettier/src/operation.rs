/// One operation of prettier that an action runs
///
/// Prettier does one job in two ways, and an action wraps one of them. This
/// vocabulary names the ways, so that an action states what it wants and
/// never writes a command line of its own.
///
/// The two operations differ in one thing: whether prettier writes. A report
/// names the files that a rewrite would change and leaves the project alone,
/// so a run that a user started in order to look changes nothing. A rewrite
/// formats what it can and says which files it changed.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Operation {
    /// Report the files that prettier would change, and rewrite nothing
    Report,

    /// Rewrite the files that prettier can format
    Rewrite,
}
