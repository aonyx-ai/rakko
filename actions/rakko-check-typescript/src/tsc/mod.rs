//! The tsc that a project runs
//!
//! This module holds the compiler that mise installed for a project and the
//! run that produces its diagnostics. An action asks for a run, and everything
//! between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that tsc wrote
pub mod report;

use rakko_action::ProjectRoot;
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::ObserveTscError;
use crate::diagnostic::Diagnostic;

/// The name that mise knows the tool by
const TSC: &str = "tsc";

/// The option that stops tsc from writing the output of a build
///
/// Tsc is the compiler of the project as well as its type checker, and it
/// writes the JavaScript of every file that it reads unless it is told not to.
/// A check that emitted would leave files behind that whoever started it did
/// not ask for, in the directory that the configuration of the project names
/// for them. The option selects the diagnosis and not the build.
const NO_EMIT: &str = "--noEmit";

/// The option that decides how tsc draws its diagnostics
///
/// Tsc draws the source of a diagnostic underneath it with the place marked in
/// color when it writes to a terminal, and it writes one line per diagnostic
/// otherwise, so the report of a run would depend on where the run happened.
/// The option settles that, and it selects the presentation of the report and
/// not the behavior of the compiler.
const PRETTY: &str = "--pretty";

/// The value that asks tsc for its diagnostics without the drawing
const PLAIN: &str = "false";

/// The option that names the project that tsc checks
const PROJECT: &str = "-p";

/// The place that a run tells tsc to look
///
/// A run starts in the root of the project, so the working directory is the
/// project, and tsc reports every path relative to it.
const HERE: &str = ".";

/// The tsc that a project runs
///
/// The value holds the compiler that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// editor and the terminal of a contributor. The version decides what the
/// language is: a release of TypeScript adds rules, and a program that one
/// version accepts is a program that the next one can refuse. Nothing here
/// installs a tool: provisioning is the job of mise, and a tsc that mise does
/// not report stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_check_typescript::Tsc;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// let tsc = Tsc::resolve(root).await?;
/// let diagnostics = tsc.observe().await?;
///
/// println!("{} diagnostics", diagnostics.len());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Tsc {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Tsc {
    /// Runs tsc over the project and reads what it reported
    ///
    /// The run names the root of the project, asks for no output, and asks for
    /// the diagnostics without the drawing. Tsc writes its diagnostics on the
    /// standard output stream, whatever they are about.
    ///
    /// The diagnostics are the answer, and the status of the process decides
    /// only whether a run without one is trusted. Tsc reports a diagnostic for
    /// everything that it finds, including a configuration that it refuses and
    /// a project that it finds no configuration of, so a run that reported
    /// nothing and ended without success stopped for a reason of its own.
    ///
    /// # Errors
    ///
    /// Returns [`TscUnavailable`][unavailable] when tsc does not run,
    /// [`MissingReport`][missing] when tsc reported nothing and ended without
    /// success, and [`UnreadableReport`][unreadable] when it wrote a report
    /// that this crate cannot read.
    ///
    /// [missing]: ObserveTscError::MissingReport
    /// [unavailable]: ObserveTscError::TscUnavailable
    /// [unreadable]: ObserveTscError::UnreadableReport
    // checktypescript[impl check.configuration]
    // checktypescript[impl check.read]
    // checktypescript[impl run.noemit]
    // checktypescript[impl run.plain]
    // checktypescript[impl run.project]
    pub async fn observe(&self) -> Result<Vec<Diagnostic>, ObserveTscError> {
        let execution = self
            .tool
            .invocation()
            .arg(NO_EMIT)
            .arg(PRETTY)
            .arg(PLAIN)
            .arg(PROJECT)
            .arg(HERE)
            .run()
            .await
            .map_err(|source| ObserveTscError::TscUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy().into_owned();

        let diagnostics =
            self::report::read(&stdout).map_err(|source| ObserveTscError::UnreadableReport {
                report: stdout.clone(),
                source,
            })?;

        if diagnostics.is_empty() && !execution.status().success() {
            let stderr = execution.stderr().to_string_lossy();

            return Err(ObserveTscError::silent(&stdout, &stderr));
        }

        Ok(diagnostics)
    }

    /// Returns the tsc that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no tsc for the
    /// project.
    // checktypescript[impl tool.missing]
    // checktypescript[impl tool.tsc]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(TSC), root).await?;

        Ok(Self { tool })
    }
}
