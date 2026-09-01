/// One operation of taplo that an action runs
///
/// Taplo does more than one job, and an action wraps one of them. This
/// vocabulary names the jobs that Rakko uses, so that an action states what
/// it wants and never writes a command line of its own.
///
/// The two formatting operations differ in one thing: whether taplo writes.
/// A check reports the files that it would rewrite and leaves the project
/// alone, so a run that a user started in order to look changes nothing.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Operation {
    /// Report the files that taplo would format, and rewrite nothing
    CheckFormat,

    /// Rewrite the files that taplo can format
    Format,

    /// Report the files that taplo cannot read, parse, or validate
    Lint,
}
