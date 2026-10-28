//! The taplo that a project runs
//!
//! This module holds the program that mise installed for a project, the run
//! of one operation, and the files that such a run reads and writes. An
//! action states which operation it wants, and everything between the action
//! and the process lives here.

/// The error that leaves a run without an answer
mod error;

use std::iter;
use std::time::Duration;

use rakko_action::{Access, FileAccess, Files, Pattern, ProjectRoot};
use rakko_tool::{Execution, ResolveToolError, Tool, ToolName};

pub use self::error::ObserveTaploError;
use crate::observation::Observation;
use crate::operation::Operation;

/// The number of times one operation starts before the crate gives up
///
/// Taplo can lose the end of its report when it exits, and a fresh run
/// almost always answers, so a few attempts separate a lost line from a
/// report that has genuinely ended.
const ATTEMPTS: u32 = 6;

/// The number of times one operation starts to get the count of the files
///
/// The count is the one part of a report that no other stream carries, and
/// a run that has nothing left to do after it counts loses it most often. A
/// caller reports a pass without the count, so the count gets a small budget
/// of its own instead of the whole one.
const COUNTED_ATTEMPTS: u32 = 2;

/// The pause that grows between the attempts of one operation
///
/// The loss happens while taplo exits, and a machine under load takes longer
/// over it, so attempts that follow each other immediately tend to lose
/// together. A growing pause carries the later attempts past the moment of
/// load.
const BACKOFF: Duration = Duration::from_millis(25);

/// The name that mise knows the tool by
const TAPLO: &str = "taplo";

/// The pattern of the files that taplo selects
///
/// Taplo selects these files when the configuration of the project names no
/// selection of its own. The pattern matches a file in a hidden directory as
/// well, as taplo does.
const SELECTION: &str = "**/*.toml";

/// The patterns of the files that taplo reads as its configuration
///
/// Taplo looks for one of these files in the root of the project, where every
/// run starts.
const CONFIGURATION: [&str; 2] = [".taplo.toml", "taplo.toml"];

/// The subcommand of taplo that formats TOML files
const FORMAT: &str = "fmt";

/// The subcommand of taplo that validates TOML files
const LINT: &str = "lint";

/// The flag that asks the formatter to report instead of rewriting
const CHECK: &str = "--check";

/// The flag that asks the formatter to name its files on standard output
///
/// The flag makes taplo print the difference that it would write for each
/// file that is not formatted. The crate reads the header of each of those
/// differences and not the text below it, because the header names the file
/// and the stream that carries it does not lose lines.
const DIFF: &str = "--diff";

/// The flag that selects how taplo colors its report
const COLORS: &str = "--colors";

/// The value that asks taplo for a report without color codes
///
/// The report is read as data, and a color code inside a path would corrupt
/// the reading. The flag selects the presentation of the report and not the
/// behavior of the tool: what taplo does to a project comes from the
/// configuration of that project alone.
const PLAIN: &str = "never";

/// The taplo that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a taplo that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_taplo::{Operation, Taplo};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let taplo = Taplo::resolve(ProjectRoot::new("/home/otter/project".into())).await?;
///
/// let observation = taplo.observe(Operation::CheckFormat).await?;
///
/// println!("{:?} files", observation.checked());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Taplo {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Taplo {
    /// Returns the files that a run of taplo reads and writes
    ///
    /// The answer is the declaration of an action that runs taplo with the
    /// given access. Every run reads the configuration of taplo and the files
    /// that mise reads to find taplo. A run that reads also reads the TOML
    /// files of the project, and it writes no file. A run that writes
    /// rewrites the TOML files, so it writes them and reads only the
    /// configuration.
    ///
    /// The patterns cover the files that taplo selects when the configuration
    /// of the project names no selection. A configuration that widens the
    /// selection, such as an `include` that adds `Pipfile`, is outside the
    /// patterns.
    ///
    /// The function does no I/O, so a declaration can call it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_action::{Access, Files, Pattern};
    /// use rakko_taplo::Taplo;
    ///
    /// let files = Taplo::files(Access::Write);
    ///
    /// assert_eq!(
    ///     files.writes(),
    ///     &Files::Matching(vec![Pattern::from_static("**/*.toml")])
    /// );
    /// ```
    pub fn files(access: Access) -> FileAccess {
        let selection = Pattern::from_static(SELECTION);
        let configuration = CONFIGURATION
            .into_iter()
            .map(Pattern::from_static)
            .chain(Tool::resolution_files());

        match access {
            Access::Read => FileAccess::builder()
                .reads(Files::Matching(
                    iter::once(selection).chain(configuration).collect(),
                ))
                .writes(Files::Matching(Vec::new()))
                .build(),
            Access::Write => FileAccess::builder()
                .reads(Files::Matching(configuration.collect()))
                .writes(Files::Matching(vec![selection]))
                .build(),
        }
    }

    /// Runs one operation and reads what taplo reported
    ///
    /// Taplo can lose the end of its report when it exits. The answer of a
    /// run survives that loss: the exit status carries whether taplo found
    /// anything, a formatting run names the files that it would rewrite on
    /// its standard output stream, a file that taplo read and could not
    /// accept gets a diagnostic that the same loss never reaches, and the
    /// reading of a failed validating run reads the files of the run itself
    /// to find the ones that taplo could not read. A report that holds no
    /// answer at all lost the lines that carried one, and such a run starts
    /// again, a few times, before the crate gives up.
    ///
    /// The count of the files is the one part that no other stream carries.
    /// An operation starts again to get it, with a budget of its own, and
    /// answers without the count instead of failing over it: a run that
    /// ended with success found nothing, whatever its report lost.
    ///
    /// Repeating is safe for every operation, because a report reads the
    /// project again, and a rewrite formats files that a previous attempt
    /// already formatted.
    ///
    /// # Errors
    ///
    /// Returns [`TaploUnavailable`][unavailable] when taplo does not run,
    /// and [`IncompleteReport`][incomplete] when no attempt answered.
    ///
    /// [incomplete]: ObserveTaploError::IncompleteReport
    /// [unavailable]: ObserveTaploError::TaploUnavailable
    pub async fn observe(&self, operation: Operation) -> Result<Observation, ObserveTaploError> {
        let mut stderr = String::new();
        let mut answered: Option<Observation> = None;

        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(BACKOFF * attempt).await;
            }

            let observation = Observation::read(&self.start(operation).await?, operation).await;

            if !observation.complete() {
                stderr = observation.stderr().clone();
                continue;
            }

            if observation.checked().is_some() || attempt + 1 >= COUNTED_ATTEMPTS {
                return Ok(observation);
            }

            answered.get_or_insert(observation);
        }

        answered.ok_or(ObserveTaploError::IncompleteReport { stderr })
    }

    /// Returns the taplo that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names,
    /// so the version that the project pinned answers, whatever the shell
    /// that started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no taplo for the
    /// project.
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(TAPLO), root).await?;

        Ok(Self { tool })
    }

    /// Starts one operation and collects what it produced
    ///
    /// # Errors
    ///
    /// Returns [`TaploUnavailable`][unavailable] when taplo does not run.
    ///
    /// [unavailable]: ObserveTaploError::TaploUnavailable
    async fn start(&self, operation: Operation) -> Result<Execution, ObserveTaploError> {
        self.tool
            .invocation()
            .args(arguments(operation).iter().copied())
            .arg(COLORS)
            .arg(PLAIN)
            .run()
            .await
            .map_err(|source| ObserveTaploError::TaploUnavailable { source })
    }
}

/// Returns the command line of one operation of taplo
fn arguments(operation: Operation) -> &'static [&'static str] {
    match operation {
        Operation::CheckFormat => &[FORMAT, CHECK, DIFF],
        Operation::Format => &[FORMAT],
        Operation::Lint => &[LINT],
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns the patterns that the literals describe
    fn patterns<const N: usize>(literals: [&'static str; N]) -> Vec<Pattern> {
        literals.into_iter().map(Pattern::from_static).collect()
    }

    /// Returns the patterns of the configuration that every run reads
    fn configuration() -> Vec<Pattern> {
        let mut configuration = patterns(CONFIGURATION);
        configuration.extend(Tool::resolution_files());

        configuration
    }

    #[test]
    fn arguments_of_a_format_check_report_instead_of_rewriting() {
        let selected = arguments(Operation::CheckFormat);

        assert_eq!(selected, ["fmt", "--check", "--diff"]);
    }

    #[test]
    fn arguments_of_a_format_rewrite_the_project() {
        let selected = arguments(Operation::Format);

        assert_eq!(selected, ["fmt"]);
    }

    #[test]
    fn arguments_of_a_lint_validate_the_project() {
        let selected = arguments(Operation::Lint);

        assert_eq!(selected, ["lint"]);
    }

    #[test]
    fn files_of_a_read_cover_the_toml_files_and_the_configuration() {
        let mut expected = patterns(["**/*.toml"]);
        expected.extend(configuration());

        let files = Taplo::files(Access::Read);

        assert_eq!(files.reads(), &Files::Matching(expected));
    }

    #[test]
    fn files_of_a_read_write_no_file() {
        let files = Taplo::files(Access::Read);

        assert_eq!(files.writes(), &Files::Matching(Vec::new()));
    }

    #[test]
    fn files_of_a_write_read_the_configuration() {
        let files = Taplo::files(Access::Write);

        assert_eq!(files.reads(), &Files::Matching(configuration()));
    }

    #[test]
    fn files_of_a_write_write_the_toml_files() {
        let files = Taplo::files(Access::Write);

        assert_eq!(files.writes(), &Files::Matching(patterns(["**/*.toml"])));
    }
}
