//! The taplo that a project runs
//!
//! This module holds the program that mise installed for a project, the look
//! that tells whether taplo has anything to do there, and the run of one
//! operation. An action states which operation it wants, and everything
//! between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;

use std::ffi::OsStr;
use std::time::Duration;

use rakko_action::ProjectRoot;
use rakko_tool::{Execution, ResolveToolError, Tool, ToolName};

pub use self::error::ObserveTaploError;
use crate::observation::Observation;
use crate::operation::Operation;

/// The number of times one operation starts before the crate gives up
///
/// Taplo can lose the tail of its report when it exits, and a fresh run
/// almost always answers completely, so a few attempts separate a lost tail
/// from a report that has genuinely ended.
const ATTEMPTS: u32 = 6;

/// The pause that grows between the attempts of one operation
///
/// The loss correlates with the load of the machine, so attempts that follow
/// each other immediately tend to lose together. A growing pause carries the
/// later attempts past the moment of load.
const BACKOFF: Duration = Duration::from_millis(25);

/// The name that mise knows the tool by
const TAPLO: &str = "taplo";

/// The subcommand of taplo that formats TOML files
const FORMAT: &str = "fmt";

/// The subcommand of taplo that validates TOML files
const LINT: &str = "lint";

/// The flag that asks the formatter to report instead of rewriting
const CHECK: &str = "--check";

/// The flag that selects how taplo colors its report
const COLORS: &str = "--colors";

/// The value that asks taplo for a report without color codes
///
/// The report is read as data, and a color code inside a path would corrupt
/// the reading. The flag selects the presentation of the report and not the
/// behavior of the tool: what taplo does to a project comes from the
/// configuration of that project alone.
const PLAIN: &str = "never";

/// The extension of the files that taplo works on
const TOML_EXTENSION: &str = "toml";

/// The directory entry that the look does not read
const GIT_DIRECTORY: &str = ".git";

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
    /// Returns whether the project holds a file that taplo would look at
    ///
    /// The look walks the project from its root and stops at the first file
    /// with the `.toml` extension, in the case that taplo matches. It reads
    /// hidden directories, because taplo reads them, and it does not read
    /// the `.git` entry, which holds no file of the project. It follows no
    /// symbolic link, so a cycle of links cannot trap it.
    ///
    /// A directory that the look cannot read counts as holding TOML files. A
    /// look that cannot prove absence must not hide a real check behind a
    /// skip, and taplo reports its own failure when a run reaches it.
    ///
    /// The look and taplo can still disagree at the margins, because the
    /// configuration of a project can exclude every file that the look
    /// found. A caller that reaches taplo therefore reports what taplo saw.
    // taplo[impl look.git]
    // taplo[impl look.links]
    // taplo[impl look.toml]
    // taplo[impl look.unreadable]
    pub async fn applies(root: &ProjectRoot) -> bool {
        let mut pending = vec![root.get().to_path_buf()];

        while let Some(directory) = pending.pop() {
            let Ok(mut entries) = tokio::fs::read_dir(&directory).await else {
                return true;
            };

            loop {
                match entries.next_entry().await {
                    Ok(Some(entry)) => {
                        if entry.file_name() == GIT_DIRECTORY {
                            continue;
                        }

                        let Ok(kind) = entry.file_type().await else {
                            return true;
                        };

                        if kind.is_dir() {
                            pending.push(entry.path());
                        } else if kind.is_file()
                            && entry.path().extension() == Some(OsStr::new(TOML_EXTENSION))
                        {
                            return true;
                        }
                    }
                    Ok(None) => break,
                    Err(_) => return true,
                }
            }
        }

        false
    }

    /// Runs one operation and reads what taplo reported
    ///
    /// Taplo can lose the tail of its report when it exits, and a lost tail
    /// can hide problems that taplo found. A run whose report is incomplete
    /// therefore starts again, a few times, before the crate gives up.
    /// Repeating is safe for every operation: a report reads the project
    /// again, and a rewrite formats files that a previous attempt already
    /// formatted.
    ///
    /// # Errors
    ///
    /// Returns [`TaploUnavailable`][unavailable] when taplo does not run,
    /// and [`IncompleteReport`][incomplete] when every attempt lost part of
    /// its report.
    ///
    /// [incomplete]: ObserveTaploError::IncompleteReport
    /// [unavailable]: ObserveTaploError::TaploUnavailable
    // taplo[impl run.complete]
    pub async fn observe(&self, operation: Operation) -> Result<Observation, ObserveTaploError> {
        let mut stderr = String::new();

        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(BACKOFF * attempt).await;
            }

            let observation = Observation::read(&self.start(operation).await?);

            if observation.complete() {
                return Ok(observation);
            }

            stderr = observation.stderr().clone();
        }

        Err(ObserveTaploError::IncompleteReport { stderr })
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
    // taplo[impl tool.missing]
    // taplo[impl tool.resolve]
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
    // taplo[impl run.operation]
    // taplo[impl run.plain]
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
// taplo[impl run.operation]
fn arguments(operation: Operation) -> &'static [&'static str] {
    match operation {
        Operation::CheckFormat => &[FORMAT, CHECK],
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

    // taplo[verify run.operation]
    #[test]
    fn arguments_of_a_format_check_report_instead_of_rewriting() {
        let selected = arguments(Operation::CheckFormat);

        assert_eq!(selected, ["fmt", "--check"]);
    }

    // taplo[verify run.operation]
    #[test]
    fn arguments_of_a_format_rewrite_the_project() {
        let selected = arguments(Operation::Format);

        assert_eq!(selected, ["fmt"]);
    }

    // taplo[verify run.operation]
    #[test]
    fn arguments_of_a_lint_validate_the_project() {
        let selected = arguments(Operation::Lint);

        assert_eq!(selected, ["lint"]);
    }
}
