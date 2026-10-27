//! The markdownlint that a project runs
//!
//! This module holds the program that mise installed for a project and the
//! run that produces a report. An action asks for a run, and everything
//! between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that markdownlint wrote
mod report;

use rakko_action::ProjectRoot;
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::ObserveMarkdownlintError;
use crate::observation::Observation;

/// The name that mise knows the tool by
const MARKDOWNLINT: &str = "markdownlint";

/// The flag that asks markdownlint for its report as data
///
/// Markdownlint writes one line per result for a reader by default, and the
/// same results as JSON on request, which carries the rule, the position, and
/// the message in fields instead of in a line that a reader has to take
/// apart. The flag selects the presentation of the report and not the
/// behavior of the tool: which rules apply to which file comes from the
/// configuration of the project alone.
const JSON: &str = "--json";

/// The flag that asks markdownlint to examine the entries with a dot
///
/// Markdownlint skips every file and directory whose name starts with a dot
/// by default. Projects keep Markdown in such directories: GitHub reads issue
/// templates from `.github`, and a changeset tool writes its changesets to
/// `.changeset`. Markdownlint accepts this request on its command line only,
/// and its configuration has no key for it, so a project cannot make the
/// request itself. The ignore file of the project still excludes what it
/// names, in a directory with a dot as well.
const DOT: &str = "--dot";

/// The flag that leaves the files of a pattern out of a run
///
/// Markdownlint applies the flag next to the ignore file of the project, and
/// a file that either one names stays out of the run.
const IGNORE: &str = "--ignore";

/// The pattern of the directories in which Git keeps a repository
///
/// The directory of a repository starts with a dot, so a run that asks for
/// the entries with a dot reaches it, and it holds no content of the project.
/// The pattern matches at every depth, because a repository that a project
/// holds in its tree has a `.git` directory of its own. The argument reaches
/// markdownlint without a shell, so nothing expands the wildcards on the way.
const GIT: &str = "**/.git/**";

/// The place that a run tells markdownlint to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and markdownlint reports every path relative to it.
const HERE: &str = ".";

/// The sentence that markdownlint writes when it finds no file to examine
///
/// Markdownlint writes this sentence on its standard error stream, in place
/// of the report, when the places that it received hold no Markdown file. The
/// run then ends with success.
const NO_MATCHING_FILES: &str = "No matching files found";

/// The markdownlint that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a markdownlint that mise does not
/// report stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_lint_markdown::Markdownlint;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// let markdownlint = Markdownlint::resolve(root).await?;
/// let observation = markdownlint.observe().await?;
///
/// if observation.examined() {
///     println!("{} problems", observation.problems().len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Markdownlint {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Markdownlint {
    /// Runs markdownlint over the project and reads what it reported
    ///
    /// The run names the root of the project and asks for the report as data.
    /// It includes the files and directories whose name starts with a dot,
    /// which markdownlint skips by default, and it leaves out every directory
    /// with the name `.git`. The ignore file of the project decides what else
    /// the run leaves out.
    ///
    /// Markdownlint writes the report on its standard error stream. A run
    /// that finds no file to examine writes no report and ends with success,
    /// like a run that found nothing wrong, so the observation also says
    /// whether the run examined a file.
    ///
    /// # Errors
    ///
    /// Returns [`MarkdownlintUnavailable`][unavailable] when markdownlint
    /// does not run, and [`UnreadableReport`][unreadable] when it wrote
    /// something else than the report that the crate reads.
    ///
    /// [unavailable]: ObserveMarkdownlintError::MarkdownlintUnavailable
    /// [unreadable]: ObserveMarkdownlintError::UnreadableReport
    pub async fn observe(&self) -> Result<Observation, ObserveMarkdownlintError> {
        let execution = self
            .tool
            .invocation()
            .arg(JSON)
            .arg(DOT)
            .arg(IGNORE)
            .arg(GIT)
            .arg(HERE)
            .run()
            .await
            .map_err(|source| ObserveMarkdownlintError::MarkdownlintUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy();
        let stderr = execution.stderr().to_string_lossy();

        let examined = examined(&stdout, &stderr);

        let problems = if examined {
            self::report::problems(&stderr).map_err(|source| {
                ObserveMarkdownlintError::UnreadableReport {
                    report: stderr.to_string(),
                    source,
                }
            })?
        } else {
            Vec::new()
        };

        Ok(Observation::builder()
            .problems(problems)
            .examined(examined)
            .stderr(stderr.into_owned())
            .succeeded(execution.status().success())
            .build())
    }

    /// Returns the markdownlint that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no markdownlint for
    /// the project.
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(MARKDOWNLINT), root).await?;

        Ok(Self { tool })
    }
}

/// Returns whether a run of markdownlint examined a file
///
/// Markdownlint answers a run that finds no file to examine in one of two
/// ways, and the version of markdownlint selects the way. An older version
/// writes its usage text on the standard output stream, which a run that
/// asks for the report as data leaves empty otherwise. A newer version writes
/// a single sentence on the standard error stream in place of the report.
/// Both versions then end with success, like a run that examined the project
/// and found nothing.
fn examined(stdout: &str, stderr: &str) -> bool {
    stdout.trim().is_empty() && stderr.trim() != NO_MATCHING_FILES
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// The start of the usage text that an older markdownlint writes
    const USAGE: &str = "Usage: markdownlint [options] [files|directories|globs...]\n";

    /// A report of a run that found a problem
    const REPORT: &str = r#"[{"fileName":"notes.md","lineNumber":1}]"#;

    #[test]
    fn examined_of_a_run_with_a_report_is_true() {
        let examined = examined("", REPORT);

        assert!(examined);
    }

    #[test]
    fn examined_of_a_run_with_no_matching_files_is_false() {
        let examined = examined("", "No matching files found\n");

        assert!(!examined);
    }

    #[test]
    fn examined_of_a_run_with_the_usage_text_is_false() {
        let examined = examined(USAGE, "");

        assert!(!examined);
    }

    #[test]
    fn examined_of_a_run_without_output_is_true() {
        let examined = examined("", "");

        assert!(examined);
    }
}
