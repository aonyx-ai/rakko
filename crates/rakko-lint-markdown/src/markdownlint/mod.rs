//! The markdownlint that a project runs
//!
//! This module holds the program that mise installed for a project, the look
//! that tells whether markdownlint has anything to do there, and the run that
//! produces a report. An action asks for a run, and everything between the
//! action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that markdownlint wrote
mod report;

use std::ffi::OsStr;
use std::path::Path;

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

/// The place that a run tells markdownlint to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and markdownlint reports every path relative to it.
const HERE: &str = ".";

/// The character that starts the name of an entry that markdownlint skips
const HIDDEN: char = '.';

/// The extensions of the files that markdownlint collects below a directory
const MARKDOWN_EXTENSIONS: [&str; 2] = ["md", "markdown"];

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
/// if Markdownlint::applies(&root).await {
///     let markdownlint = Markdownlint::resolve(root).await?;
///     let observation = markdownlint.observe().await?;
///
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
    /// Returns whether the project holds a file that markdownlint would look
    /// at
    ///
    /// The look walks the project from its root and stops at the first file
    /// with the `.md` or the `.markdown` extension, which are the two that
    /// markdownlint collects below a directory. It reads no entry whose name
    /// starts with a dot, because markdownlint reads none either, and that
    /// covers the `.git` entry, which holds no file of the project. It
    /// follows no symbolic link, so a cycle of links cannot trap it.
    ///
    /// A directory that the look cannot read counts as holding Markdown
    /// files. A look that cannot prove absence must not hide a real check
    /// behind a skip, and markdownlint reports its own failure when a run
    /// reaches it.
    ///
    /// The look and markdownlint can still disagree at the margins, because
    /// markdownlint follows a link that the look leaves alone, and because
    /// the ignore file of a project can exclude every file that the look
    /// found. A caller that reaches markdownlint therefore reports what
    /// markdownlint saw.
    // lintmarkdown[impl skip.hidden]
    // lintmarkdown[impl skip.links]
    // lintmarkdown[impl skip.missing]
    pub async fn applies(root: &ProjectRoot) -> bool {
        let mut pending = vec![root.get().to_path_buf()];

        while let Some(directory) = pending.pop() {
            let Ok(mut entries) = tokio::fs::read_dir(&directory).await else {
                return true;
            };

            loop {
                match entries.next_entry().await {
                    Ok(Some(entry)) => {
                        if entry.file_name().to_string_lossy().starts_with(HIDDEN) {
                            continue;
                        }

                        let Ok(kind) = entry.file_type().await else {
                            return true;
                        };

                        if kind.is_dir() {
                            pending.push(entry.path());
                        } else if kind.is_file() && markdown(&entry.path()) {
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

    /// Runs markdownlint over the project and reads what it reported
    ///
    /// The run names the root of the project and asks for the report as data.
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
    // lintmarkdown[impl run.project]
    // lintmarkdown[impl run.structured]
    pub async fn observe(&self) -> Result<Observation, ObserveMarkdownlintError> {
        let execution = self
            .tool
            .invocation()
            .arg(JSON)
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

/// Returns whether markdownlint would collect the file below a directory
///
/// The comparison ignores the case of the extension, which is what
/// markdownlint does on macOS and on Windows, and is the wider answer on the
/// platforms where markdownlint matches the case. A look that answers for the
/// wider set never hides a check behind a skip: a project whose only Markdown
/// file is written `NOTES.MD` on such a platform reaches markdownlint, which
/// then reports that it found nothing to examine.
// lintmarkdown[impl skip.missing]
fn markdown(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };

    MARKDOWN_EXTENSIONS
        .iter()
        .any(|candidate| extension.eq_ignore_ascii_case(candidate))
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use std::path::PathBuf;

    use super::*;

    // lintmarkdown[verify skip.missing]
    #[test]
    fn markdown_of_a_file_of_another_extension_reports_no_match() {
        let matched = markdown(&PathBuf::from("sub/notes.txt"));

        assert!(!matched);
    }

    // lintmarkdown[verify skip.missing]
    #[test]
    fn markdown_of_a_long_extension_reports_a_match() {
        let matched = markdown(&PathBuf::from("sub/notes.markdown"));

        assert!(matched);
    }

    // lintmarkdown[verify skip.missing]
    #[test]
    fn markdown_of_a_short_extension_reports_a_match() {
        let matched = markdown(&PathBuf::from("sub/notes.md"));

        assert!(matched);
    }

    // lintmarkdown[verify skip.missing]
    #[test]
    fn markdown_of_a_file_without_an_extension_reports_no_match() {
        let matched = markdown(&PathBuf::from("justfile"));

        assert!(!matched);
    }
}
