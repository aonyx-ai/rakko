//! The oxfmt that a project runs
//!
//! This module holds the version of oxfmt that the crate declares and the
//! program that mise installed at that version. It also holds the two runs
//! that an action asks for. These are the list of the files that a rewrite
//! would change and the rewrite itself. An action asks for a run, and
//! everything between the action and the process lives here.

mod error;
mod resolve_error;

use rakko::{Action, Context, ToolOutput};
use rakko_tool::{Launch, ShownStreams, Tool};

pub(crate) use self::error::ObserveOxfmtError;
pub(crate) use self::resolve_error::ResolveOxfmtError;
use crate::observation::Observation;

/// The name that mise knows the tool by, which is also the name of its
/// program
const OXFMT: &str = "oxfmt";

/// The name that mise knows Node by, which is also the name of its program
///
/// Oxfmt starts Node by this name, and Node is a toolchain of the project. So
/// the project pins its version, and the action requires no range of it.
const NODE: &str = "node";

/// The option that asks oxfmt to name the files that a rewrite would change
///
/// Oxfmt writes one path per file and rewrites nothing. Its other reporting
/// operation counts the files and draws a line per file with the time it took.
/// This says nothing more about what is wrong. It costs a reading of the
/// decoration that oxfmt puts around it. The option selects the presentation
/// of the report and not the behavior of the tool.
const LIST: &str = "--list-different";

/// The option that asks oxfmt to rewrite the files that it can format
///
/// Oxfmt rewrites a project when no operation is named at all. So a run that
/// only reports names its operation as deliberately as a run that rewrites.
const WRITE: &str = "--write";

/// The files that a run of the action names to oxfmt
///
/// Oxfmt formats more languages than this action asks it about. The actions
/// that wrap prettier own those languages in a project that mounts them. So the
/// pattern names the extensions of this one and oxfmt never sees the rest. The
/// name of the action says TypeScript. The pattern holds JavaScript as well,
/// because a TypeScript project carries configuration and scripts in that
/// language. The tool reads both with one parser.
///
/// Oxfmt skips the dependencies of a Node project. It reads the ignore files of
/// the project on its own. So the extensions are the whole of what the action
/// decides about the selection.
const PATTERN: &str = "**/*.{ts,tsx,mts,cts,js,jsx,mjs,cjs}";

/// The oxfmt that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it, so a run reaches
/// the version that the tests of the crate cover. The program runs on the Node
/// that the project pins. Nothing here installs a tool: provisioning is the job
/// of mise, and an oxfmt that mise does not report stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Oxfmt {
    /// The program that mise installed at the declared version
    tool: Tool,

    /// The tool output of the context that resolved oxfmt
    ///
    /// Each run sends what oxfmt writes to it.
    output: ToolOutput,
}

#[bon::bon]
impl Oxfmt {
    /// Names the files that a rewrite would change, and changes nothing
    ///
    /// Oxfmt writes the paths on its standard output stream. Everything that it
    /// could not do goes to its standard error stream. The observation carries
    /// both.
    ///
    /// # Errors
    ///
    /// Returns [`OxfmtUnavailable`][unavailable] when oxfmt does not run.
    ///
    /// [unavailable]: ObserveOxfmtError::OxfmtUnavailable
    pub(crate) async fn list(&self) -> Result<Observation, ObserveOxfmtError> {
        self.observe(LIST).await
    }

    /// Returns the oxfmt that mise installed at the version that the
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
    /// tool, or when mise reports no oxfmt at that version, for example
    /// because nothing installed it. Returns [`UnresolvedToolchain`][toolchain]
    /// when the project pins no Node, or when mise did not install that Node.
    /// It also returns that error when the directory of that Node cannot be an
    /// entry of `PATH`. Both errors also occur when mise does not run.
    ///
    /// [tool]: ResolveOxfmtError::UnresolvedTool
    /// [toolchain]: ResolveOxfmtError::UnresolvedToolchain
    #[builder]
    pub(crate) async fn resolve<A: Action>(
        action: &A,
        context: &Context,
    ) -> Result<Self, ResolveOxfmtError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(OXFMT)
            .executable(OXFMT)
            .context(context)
            .call()
            .await
            .map_err(|source| ResolveOxfmtError::UnresolvedTool { source })?;
        let node = Tool::resolve_toolchain()
            .action(action)
            .toolchain(NODE)
            .executable(NODE)
            .context(context)
            .call()
            .await
            .map_err(|source| ResolveOxfmtError::UnresolvedToolchain { source })?;
        let tool = tool
            .with_toolchain(&node)
            .map_err(|source| ResolveOxfmtError::UnresolvedToolchain { source })?;

        Ok(Self {
            tool,
            output: context.tool_output().clone(),
        })
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
    pub(crate) async fn rewrite(&self) -> Result<Observation, ObserveOxfmtError> {
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
            .launch_with(&self.output, ShownStreams::Both)
            .await
            .map_err(|source| ObserveOxfmtError::OxfmtUnavailable { source })?;

        Ok(Observation::read(&execution))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use std::path::Path;

    use super::*;
    use crate::FormatTypeScript;

    /// Returns the context of a run in the root of this repository
    ///
    /// The repository pins a Node, and it installs the oxfmt that the crate
    /// declares. The crate lives two directories below the root.
    fn context() -> Context {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository");

        Context::builder().root(root).build()
    }

    #[test]
    fn oxfmt_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Oxfmt>();
    }

    #[tokio::test]
    async fn resolve_runs_oxfmt_on_the_node_of_the_project() {
        let context = context();
        let node = Tool::resolve_toolchain()
            .action(&FormatTypeScript)
            .toolchain(NODE)
            .executable(NODE)
            .context(&context)
            .call()
            .await
            .expect("the test resolves the Node that the repository pins");

        let oxfmt = Oxfmt::resolve()
            .action(&FormatTypeScript)
            .context(&context)
            .call()
            .await
            .expect("the test resolves the oxfmt that the crate declares");

        let invocation = oxfmt.tool.invocation();
        let path = invocation
            .environment()
            .iter()
            .find(|variable| variable.name().get() == "PATH")
            .map(|variable| variable.value().get().to_owned())
            .expect("expected the command of oxfmt to set the path");
        assert_eq!(
            std::env::split_paths(&path).next().as_deref(),
            node.program().get().parent()
        );
    }
}
