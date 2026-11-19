//! The prettier that a project runs
//!
//! This module holds the version of prettier that the crate declares. It also
//! holds the program that mise installed at that version, on the Node that the
//! project pins, and the run of one operation. An action states which
//! operation it wants and which files it cares about, and everything between
//! the action and the process lives here.

mod error;
mod resolve_error;

use rakko::{Action, Context, ToolOutput, Tools};
use rakko_tool::{Launch, ShownStreams, Tool};

pub use self::error::ObservePrettierError;
pub use self::resolve_error::ResolvePrettierError;
use crate::filter::Filter;
use crate::observation::Observation;
use crate::operation::Operation;

/// The name that mise knows the tool by
const PRETTIER: &str = "npm:prettier";

/// The program of the tool
const PROGRAM: &str = "prettier";

/// The name that mise knows Node by, which is also the name of its program
///
/// Prettier starts Node by this name, and Node is a toolchain of the
/// project. So the project pins its version, and the crate requires no
/// range of it.
const NODE: &str = "node";

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
/// The report is read as data. A color code inside the marker of a line would
/// corrupt the reading. Prettier colors its report when it takes its output for
/// a terminal, and it takes a continuous integration job for one. So a run that
/// asked for nothing would be read correctly on the machine of a contributor.
/// It would not be read correctly in the job that gates a pull request.
///
/// The flag selects the presentation of the report and not the behavior of the
/// tool. What prettier does to a project comes from the configuration of that
/// project alone.
const NO_COLOR: &str = "--no-color";

/// The prettier that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it, so a run
/// reaches the version that the tests of the crate cover.
/// Prettier runs on the Node that the project pins, and not on a Node that
/// comes first on the path of the environment.
/// Nothing here installs a tool: provisioning is the job of mise.
/// A prettier or a Node that mise does not report stops the caller.
///
/// An action that runs prettier adds [`TOOLS`][tools] to its tools, so that
/// the machinery can read the version, and a run can resolve prettier.
///
/// # Examples
///
/// ```no_run
/// use rakko::{Action, Context};
/// use aonyx_rakko_prettier::{FileExtension, Filter, Operation, Prettier};
///
/// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
/// let filter = Filter::new([FileExtension::new("md")]);
/// let context = Context::builder().root("/home/otter/project").build();
/// let prettier = Prettier::resolve().action(action).context(&context).call().await?;
///
/// let observation = prettier.observe(Operation::Report, &filter).await?;
///
/// println!("{} problems", observation.problems().len());
/// # Ok(())
/// # }
/// ```
///
/// [tools]: Prettier::TOOLS
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Prettier {
    /// The program that mise installed at the declared version, on the Node
    /// of the project
    tool: Tool,

    /// The tool output of the context that resolved prettier
    ///
    /// Each run sends what prettier writes to it.
    output: ToolOutput,
}

#[bon::bon]
impl Prettier {
    /// The tools that the mise file of the crate pins, which is prettier alone
    ///
    /// An action that runs prettier through this crate returns the constant
    /// from [`Action::additional_tools`]. The crate keeps the pin, so every
    /// action that uses it runs the version that the tests of the crate
    /// cover. Node is a toolchain of the project, so the constant holds no
    /// Node.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{Action, Context, Documented, Name, Outcome, Tools, action_name};
    /// use aonyx_rakko_prettier::Prettier;
    ///
    /// /// Formats the CSS files of the project
    /// #[derive(Documented)]
    /// struct FormatCss;
    ///
    /// impl Action for FormatCss {
    ///     type Args = ();
    ///
    ///     fn name(&self) -> Name {
    ///         action_name!("format-css")
    ///     }
    ///
    ///     fn additional_tools(&self) -> Vec<Tools> {
    ///         vec![Prettier::TOOLS]
    ///     }
    ///
    ///     async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
    ///         Outcome::Passed { summary: None }
    ///     }
    /// }
    ///
    /// assert_eq!(FormatCss.tools()[0].tool().get(), "npm:prettier");
    /// ```
    pub const TOOLS: Tools = rakko::bundled_tools!();

    /// Runs one operation over the files that the filter selects
    ///
    /// The run sends what prettier writes to the tool output of the context
    /// that resolved the prettier.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Action, Context};
    /// use aonyx_rakko_prettier::{FileExtension, Filter, Operation, Prettier};
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    /// let prettier = Prettier::resolve().action(action).context(&context).call().await?;
    /// let filter = Filter::new([FileExtension::new("md")]);
    /// let observation = prettier.observe(Operation::Report, &filter).await?;
    ///
    /// assert!(observation.problems().is_empty());
    /// # Ok(())
    /// # }
    /// ```
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
            .launch_with(&self.output, ShownStreams::Both)
            .await
            .map_err(|source| ObservePrettierError::PrettierUnavailable { source })?;

        Ok(Observation::read()
            .execution(&execution)
            .operation(operation)
            .call())
    }

    /// Returns the prettier that mise installed at the version that an action
    /// declares, on the Node that the project pins
    ///
    /// The lookup reads the tools that the action declares, which hold
    /// [`TOOLS`][tools]. It asks mise for prettier at the version that they
    /// give, so a pin of prettier in the project does not change the answer.
    /// It also asks mise for the Node that the project pins and that the
    /// requirements of the action accept. An action that requires nothing of
    /// Node runs on the first pin. A run of prettier starts in the root of the
    /// context, with that Node first on its path. So a Node that comes first
    /// on the path of the environment does not run prettier.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Action, Context};
    /// use aonyx_rakko_prettier::Prettier;
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    /// let prettier = Prettier::resolve().action(action).context(&context).call().await?;
    ///
    /// assert_eq!(prettier, Prettier::resolve().action(action).context(&context).call().await?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`UnresolvedTool`][tool] when the action does not declare
    /// prettier, or when mise reports no prettier at that version, for
    /// example because nothing installed it. Returns
    /// [`UnresolvedToolchain`][toolchain] when the project pins no Node that
    /// the action accepts, or when mise did not install that Node. It also
    /// returns that error when the directory of that Node cannot be an entry of
    /// `PATH`. Both errors also occur when mise does not run.
    ///
    /// [tool]: ResolvePrettierError::UnresolvedTool
    /// [toolchain]: ResolvePrettierError::UnresolvedToolchain
    /// [tools]: Prettier::TOOLS
    #[builder]
    pub async fn resolve<A: Action>(
        action: &A,
        context: &Context,
    ) -> Result<Self, ResolvePrettierError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(PRETTIER)
            .executable(PROGRAM)
            .context(context)
            .call()
            .await
            .map_err(|source| ResolvePrettierError::UnresolvedTool { source })?;
        let node = Tool::resolve_toolchain()
            .action(action)
            .toolchain(NODE)
            .executable(NODE)
            .context(context)
            .call()
            .await
            .map_err(|source| ResolvePrettierError::UnresolvedToolchain { source })?;
        let tool = tool
            .with_toolchain(&node)
            .map_err(|source| ResolvePrettierError::UnresolvedToolchain { source })?;

        Ok(Self {
            tool,
            output: context.tool_output().clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "An assertion in a test panics by design, and a test that resolves a tool expects mise to find it. A `# Panics` section on every test would repeat that and give the reader no information."
    )]
    #![allow(clippy::missing_panics_doc)]

    use std::path::Path;

    use rakko::{Documented, Name, Outcome, action_name};

    use super::*;

    /// An action that runs prettier through the crate and requires nothing
    /// of Node
    #[derive(Documented)]
    struct Formatting;

    impl Action for Formatting {
        type Args = ();

        fn name(&self) -> Name {
            action_name!("formatting")
        }

        fn additional_tools(&self) -> Vec<Tools> {
            vec![Prettier::TOOLS]
        }

        async fn run(&self, _context: &Context, _args: &Self::Args) -> Outcome {
            Outcome::Passed { summary: None }
        }
    }

    /// Returns the context of a run in the root of this repository
    ///
    /// The repository pins a Node, and it installs the prettier that the
    /// crate declares. The crate lives two directories below the root.
    fn context() -> Context {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .and_then(Path::parent)
            .expect("the crate lives two directories below the root of the repository");

        Context::builder().root(root).build()
    }

    #[test]
    fn prettier_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Prettier>();
    }

    #[tokio::test]
    async fn resolve_runs_prettier_on_the_node_of_the_project() {
        let context = context();
        let node = Tool::resolve_toolchain()
            .action(&Formatting)
            .toolchain(NODE)
            .executable(NODE)
            .context(&context)
            .call()
            .await
            .expect("the test resolves the Node that the repository pins");

        let prettier = Prettier::resolve()
            .action(&Formatting)
            .context(&context)
            .call()
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

    #[test]
    fn tools_declare_prettier_alone() {
        let tools: Vec<String> = Prettier::TOOLS
            .requirements()
            .iter()
            .map(|requirement| requirement.tool().get().to_owned())
            .collect();

        assert_eq!(tools, [PRETTIER]);
    }
}
