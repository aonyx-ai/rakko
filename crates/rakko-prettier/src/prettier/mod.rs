//! The prettier that a project runs
//!
//! This module holds the version of prettier that the crate declares, the
//! range of Node that it accepts, the program that mise installed at that
//! version, and the run of one operation. An action states which operation
//! it wants and which files it cares about, and everything between the action
//! and the process lives here.

/// The error that leaves a run without an answer
mod error;

use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion, VersionRange};
use rakko_tool::{Launch, ResolveRequirementError, Tool, pinned_version};

pub use self::error::ObservePrettierError;
use crate::filter::Filter;
use crate::observation::Observation;
use crate::operation::Operation;

/// The name that mise knows the tool by
const PRETTIER: &str = "npm:prettier";

/// The program of the tool
const PROGRAM: &str = "prettier";

/// The version of prettier that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str = pinned_version(include_str!("../../mise.rakko.toml"), PRETTIER);

/// The name that mise knows Node by, which is also the name of its program
const NODE: &str = "node";

/// The versions of Node that prettier runs on
///
/// The range is the one that the package of prettier states for its engine.
/// Node is a toolchain of the project, so the project pins its version, and
/// a run stops when that version is outside the range.
const NODE_RANGE: &str = ">=14";

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
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it, so a run
/// reaches the version that the tests of the crate cover. Prettier runs on
/// the Node that the project pins, when the range of the crate accepts it.
/// Nothing here installs a tool: provisioning is the job of mise, and a
/// prettier or a Node that mise does not report stops the caller.
///
/// An action that runs prettier declares [`requirement`][requirement] and
/// [`node_requirement`][node] among its tools, so that the machinery can read
/// the versions.
///
/// # Examples
///
/// ```no_run
/// use rakko::Context;
/// use rakko_prettier::{FileExtension, Filter, Operation, Prettier};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let filter = Filter::new([FileExtension::new("md")]);
/// let context = Context::builder().root("/home/otter/project").build();
/// let prettier = Prettier::resolve(&context).await?;
///
/// let observation = prettier.observe(Operation::Report, &filter).await?;
///
/// println!("{} problems", observation.problems().len());
/// # Ok(())
/// # }
/// ```
///
/// [node]: Prettier::node_requirement
/// [requirement]: Prettier::requirement
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Prettier {
    /// The program that mise installed at the declared version, on the Node
    /// of the project
    tool: Tool,
}

impl Prettier {
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

    /// Returns the requirement of the Node that prettier runs on
    ///
    /// Node is a toolchain, so the requirement holds the range of versions
    /// that prettier accepts, and the project pins the version. An action
    /// that runs prettier returns it from its declaration of tools, next to
    /// [`requirement`][requirement].
    ///
    /// [requirement]: Prettier::requirement
    pub fn node_requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(NODE)
            .executable(NODE)
            .version(RequiredVersion::Range(VersionRange::new(NODE_RANGE)))
            .build()
    }

    /// Returns the requirement of the prettier that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins. An action
    /// that runs prettier returns it from its declaration of tools, next to
    /// [`node_requirement`][node].
    ///
    /// [node]: Prettier::node_requirement
    pub fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(PRETTIER)
            .executable(PROGRAM)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the prettier that mise installed at the version of the crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of prettier in the project does not change the answer.
    /// It also asks mise for the Node that the project pins and that
    /// [`node_requirement`][node] accepts. A run of prettier starts in the
    /// root of the context, with that Node first on its path.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, when the
    /// project pins no Node in the range, when mise reports no prettier at the
    /// declared version or no Node at the pinned one, for example because
    /// nothing installed it, or when the directory of that Node cannot be an
    /// entry of `PATH`.
    ///
    /// [node]: Prettier::node_requirement
    /// [requirement]: Prettier::requirement
    pub async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
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
    /// The repository pins a Node that the range of the crate accepts, and
    /// it installs the prettier that the crate declares. The crate lives two
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
    async fn resolve_runs_prettier_on_the_node_of_the_project() {
        let context = context();
        let node = Tool::resolve_requirement(&Prettier::node_requirement(), &context)
            .await
            .expect("the test resolves the Node that the repository pins");

        let prettier = Prettier::resolve(&context)
            .await
            .expect("the test resolves the prettier that the crate declares");

        let invocation = prettier.tool.invocation();
        let path = invocation
            .environment()
            .iter()
            .find(|variable| variable.name().get() == "PATH")
            .map(|variable| variable.value().get().to_owned())
            .expect("expected the command of prettier to set the path");
        assert_eq!(
            std::env::split_paths(&path).next().as_deref(),
            node.program().get().parent()
        );
    }
}
