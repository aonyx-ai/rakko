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
    /// Markdownlint writes the report on its standard error stream, and it
    /// leaves the standard output stream empty for as long as it has a file
    /// to examine, so a run that wrote there examined nothing and answered
    /// with its usage text instead.
    ///
    /// # Errors
    ///
    /// Returns [`MarkdownlintUnavailable`][unavailable] when markdownlint
    /// does not run, and [`UnreadableReport`][unreadable] when it wrote
    /// something else than the report that the crate reads.
    ///
    /// [unavailable]: ObserveMarkdownlintError::MarkdownlintUnavailable
    /// [unreadable]: ObserveMarkdownlintError::UnreadableReport
    // lintmarkdown[impl check.read]
    // lintmarkdown[impl run.dot]
    // lintmarkdown[impl run.git]
    // lintmarkdown[impl run.ignored]
    // lintmarkdown[impl run.project]
    // lintmarkdown[impl run.structured+2]
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

        // lintmarkdown[impl check.unreadable]
        let problems = self::report::problems(&stderr).map_err(|source| {
            ObserveMarkdownlintError::UnreadableReport {
                report: stderr.to_string(),
                source,
            }
        })?;

        Ok(Observation::builder()
            .problems(problems)
            // lintmarkdown[impl skip.unexamined]
            .examined(stdout.trim().is_empty())
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
    // lintmarkdown[impl tool.markdownlint]
    // lintmarkdown[impl tool.missing]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(MARKDOWNLINT), root).await?;

        Ok(Self { tool })
    }
}
