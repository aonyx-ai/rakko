//! The oxlint that a project runs
//!
//! This module holds the program that mise installed for a project and the run
//! that produces a report. An action asks for a run, and everything between the
//! action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that oxlint wrote
pub mod report;

use rakko_action::ProjectRoot;
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
    // linttypescript[impl check.configuration]
    // linttypescript[impl check.read]
    // linttypescript[impl run.project]
    // linttypescript[impl run.structured]
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
    // linttypescript[impl tool.missing]
    // linttypescript[impl tool.oxlint]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(OXLINT), root).await?;

        Ok(Self { tool })
    }
}
