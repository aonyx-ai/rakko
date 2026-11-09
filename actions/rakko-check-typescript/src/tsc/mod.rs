//! The tsc that a project runs
//!
//! This module holds the versions of TypeScript and of Node that the crate
//! accepts, the compiler that mise installed at the version that the project
//! pins, and the run that produces its diagnostics. An action asks for a run,
//! and everything between the action and the process lives here.

/// The error that leaves a run without an answer
mod error;
/// The reading of the report that tsc wrote
pub(crate) mod report;

use rakko::{Context, RequiredVersion, ToolRequirement, VersionRange};
use rakko_tool::{Launch, ResolveRequirementError, Tool};

pub(crate) use self::error::ObserveTscError;
use crate::diagnostic::Diagnostic;

/// The name that mise knows TypeScript by
const TYPESCRIPT: &str = "npm:typescript";

/// The program of the compiler
const TSC: &str = "tsc";

/// The versions of TypeScript that the action reads the report of
///
/// TypeScript is a toolchain of the project, so the project pins its version,
/// and a run stops when that version is outside the range. The action reads
/// the diagnostics of TypeScript 5 and later, and it knows the number that the
/// diagnostic of a missing configuration has in each of these releases.
const TYPESCRIPT_RANGE: &str = ">=5";

/// The name that mise knows Node by, which is also the name of its program
const NODE: &str = "node";

/// The versions of Node that tsc runs on
///
/// Node is a toolchain of the project, so the project pins its version, and
/// a run stops when that version is outside the range. The range is the one
/// of the oldest TypeScript that the action accepts. A newer TypeScript can
/// need a newer Node, such as 16.20 and later for TypeScript 7.
const NODE_RANGE: &str = ">=12.20";

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
/// version accepts is a program that the next one can refuse. TypeScript is
/// therefore a toolchain of the project, and the crate states only the range
/// of versions that it accepts. The compiler runs on the Node that the project
/// pins, when the range of the crate accepts it. Nothing here installs a tool:
/// provisioning is the job of mise, and a tsc that mise does not report stops
/// the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Tsc {
    /// The program that mise installed at the version that the project pins
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
    pub(crate) async fn observe(&self) -> Result<Vec<Diagnostic>, ObserveTscError> {
        let execution = self
            .tool
            .invocation()
            .arg(NO_EMIT)
            .arg(PRETTY)
            .arg(PLAIN)
            .arg(PROJECT)
            .arg(HERE)
            .launch()
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

    /// Returns the requirement of the Node that tsc runs on
    ///
    /// Node is a toolchain, so the requirement holds the range of versions
    /// that tsc accepts, and the project pins the version.
    pub(crate) fn node_requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(NODE)
            .executable(NODE)
            .version(RequiredVersion::Range(VersionRange::new(NODE_RANGE)))
            .build()
    }

    /// Returns the requirement of the TypeScript that the action checks with
    ///
    /// TypeScript is a toolchain, so the requirement holds the range of
    /// versions that the action accepts, and the project pins the version.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(TYPESCRIPT)
            .executable(TSC)
            .version(RequiredVersion::Range(VersionRange::new(TYPESCRIPT_RANGE)))
            .build()
    }

    /// Returns the tsc that mise installed at the version that the project
    /// pins
    ///
    /// The lookup asks mise for the TypeScript that the project pins and that
    /// [`requirement`][requirement] accepts. It also asks mise for the Node
    /// that the project pins and that [`node_requirement`][node] accepts. A
    /// run starts in the root of the context, with that Node first on its
    /// path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] in these conditions:
    ///
    /// - Mise does not run.
    /// - The project pins no TypeScript or no Node in the range.
    /// - Mise reports no tsc or no Node at the pinned version, for example
    ///   because nothing installed it.
    /// - The directory of that Node cannot be an entry of `PATH`.
    ///
    /// [node]: Tsc::node_requirement
    /// [requirement]: Tsc::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let node = Tool::resolve_requirement(&Self::node_requirement(), context).await?;
        let tool = Tool::resolve_requirement(&Self::requirement(), context)
            .await?
            .with_toolchain(&node)?;

        Ok(Self { tool })
    }
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::expect_used)]
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use super::*;

    /// Returns the context of a run in the root of this repository
    ///
    /// The repository pins a TypeScript and a Node that the ranges of the
    /// crate accept, and it installs both. The crate lives two directories
    /// below the root.
    fn context() -> Context {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository");

        Context::builder().root(root).build()
    }

    #[tokio::test]
    async fn resolve_runs_tsc_on_the_node_of_the_project() {
        let context = context();
        let node = Tool::resolve_requirement(&Tsc::node_requirement(), &context)
            .await
            .expect("the test resolves the Node that the repository pins");

        let tsc = Tsc::resolve(&context)
            .await
            .expect("the test resolves the TypeScript that the repository pins");

        let invocation = tsc.tool.invocation();
        let path = invocation
            .environment()
            .iter()
            .find(|variable| variable.name().get() == "PATH")
            .map(|variable| variable.value().get().to_owned())
            .expect("expected the command of tsc to set the path");
        assert_eq!(
            std::env::split_paths(&path).next().as_deref(),
            node.program().get().parent()
        );
    }
}
