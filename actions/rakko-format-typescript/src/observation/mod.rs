//! What one run of oxfmt produced
//!
//! Oxfmt reports over both of its streams: the files that a run listed travel
//! on the standard output stream, and everything that oxfmt could not do
//! travels on the standard error stream. This module turns both into data. The
//! reading recognizes what carries an answer and ignores the rest, so a line
//! that a new version adds does not break it. What the reading cannot find is
//! absent from the observation, and the caller decides what the absence means.

mod report;

use bon::bon;
use getset::{CopyGetters, Getters};
use rakko_tool::Execution;

use crate::problem::OxfmtProblem;

/// Whether oxfmt ended with success
///
/// # Examples
///
/// ```
/// use rakko_format_typescript::observation::Exit;
///
/// assert_eq!(Exit::default(), Exit::Failed);
///
/// let succeeded = match Exit::Succeeded {
///     Exit::Succeeded => true,
///     Exit::Failed => false,
/// };
/// assert!(succeeded);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Exit {
    /// Oxfmt ended with success
    Succeeded,

    /// Oxfmt ended with failure
    #[default]
    Failed,
}

/// Whether oxfmt matched the pattern of the run
///
/// # Examples
///
/// ```
/// use rakko_format_typescript::observation::Pattern;
///
/// assert_eq!(Pattern::default(), Pattern::Matched);
///
/// let nothing_to_examine = match Pattern::Unmatched {
///     Pattern::Matched => false,
///     Pattern::Unmatched => true,
/// };
/// assert!(nothing_to_examine);
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub enum Pattern {
    /// Oxfmt matched the pattern that the run named
    #[default]
    Matched,

    /// Oxfmt did not match the pattern
    Unmatched,
}

/// What one run of oxfmt produced
///
/// The value holds the answer of one run: whether oxfmt ended with success,
/// what it said about a configuration that it refused, whether it refused the
/// pattern of the run, the problems that it named, and the text that it wrote
/// about its failures.
///
/// An observation describes the run and judges nothing. A file that is not
/// formatted and a file that oxfmt refused are both problems of the project,
/// and the action that asked for the run decides what they mean for its
/// outcome.
///
/// # Examples
///
/// ```
/// use rakko_format_typescript::Observation;
/// use rakko_format_typescript::observation::{Exit, Pattern};
///
/// let observation = Observation::builder().exit(Exit::Succeeded).build();
///
/// assert_eq!(observation.exit(), Exit::Succeeded);
/// assert_eq!(observation.pattern(), Pattern::Matched);
/// assert!(observation.problems().is_empty());
/// ```
#[derive(Clone, Eq, PartialEq, Debug, CopyGetters, Getters)]
pub struct Observation {
    /// The problems that oxfmt reported, in the order of the report
    #[getset(get = "pub")]
    problems: Vec<OxfmtProblem>,

    /// What oxfmt said about a configuration file that it refused
    ///
    /// Oxfmt formats nothing at all when it cannot accept a configuration, so
    /// this is a run that never looked at a file of the project.
    #[getset(get = "pub")]
    rejected_configuration: Option<String>,

    /// What oxfmt wrote to its standard error stream
    #[getset(get = "pub")]
    stderr: String,

    /// Whether oxfmt ended with success
    #[getset(get_copy = "pub")]
    exit: Exit,

    /// Whether oxfmt matched the pattern of the run
    ///
    /// Oxfmt treats a pattern that matches nothing as an error, so this is how
    /// a run reports that the project holds no file of the language. A caller
    /// answers it as a run that had nothing to examine, not as a failure.
    #[getset(get_copy = "pub")]
    pattern: Pattern,
}

#[bon]
impl Observation {
    /// Creates the observation of a run
    ///
    /// A run of oxfmt builds one through [`read`][read], and a caller builds
    /// one where a test stands in for an oxfmt that nobody started. Every part
    /// is optional, so a test names what its case is about and leaves the rest
    /// at the answer of a run that reported nothing.
    ///
    /// [read]: Observation::read
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_format_typescript::Observation;
    ///
    /// let observation = Observation::builder().stderr("All done").build();
    ///
    /// assert_eq!(observation.stderr(), "All done");
    /// ```
    #[builder]
    pub fn new(
        #[builder(default)] problems: Vec<OxfmtProblem>,
        rejected_configuration: Option<String>,
        #[builder(into, default)] stderr: String,
        #[builder(default)] exit: Exit,
        #[builder(default)] pattern: Pattern,
    ) -> Self {
        Self {
            problems,
            rejected_configuration,
            stderr,
            exit,
            pattern,
        }
    }

    /// Reads what a run of oxfmt produced
    ///
    /// The reading takes both streams of the run and its exit status.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_format_typescript::Observation;
    /// use rakko_tool::Invocation;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let execution = Invocation::new("oxfmt").arg("--list-different").run().await?;
    /// let observation = Observation::read(&execution);
    ///
    /// println!("Exit: {:?}", observation.exit());
    /// # Ok(())
    /// # }
    /// ```
    pub fn read(execution: &Execution) -> Self {
        let exit = if execution.status().success() {
            Exit::Succeeded
        } else {
            Exit::Failed
        };
        self::report::read()
            .stdout(&execution.stdout().to_string_lossy())
            .stderr(&execution.stderr().to_string_lossy())
            .exit(exit)
            .call()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Exit>();
    }

    #[test]
    fn observation_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Observation>();
    }

    #[test]
    fn pattern_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Pattern>();
    }
}
