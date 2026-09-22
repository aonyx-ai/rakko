//! The oxfmt that a project runs
//!
//! This module holds the program that mise installed for a project and the two
//! runs that an action asks for: the list of the files that a rewrite would
//! change, and the rewrite itself. An action asks for a run, and everything
//! between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;

use rakko_action::ProjectRoot;
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::ObserveOxfmtError;
use crate::observation::Observation;

/// The name that mise knows the tool by
const OXFMT: &str = "oxfmt";

/// The option that asks oxfmt to name the files that a rewrite would change
///
/// Oxfmt writes one path per file and rewrites nothing. Its other reporting
/// operation counts the files and draws a line per file with the time that it
/// took, which says nothing more about what is wrong and costs a reading of
/// the decoration that oxfmt puts around it. The option selects the
/// presentation of the report and not the behavior of the tool.
const LIST: &str = "--list-different";

/// The option that asks oxfmt to rewrite the files that it can format
///
/// Oxfmt rewrites a project when no operation is named at all, so a run that
/// only reports names its operation as deliberately as a run that rewrites.
const WRITE: &str = "--write";

/// The files that a run of the action names to oxfmt
///
/// Oxfmt formats more languages than this action asks it about, and the
/// actions that wrap prettier own those languages in a project that mounts
/// them, so the pattern names the extensions of this one and oxfmt never sees
/// the rest. The name of the action says TypeScript and the pattern holds
/// JavaScript as well, because a TypeScript project carries configuration and
/// scripts in that language and the tool reads both with one parser.
///
/// Oxfmt skips the dependencies of a Node project and reads the ignore files
/// of the project on its own, so the extensions are the whole of what the
/// action decides about the selection.
const PATTERN: &str = "**/*.{ts,tsx,mts,cts,js,jsx,mjs,cjs}";

/// The oxfmt that a project runs
///
/// The value holds the program that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. Nothing here installs a tool:
/// provisioning is the job of mise, and an oxfmt that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_format_typescript::Oxfmt;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// let oxfmt = Oxfmt::resolve(root).await?;
/// let observation = oxfmt.list().await?;
///
/// println!("{} problems", observation.problems().len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Oxfmt {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Oxfmt {
    /// Names the files that a rewrite would change, and changes nothing
    ///
    /// Oxfmt writes the paths on its standard output stream and everything
    /// that it could not do on its standard error stream, and the observation
    /// carries both.
    ///
    /// # Errors
    ///
    /// Returns [`OxfmtUnavailable`][unavailable] when oxfmt does not run.
    ///
    /// [unavailable]: ObserveOxfmtError::OxfmtUnavailable
    // formattypescript[impl check.operation]
    // formattypescript[impl check.read]
    // formattypescript[impl files.extensions]
    pub async fn list(&self) -> Result<Observation, ObserveOxfmtError> {
        self.observe(LIST).await
    }

    /// Returns the oxfmt that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no oxfmt for the
    /// project.
    // formattypescript[impl tool.missing]
    // formattypescript[impl tool.oxfmt]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(OXFMT), root).await?;

        Ok(Self { tool })
    }

    /// Rewrites the files of the project that oxfmt can format
    ///
    /// Oxfmt names no file that it rewrote, so the observation of this run
    /// says what oxfmt could not do and nothing about what it did. A caller
    /// that wants to know what changed asks for the list before and after.
    ///
    /// # Errors
    ///
    /// Returns [`OxfmtUnavailable`][unavailable] when oxfmt does not run.
    ///
    /// [unavailable]: ObserveOxfmtError::OxfmtUnavailable
    // formattypescript[impl files.extensions]
    // formattypescript[impl fix.write]
    pub async fn rewrite(&self) -> Result<Observation, ObserveOxfmtError> {
        self.observe(WRITE).await
    }

    /// Runs oxfmt with the given operation and reads what it reported
    ///
    /// # Errors
    ///
    /// Returns [`OxfmtUnavailable`][unavailable] when oxfmt does not run.
    ///
    /// [unavailable]: ObserveOxfmtError::OxfmtUnavailable
    async fn observe(&self, operation: &str) -> Result<Observation, ObserveOxfmtError> {
        let execution = self
            .tool
            .invocation()
            .arg(operation)
            .arg(PATTERN)
            .run()
            .await
            .map_err(|source| ObserveOxfmtError::OxfmtUnavailable { source })?;

        Ok(Observation::read(&execution))
    }
}
