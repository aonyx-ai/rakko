//! The oxlint that a project runs
//!
//! This module holds the program that mise installed for a project, the run
//! that produces a report, and the files that such a run reads. An action asks
//! for a run, and everything between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that oxlint wrote
pub mod report;

use rakko_action::{FileAccess, Files, Pattern, ProjectRoot};
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::ObserveOxlintError;
use crate::observation::Observation;

/// The name that mise knows the tool by
const OXLINT: &str = "oxlint";

/// The option that asks oxlint for its report as data
///
/// Oxlint draws a block of source per diagnostic by default, with the rule
/// above it and the places that it marks drawn into it. The same run writes the
/// diagnostics as one JSON object on request, and each of them then carries the
/// rule, the severity, the file, the place, and the message in fields instead
/// of in a block that a reader has to take apart. The object also counts the
/// files that the run examined, which no other format reports. The format
/// protects a run from its environment as well, because the default format
/// changes on a terminal and on a build server. The option selects the
/// presentation of the report and not the behavior of the tool.
const FORMAT: &str = "--format";

/// The name of the format that carries the diagnostics as data
const JSON: &str = "json";

/// The place that a run tells oxlint to look
///
/// A run starts in the root of the project, so the working directory is the
/// place, and oxlint reports every path relative to it.
const HERE: &str = ".";

/// The pattern of the files that oxlint selects
///
/// A run names the root of the project. Oxlint then selects every file whose
/// extension it lints, in every directory. It also looks in a directory whose
/// name starts with a dot. The extensions are those of TypeScript and
/// JavaScript, and those of the components of Vue, Svelte, and Astro, whose
/// scripts oxlint lints too.
const SELECTION: &str = "**/*.{ts,tsx,mts,cts,js,jsx,mjs,cjs,vue,svelte,astro}";

/// The patterns of the files that oxlint reads as its configuration
///
/// Oxlint looks for a configuration file and for the ignore files of ESLint
/// and git in every directory of the project. It reads the ignore rules of
/// `.git/info/exclude` as well. In the root of the project alone, it reads
/// `oxlint-suppressions.json`, which lists the problems that the project
/// suppresses.
const CONFIGURATION: [&str; 8] = [
    "**/.eslintignore",
    "**/.gitignore",
    "**/.oxlintrc.json",
    "**/.oxlintrc.jsonc",
    "**/oxlint.config.mts",
    "**/oxlint.config.ts",
    ".git/info/exclude",
    "oxlint-suppressions.json",
];

/// The oxlint that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and an oxlint that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_lint_typescript::Oxlint;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// let oxlint = Oxlint::resolve(root).await?;
/// let observation = oxlint.observe().await?;
///
/// if observation.examined() > 0 {
///     println!("{} problems", observation.problems().len());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Oxlint {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Oxlint {
    /// Returns the files that a run of oxlint reads and writes
    ///
    /// The answer is the declaration of an action that calls
    /// [`observe`][observe]. The run reads the files that oxlint lints, the
    /// configuration of oxlint, and the files that mise reads to find oxlint.
    /// It writes no file.
    ///
    /// The patterns cover the files that oxlint reads when the project
    /// configures nothing else. A configuration that makes oxlint read
    /// another file is outside the patterns. Examples are a file that its
    /// `extends` key names, and the `tsconfig.json` that its import plugin
    /// reads.
    ///
    /// The function does no I/O, so a declaration can call it.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko_action::Files;
    /// use rakko_lint_typescript::Oxlint;
    ///
    /// let files = Oxlint::files();
    ///
    /// assert_eq!(files.writes(), &Files::Matching(Vec::new()));
    /// ```
    ///
    /// [observe]: Oxlint::observe
    pub fn files() -> FileAccess {
        let reads = [SELECTION]
            .into_iter()
            .chain(CONFIGURATION)
            .map(Pattern::from_static)
            .chain(Tool::resolution_files())
            .collect();

        FileAccess::builder()
            .reads(Files::Matching(reads))
            .writes(Files::Matching(Vec::new()))
            .build()
    }

    /// Runs oxlint over the project and reads what it reported
    ///
    /// The run names the root of the project and asks for the report as data.
    /// Oxlint writes the report on its standard output stream, and it writes
    /// the diagnosis of a run that it could not start on the same stream.
    ///
    /// The status of the process answers nothing here, and the report answers
    /// everything. Oxlint ends with success for a project whose broken rules
    /// the project weighs as warnings, and it ends without success for a
    /// project that holds no file to lint, so a caller that read the status
    /// would pass a project with problems and fail a project with none.
    ///
    /// A run that examined no file is no failure. The observation reports it
    /// through the count of the files, because oxlint writes the same empty
    /// list of diagnostics for a project that it linted and found clean.
    ///
    /// # Errors
    ///
    /// Returns [`OxlintUnavailable`][unavailable] when oxlint does not run,
    /// [`MissingReport`][missing] when oxlint wrote no report, and
    /// [`UnreadableReport`][unreadable] when it wrote a report that this crate
    /// cannot read.
    ///
    /// [missing]: ObserveOxlintError::MissingReport
    /// [unavailable]: ObserveOxlintError::OxlintUnavailable
    /// [unreadable]: ObserveOxlintError::UnreadableReport
    pub async fn observe(&self) -> Result<Observation, ObserveOxlintError> {
        let execution = self
            .tool
            .invocation()
            .arg(FORMAT)
            .arg(JSON)
            .arg(HERE)
            .run()
            .await
            .map_err(|source| ObserveOxlintError::OxlintUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy();

        self::report::read(&stdout).map_err(|source| {
            ObserveOxlintError::of(source, &stdout, &execution.stderr().to_string_lossy())
        })
    }

    /// Returns the oxlint that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no oxlint for the
    /// project.
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(OXLINT), root).await?;

        Ok(Self { tool })
    }
}
