//! The node that a project runs
//!
//! This module holds the versions of Node that the crate accepts, the runtime
//! that mise installed at the version that the project pins, and the run of
//! its test runner. An action asks for a run, and everything between the
//! action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that the test runner wrote
pub(crate) mod report;

use rakko::{Context, RequiredVersion, ToolRequirement, VersionRange};
use rakko_tool::{Launch, ResolveRequirementError, Tool};

pub(crate) use self::error::ObserveNodeError;
use self::report::Report;

/// The name that mise knows Node by, which is also the name of its program
const NODE: &str = "node";

/// The versions of Node that the action runs the tests on
///
/// Node is a toolchain of the project, so the project pins its version, and a
/// run stops when that version is outside the range. Node finds and runs a
/// test file in TypeScript only in the releases that strip types by default:
/// 22.18 and later releases of 22, and 23.6 and later. An older release leaves
/// out every such test, and it says nothing about it.
const NODE_RANGE: &str = "^22.18 || >=23.6";

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
/// shape of its report over several releases of Node. Node is therefore a
/// toolchain of the project, and the crate states only the range of versions
/// that it accepts. Nothing here installs a tool: provisioning is the job of
/// mise, and a node that mise does not report stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Node {
    /// The program that mise installed at the version that the project pins
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
    pub(crate) async fn observe(&self) -> Result<Report, ObserveNodeError> {
        let execution = self
            .tool
            .invocation()
            .arg(TEST)
            .arg(REPORTER)
            .launch()
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

    /// Returns the requirement of the Node that the action runs the tests on
    ///
    /// Node is a toolchain, so the requirement holds the range of versions
    /// that the action accepts, and the project pins the version.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(NODE)
            .executable(NODE)
            .version(RequiredVersion::Range(VersionRange::new(NODE_RANGE)))
            .build()
    }

    /// Returns the node that mise installed at the version that the project
    /// pins
    ///
    /// The lookup asks mise for the Node that the project pins and that
    /// [`requirement`][requirement] accepts. A run starts in the root of the
    /// context.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, when the
    /// project pins no Node in the range, or when mise reports no node at the
    /// pinned version, for example because nothing installed it.
    ///
    /// [requirement]: Node::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement(&Self::requirement(), context).await?;

        Ok(Self { tool })
    }
}
