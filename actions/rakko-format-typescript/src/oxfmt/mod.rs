//! The oxfmt that a project runs
//!
//! This module holds the version of oxfmt that the crate declares and the
//! program that mise installed at that version. It also holds the two runs
//! that an action asks for. These are the list of the files that a rewrite
//! would change and the rewrite itself. An action asks for a run, and
//! everything between the action and the process lives here.

mod error;

use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion, VersionRange};
use rakko_tool::{Launch, MiseFile, ResolveRequirementError, Tool};

pub(crate) use self::error::ObserveOxfmtError;
use crate::observation::Observation;

/// The name that mise knows the tool by, which is also the name of its
/// program
const OXFMT: &str = "oxfmt";

/// The version of oxfmt that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(OXFMT);

/// The name that mise knows Node by, which is also the name of its program
const NODE: &str = "node";

/// The versions of Node that oxfmt runs on
///
/// Node is a toolchain of the project, so the project pins its version, and
/// a run stops when that version is outside the range. The range is the one
/// of the package: 20.19 and later releases of 20, and 22.12 and later
/// releases.
const NODE_RANGE: &str = "^20.19.0 || >=22.12.0";

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
/// that the project pins, when the range of the crate accepts it. Nothing here
/// installs a tool: provisioning is the job of mise, and an oxfmt that mise
/// does not report stops the caller.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Oxfmt {
    /// The program that mise installed at the declared version
    tool: Tool,
}

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

    /// Returns the requirement of the Node that oxfmt runs on
    ///
    /// Node is a toolchain, so the requirement holds the range of versions
    /// that oxfmt accepts, and the project pins the version.
    pub(crate) fn node_requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(NODE)
            .executable(NODE)
            .version(RequiredVersion::Range(VersionRange::new(NODE_RANGE)))
            .build()
    }

    /// Returns the requirement of the oxfmt that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(OXFMT)
            .executable(OXFMT)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the oxfmt that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of oxfmt in the project does not change the answer. It
    /// also asks mise for the Node that the project pins and that
    /// [`node_requirement`][node] accepts. A run starts in the root of the
    /// context, with that Node first on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] in these conditions:
    ///
    /// - Mise does not run.
    /// - The project pins no Node in the range.
    /// - Mise reports no oxfmt at the declared version or no Node at the
    ///   pinned one, for example because nothing installed it.
    /// - The directory of that Node cannot be an entry of `PATH`.
    ///
    /// [node]: Oxfmt::node_requirement
    /// [requirement]: Oxfmt::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let node = Tool::resolve_requirement()
            .requirement(&Self::node_requirement())
            .context(context)
            .call()
            .await?;
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
            .context(context)
            .call()
            .await?
            .with_toolchain(&node)?;

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
            .launch()
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

    /// Returns the context of a run in the root of this repository
    ///
    /// The repository pins a Node that the range of the crate accepts, and
    /// it installs the oxfmt that the crate declares. The crate lives two
    /// directories below the root.
    fn context() -> Context {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository");

        Context::builder().root(root).build()
    }

    #[tokio::test]
    async fn resolve_runs_oxfmt_on_the_node_of_the_project() {
        let context = context();
        let node = Tool::resolve_requirement()
            .requirement(&Oxfmt::node_requirement())
            .context(&context)
            .call()
            .await
            .expect("the test resolves the Node that the repository pins");

        let oxfmt = Oxfmt::resolve(&context)
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
