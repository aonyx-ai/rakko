//! The markdownlint that a project runs
//!
//! This module holds the version of markdownlint that the crate declares and
//! the program that mise installed at that version. It also holds the run that
//! produces a report. An action asks for a run, and everything between the
//! action and the process lives here.

mod error;
mod reference;
mod report;

use aonyx_rakko_node::{Node, ResolveNodeToolError};
use rakko::{Action, Context, ProjectRoot, ToolOutput};
use rakko_tool::{Launch, ShownStreams, Tool};

pub(crate) use self::error::ObserveMarkdownlintError;
use self::reference::Reference;
use crate::observation::{Examination, Exit, Observation};

/// The name that mise knows the tool by
const MARKDOWNLINT_CLI: &str = "npm:markdownlint-cli";

/// The program of the tool
const MARKDOWNLINT: &str = "markdownlint";

/// The flag that asks markdownlint for its report as data
///
/// Markdownlint writes one line per result for a reader by default. On request,
/// JSON carries the rule, the position, and the message in fields instead of in
/// a line that a reader has to take apart. The flag selects the presentation of
/// the report, not the behavior of the tool. Which rules apply to which file
/// comes from the configuration of the project alone.
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
/// The repository directory starts with a dot. So a run that asks for the
/// entries with a dot reaches it, and it holds no project content. The pattern
/// matches at every depth, because a repository that a project holds in its
/// tree has a `.git` directory of its own. The argument reaches markdownlint
/// without a shell, so nothing expands the wildcards on the way.
const GIT: &str = "**/.git/**";

/// The flag that gives markdownlint a configuration file to read
///
/// Markdownlint reports a configuration that it cannot parse only for the file
/// that this flag gives.
const CONFIG: &str = "--config";

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
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it. So a run reaches
/// the version that the tests of the crate cover. The program runs on the Node
/// that the project pins. Nothing here installs a tool: provisioning is the job
/// of mise, and a markdownlint that mise does not report stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Markdownlint {
    /// The program that mise installed at the declared version
    tool: Tool,

    /// The tool output of the context that resolved markdownlint
    ///
    /// Each run sends what markdownlint writes to it.
    output: ToolOutput,
}

#[bon::bon]
impl Markdownlint {
    /// Runs markdownlint over the project and reads what it reported
    ///
    /// The run names the root of the project and asks for the report as data.
    /// It includes files and directories whose name starts with a dot, which
    /// markdownlint skips by default. It leaves out the `.git` directory. The
    /// ignore file of the project decides what else the run leaves out.
    ///
    /// Markdownlint finds the configuration of the project on its own. It
    /// treats a configuration file that it cannot parse as a file that is not
    /// there. So the run also refers markdownlint to the first configuration
    /// file in the root, through a temporary file outside the project.
    /// Markdownlint then reports that file when it cannot parse it. It applies
    /// the same configuration as without the reference.
    ///
    /// Markdownlint writes the report on its standard error stream. A run that
    /// finds no file to examine writes no report and ends with success, like a
    /// run that found nothing wrong. So the observation also says whether the
    /// run examined a file.
    ///
    /// # Errors
    ///
    /// Returns [`UnwritableReference`][unwritable] when the temporary file
    /// cannot be written, and [`MarkdownlintUnavailable`][unavailable] when
    /// markdownlint does not run. Returns
    /// [`RejectedConfiguration`][rejected] when markdownlint cannot parse the
    /// configuration file of the project. Returns
    /// [`UnreadableReport`][unreadable] when markdownlint wrote something else
    /// than the report that the crate reads.
    ///
    /// [rejected]: ObserveMarkdownlintError::RejectedConfiguration
    /// [unavailable]: ObserveMarkdownlintError::MarkdownlintUnavailable
    /// [unreadable]: ObserveMarkdownlintError::UnreadableReport
    /// [unwritable]: ObserveMarkdownlintError::UnwritableReference
    pub(crate) async fn observe(
        &self,
        root: &ProjectRoot,
    ) -> Result<Observation, ObserveMarkdownlintError> {
        let reference = Reference::to_configuration_of(root.get())
            .map_err(|source| ObserveMarkdownlintError::UnwritableReference { source })?;

        let invocation = self
            .tool
            .invocation()
            .arg(JSON)
            .arg(DOT)
            .arg(IGNORE)
            .arg(GIT);
        let invocation = match &reference {
            Some(reference) => invocation.arg(CONFIG).arg(reference.path()),
            None => invocation,
        };

        let execution = invocation
            .arg(HERE)
            .launch_with(&self.output, ShownStreams::StandardOutput)
            .await
            .map_err(|source| ObserveMarkdownlintError::MarkdownlintUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy();
        let stderr = execution.stderr().to_string_lossy();

        if let Some(reference) = &reference
            && let Some(details) = reference.rejection(&stderr)
        {
            return Err(ObserveMarkdownlintError::RejectedConfiguration {
                path: reference.configuration().to_path_buf(),
                details,
            });
        }

        let examined = examined().stdout(&stdout).stderr(&stderr).call();

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

        let examination = if examined {
            Examination::Examined
        } else {
            Examination::Skipped
        };

        let exit = if execution.status().success() {
            Exit::Succeeded
        } else {
            Exit::Failed
        };

        Ok(Observation::builder()
            .problems(problems)
            .examined(examination)
            .stderr(stderr.into_owned())
            .succeeded(exit)
            .build())
    }

    /// Returns the markdownlint that mise installed at the version that the
    /// action declares, on the Node that the project pins
    ///
    /// The lookup reads the tools that the action declares, which hold the
    /// pin in the mise file of the crate. So a pin of the tool in the project
    /// does not change the answer. It also asks mise for the first Node that
    /// the project pins. A run starts in the root of the context, with that
    /// Node first on its path.
    ///
    /// # Errors
    ///
    /// Returns [`UnresolvedTool`][tool] when the action does not declare the
    /// tool, or when mise reports no markdownlint at that version, for example
    /// because nothing installed it. Returns [`UnresolvedNode`][node]
    /// when the project pins no Node, or when mise did not install that Node.
    /// It also returns that error when the directory of that Node cannot be an
    /// entry of `PATH`. Both errors also occur when mise does not run.
    ///
    /// [node]: ResolveNodeToolError::UnresolvedNode
    /// [tool]: ResolveNodeToolError::UnresolvedTool
    #[builder]
    pub(crate) async fn resolve<A: Action>(
        action: &A,
        context: &Context,
    ) -> Result<Self, ResolveNodeToolError> {
        let tool = Node::resolve_tool()
            .action(action)
            .tool(MARKDOWNLINT_CLI)
            .executable(MARKDOWNLINT)
            .context(context)
            .call()
            .await?;

        Ok(Self {
            tool,
            output: context.tool_output().clone(),
        })
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
#[bon::builder]
fn examined(stdout: &str, stderr: &str) -> bool {
    stdout.trim().is_empty() && stderr.trim() != NO_MATCHING_FILES
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "An assertion in a test panics by design, and a test that resolves a tool expects mise to find it. A `# Panics` section on every test would repeat that and give the reader no information."
    )]
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use super::*;
    use crate::LintMarkdown;

    /// The start of the usage text that an older markdownlint writes
    const USAGE: &str = "Usage: markdownlint [options] [files|directories|globs...]\n";

    /// A report of a run that found a problem
    const REPORT: &str = r#"[{"fileName":"notes.md","lineNumber":1}]"#;

    /// Returns the context of a run in the root of this repository
    ///
    /// The repository pins a Node, and it installs the markdownlint that the
    /// crate declares. The crate lives two directories below the root.
    fn context() -> Context {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository");

        Context::builder().root(root).build()
    }

    #[test]
    fn examined_of_a_run_with_a_report_is_true() {
        let examined = examined().stdout("").stderr(REPORT).call();

        assert!(examined);
    }

    #[test]
    fn examined_of_a_run_with_no_matching_files_is_false() {
        let examined = examined()
            .stdout("")
            .stderr("No matching files found\n")
            .call();

        assert!(!examined);
    }

    #[test]
    fn examined_of_a_run_with_the_usage_text_is_false() {
        let examined = examined().stdout(USAGE).stderr("").call();

        assert!(!examined);
    }

    #[test]
    fn examined_of_a_run_without_output_is_true() {
        let examined = examined().stdout("").stderr("").call();

        assert!(examined);
    }

    #[test]
    fn markdownlint_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Markdownlint>();
    }

    #[tokio::test]
    async fn resolve_runs_markdownlint_on_the_node_of_the_project() {
        let context = context();
        let node = Node::resolve()
            .action(&LintMarkdown)
            .context(&context)
            .call()
            .await
            .expect("the test resolves the Node that the repository pins");

        let markdownlint = Markdownlint::resolve()
            .action(&LintMarkdown)
            .context(&context)
            .call()
            .await
            .expect("the test resolves the markdownlint that the crate declares");

        let invocation = markdownlint.tool.invocation();
        let path = invocation
            .environment()
            .iter()
            .find(|variable| variable.name().get() == "PATH")
            .map(|variable| variable.value().get().to_owned())
            .expect("expected the command of markdownlint to set the path");
        assert_eq!(
            std::env::split_paths(&path).next().as_deref(),
            node.program().get().parent()
        );
    }
}
