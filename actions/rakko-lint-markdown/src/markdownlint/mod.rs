//! The markdownlint that a project runs
//!
//! This module holds the version of markdownlint that the crate declares, the
//! program that mise installed at that version, and the run that produces a
//! report. An action asks for a run, and everything between the action and
//! the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that markdownlint wrote
mod report;

use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion, VersionRange};
use rakko_tool::{Launch, ResolveRequirementError, Tool, pinned_version};

pub(crate) use self::error::ObserveMarkdownlintError;
use crate::observation::Observation;

/// The name that mise knows the tool by
const MARKDOWNLINT_CLI: &str = "npm:markdownlint-cli";

/// The program of the tool
const MARKDOWNLINT: &str = "markdownlint";

/// The version of markdownlint-cli that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = pinned_version(include_str!("../../mise.rakko.toml"), MARKDOWNLINT_CLI);

/// The name that mise knows Node by, which is also the name of its program
const NODE: &str = "node";

/// The versions of Node that markdownlint runs on
///
/// Node is a toolchain of the project, so the project pins its version, and
/// a run stops when that version is outside the range.
const NODE_RANGE: &str = ">=22";

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
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it, so a run reaches
/// the version that the tests of the crate cover. The program runs on the Node
/// that the project pins, when the range of the crate accepts it. Nothing here
/// installs a tool: provisioning is the job of mise, and a markdownlint that
/// mise does not report stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Markdownlint {
    /// The program that mise installed at the declared version
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
    pub(crate) async fn observe(&self) -> Result<Observation, ObserveMarkdownlintError> {
        let execution = self
            .tool
            .invocation()
            .arg(JSON)
            .arg(DOT)
            .arg(IGNORE)
            .arg(GIT)
            .arg(HERE)
            .launch()
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

    /// Returns the requirement of the Node that markdownlint runs on
    ///
    /// Node is a toolchain, so the requirement holds the range of versions
    /// that markdownlint accepts, and the project pins the version.
    pub(crate) fn node_requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(NODE)
            .executable(NODE)
            .version(RequiredVersion::Range(VersionRange::new(NODE_RANGE)))
            .build()
    }

    /// Returns the requirement of the markdownlint that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(MARKDOWNLINT_CLI)
            .executable(MARKDOWNLINT)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the markdownlint that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of the tool in the project does not change the answer.
    /// It also asks mise for the Node that the project pins and that
    /// [`node_requirement`][node] accepts. A run starts in the root of the
    /// context, with that Node first on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, when the
    /// project pins no Node in the range, when mise reports no markdownlint at the
    /// declared version or no Node at the pinned one, for example because
    /// nothing installed it, or when the directory of that Node cannot be an
    /// entry of `PATH`.
    ///
    /// [node]: Markdownlint::node_requirement
    /// [requirement]: Markdownlint::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let node = Tool::resolve_requirement(&Self::node_requirement(), context).await?;
        let tool = Tool::resolve_requirement(&Self::requirement(), context)
            .await?
            .with_toolchain(&node)?;

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
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use super::*;

    /// The start of the usage text that an older markdownlint writes
    const USAGE: &str = "Usage: markdownlint [options] [files|directories|globs...]\n";

    /// A report of a run that found a problem
    const REPORT: &str = r#"[{"fileName":"notes.md","lineNumber":1}]"#;

    /// Returns the context of a run in the root of this repository
    ///
    /// The repository pins a Node that the range of the crate accepts, and
    /// it installs the markdownlint that the crate declares. The crate lives
    /// two directories below the root.
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

    #[tokio::test]
    async fn resolve_runs_markdownlint_on_the_node_of_the_project() {
        let context = context();
        let node = Tool::resolve_requirement(&Markdownlint::node_requirement(), &context)
            .await
            .expect("the test resolves the Node that the repository pins");

        let markdownlint = Markdownlint::resolve(&context)
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
