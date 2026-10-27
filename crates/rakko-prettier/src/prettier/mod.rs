//! The prettier that a project runs
//!
//! This module holds the program that mise installed for a project, the run
//! of one operation, and the files that such a run reads and writes. An
//! action states which operation it wants and which files it cares about, and
//! everything between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;

use std::iter;

use rakko_action::{Access, FileAccess, Files, Pattern, ProjectRoot};
use rakko_tool::{Launch, ResolveToolError, Tool, ToolName};

pub use self::error::ObservePrettierError;
use crate::filter::Filter;
use crate::observation::Observation;
use crate::operation::Operation;

/// The name that mise knows the tool by
const PRETTIER: &str = "prettier";

/// The patterns of the files that prettier reads as its configuration
///
/// Prettier looks for a configuration file and an `.editorconfig` in the
/// directory of each file that it formats and in every directory above it,
/// so these patterns cover every directory of the project. A configuration
/// file is `.prettierrc` in one of its forms, `prettier.config` in one of its
/// forms, or a `package.json` or `package.yaml` that can hold a `prettier`
/// key. Prettier reads its two ignore files only in the root of the project,
/// where every run starts.
const CONFIGURATION: [&str; 8] = [
    "**/.editorconfig",
    "**/.prettierrc",
    "**/.prettierrc.{json,yml,yaml,json5,js,ts,mjs,mts,cjs,cts,toml}",
    "**/package.json",
    "**/package.yaml",
    "**/prettier.config.{js,ts,mjs,mts,cjs,cts}",
    ".gitignore",
    ".prettierignore",
];

/// The flag that asks prettier to name the files that it would rewrite
const LIST_DIFFERENT: &str = "--list-different";

/// The flag that asks prettier to rewrite the files that it can format
const WRITE: &str = "--write";

/// The flag that lets prettier skip a file of a language it does not know
///
/// A pattern selects files by their extension, and prettier refuses a file
/// that it cannot assign to a language. The flag turns that refusal into a
/// skip, which is what keeps a pattern for every extension usable.
const IGNORE_UNKNOWN: &str = "--ignore-unknown";

/// The flag that asks prettier for a report without color codes
///
/// The report is read as data, and a color code inside the marker of a line
/// would corrupt the reading. Prettier colors its report when it takes its
/// output for a terminal, and it takes a continuous integration job for one,
/// so a run that asked for nothing would be read correctly on the machine of
/// a contributor and not in the job that gates a pull request.
///
/// The flag selects the presentation of the report and not the behavior of
/// the tool: what prettier does to a project comes from the configuration of
/// that project alone.
const NO_COLOR: &str = "--no-color";

/// The prettier that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and a prettier that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_prettier::{FileExtension, Filter, Operation, Prettier};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let filter = Filter::new([FileExtension::new("md")]);
/// let prettier = Prettier::resolve(ProjectRoot::new("/home/otter/project".into())).await?;
///
/// let observation = prettier.observe(Operation::Report, &filter).await?;
///
/// println!("{} problems", observation.problems().len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Prettier {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Prettier {
    /// Returns the files that a run of prettier over the filter reads and
    /// writes
    ///
    /// The answer is the declaration of an action that runs prettier with the
    /// given access. Every run reads the configuration of prettier and the
    /// files that mise reads to find prettier. A run that reads also reads
    /// the files that the filter selects, and it writes no file. A run that
    /// writes rewrites the files that the filter selects, so it writes them
    /// and reads only the configuration.
    ///
    /// The patterns cover the files that prettier selects and reads when the
    /// project configures nothing else. A configuration that makes prettier
    /// read another file, such as a shared configuration from a package, is
    /// outside the patterns.
    ///
    /// A filter whose pattern is not a valid glob, such as one with an
    /// extension that holds an unclosed `[`, describes no files that the
    /// answer can name. Its answer is then the whole project for each side,
    /// which is the answer that is always safe.
    ///
    /// The function does no I/O, so a declaration can call it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_action::{Access, Files, Pattern};
    /// use rakko_prettier::{FileExtension, Filter, Prettier};
    ///
    /// let filter = Filter::new([FileExtension::new("md")]);
    ///
    /// let files = Prettier::files(Access::Write, &filter);
    ///
    /// assert_eq!(
    ///     files.writes(),
    ///     &Files::Matching(vec![Pattern::from_static("**/*.md")])
    /// );
    /// ```
    pub fn files(access: Access, filter: &Filter) -> FileAccess {
        let Ok(selection) = filter.pattern().parse::<Pattern>() else {
            return FileAccess::default();
        };

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

    /// Runs one operation over the files that the filter selects
    ///
    /// # Errors
    ///
    /// Returns [`PrettierUnavailable`][unavailable] when prettier does not
    /// run.
    ///
    /// [unavailable]: ObservePrettierError::PrettierUnavailable
    pub async fn observe(
        &self,
        operation: Operation,
        filter: &Filter,
    ) -> Result<Observation, ObservePrettierError> {
        let flag = match operation {
            Operation::Report => LIST_DIFFERENT,
            Operation::Rewrite => WRITE,
        };

        let execution = self
            .tool
            .invocation()
            .arg(flag)
            .arg(NO_COLOR)
            .arg(IGNORE_UNKNOWN)
            .arg(filter.pattern())
            .launch()
            .await
            .map_err(|source| ObservePrettierError::PrettierUnavailable { source })?;

        Ok(Observation::read(&execution, operation))
    }

    /// Returns the prettier that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no prettier for the
    /// project.
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(PRETTIER), root).await?;

        Ok(Self { tool })
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;
    use crate::filter::FileExtension;

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

    /// Returns the filter of the Markdown files
    fn markdown() -> Filter {
        Filter::new([FileExtension::new("md")])
    }

    #[test]
    fn files_of_a_filter_that_is_no_glob_cover_the_whole_project() {
        let filter = Filter::new([FileExtension::new("[md")]);

        let files = Prettier::files(Access::Write, &filter);

        assert_eq!(files, FileAccess::default());
    }

    #[test]
    fn files_of_a_read_cover_the_selection_and_the_configuration() {
        let mut expected = patterns(["**/*.md"]);
        expected.extend(configuration());

        let files = Prettier::files(Access::Read, &markdown());

        assert_eq!(files.reads(), &Files::Matching(expected));
    }

    #[test]
    fn files_of_a_read_write_no_file() {
        let files = Prettier::files(Access::Read, &markdown());

        assert_eq!(files.writes(), &Files::Matching(Vec::new()));
    }

    #[test]
    fn files_of_a_write_read_the_configuration() {
        let files = Prettier::files(Access::Write, &markdown());

        assert_eq!(files.reads(), &Files::Matching(configuration()));
    }

    #[test]
    fn files_of_a_write_write_the_selection() {
        let files = Prettier::files(Access::Write, &markdown());

        assert_eq!(files.writes(), &Files::Matching(patterns(["**/*.md"])));
    }
}
