//! The node that a project runs
//!
//! This module holds the runtime that mise installed for a project and the run
//! of its test runner. An action asks for a run, and everything between the
//! action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that the test runner wrote
pub mod report;

use rakko_action::ProjectRoot;
use rakko_tool::{ResolveToolError, Tool, ToolName};

pub use self::error::ObserveNodeError;
use self::report::Report;

/// The name that mise knows the tool by
const NODE: &str = "node";

/// The option that starts the test runner of Node
///
/// A run that names no file after it leaves the discovery of the test files to
/// Node, which walks the directory that the run started in with the patterns
/// that it carries.
const TEST: &str = "--test";

/// The option that decides how the runner reports a run
///
/// The runner draws a run for a reader otherwise, with a summary that a person
/// reads and a machine does not, and the drawing changes with the width of a
/// terminal. This selects the presentation of the report and not the behavior
/// of the runner.
const REPORTER: &str = "--test-reporter=tap";

/// The node that a project runs
///
/// The value holds the runtime that mise installed for the project, at the
/// version that the project pinned, so a run reaches the same program as the
/// terminal of a contributor. The version decides what the test runner does:
/// it gained the discovery of its files, the reading of TypeScript, and the
/// shape of its report over several releases of Node. Nothing here installs a
/// tool: provisioning is the job of mise, and a node that mise does not report
/// stops the caller.
///
/// # Examples
///
/// ```no_run
/// use rakko_action::ProjectRoot;
/// use rakko_test_typescript::Node;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let root = ProjectRoot::new("/home/otter/project".into());
///
/// let node = Node::resolve(root).await?;
/// let report = node.observe().await?;
///
/// println!("{} tests", report.tests());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Node {
    /// The program that mise installed for the project
    tool: Tool,
}

impl Node {
    /// Runs the tests of the project and reads what the runner reported
    ///
    /// The run starts in the root of the project and names no file, so the
    /// discovery of Node selects the tests. The runner writes its report on
    /// the standard output stream.
    ///
    /// The report is the answer, and the status of the process decides only
    /// whether a run without a failure is trusted. The runner reports every
    /// test that failed, so a run that reported none and ended without success
    /// stopped for a reason of its own.
    ///
    /// # Errors
    ///
    /// Returns [`NodeUnavailable`][unavailable] when node does not run,
    /// [`MissingFailure`][missing] when the runner reported no failure and
    /// ended without success, and [`UnreadableReport`][unreadable] when it
    /// wrote a report that this crate cannot read.
    ///
    /// [missing]: ObserveNodeError::MissingFailure
    /// [unavailable]: ObserveNodeError::NodeUnavailable
    /// [unreadable]: ObserveNodeError::UnreadableReport
    // testtypescript[impl run.project]
    // testtypescript[impl run.read]
    // testtypescript[impl run.tap]
    pub async fn observe(&self) -> Result<Report, ObserveNodeError> {
        let execution = self
            .tool
            .invocation()
            .arg(TEST)
            .arg(REPORTER)
            .run()
            .await
            .map_err(|source| ObserveNodeError::NodeUnavailable { source })?;

        let stdout = execution.stdout().to_string_lossy().into_owned();

        let report =
            self::report::read(&stdout).map_err(|source| ObserveNodeError::UnreadableReport {
                report: stdout.clone(),
                source,
            })?;

        if report.failures().is_empty() && !execution.status().success() {
            let stderr = execution.stderr().to_string_lossy();

            return Err(ObserveNodeError::silent(&stdout, &stderr));
        }

        Ok(report)
    }

    /// Returns the node that mise installed for the project
    ///
    /// The lookup asks mise about the project whose root the caller names, so
    /// the version that the project pinned answers, whatever the shell that
    /// started the process carries on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when mise reports no node for the
    /// project.
    // testtypescript[impl tool.missing]
    // testtypescript[impl tool.node]
    pub async fn resolve(root: ProjectRoot) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve(ToolName::new(NODE), root).await?;

        Ok(Self { tool })
    }
}
