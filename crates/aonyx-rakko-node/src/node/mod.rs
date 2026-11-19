//! The lookups of Node and of the programs that run on it
//!
//! This module finds the Node that a project pins. It also finds a tool or a
//! toolchain that runs on that Node, and it gives that program the Node first
//! on its path.

mod resolve_node_tool_error;
mod resolve_node_toolchain_error;

use rakko::{Action, Context, Executable, MiseTool};
use rakko_tool::{ResolveToolchainError, Tool};

pub use self::resolve_node_tool_error::ResolveNodeToolError;
pub use self::resolve_node_toolchain_error::ResolveNodeToolchainError;

/// The name that mise knows Node by, which is also the name of its program
///
/// A package for Node starts Node by this name. Node is a toolchain of the
/// project, so the project pins its version.
const NODE: &str = "node";

/// The Node that a project pins, and the programs that run on it
///
/// The type has no values. It holds the lookups that an action calls, and
/// each lookup returns a [`Tool`].
///
/// Node is a toolchain of the project. So every lookup takes the first Node
/// that the project pins and that the requirements of the action accept, from
/// [`Action::toolchains`]. An action without a requirement runs on the first
/// pin. The lookups never search the `PATH` of the environment for Node.
///
/// A program that runs on Node gets the directory of that Node first on its
/// path, before the path that the process inherited. So a Node that comes
/// first on the inherited path does not run the program. Nothing here
/// installs a tool or a toolchain: provisioning is the job of mise.
///
/// # Examples
///
/// An action resolves the validator of Renovate, which starts Node by name:
///
/// ```no_run
/// use aonyx_rakko_node::Node;
/// use rakko::{Action, Context};
///
/// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
/// let context = Context::builder().root("/home/otter/project").build();
///
/// let validator = Node::resolve_tool()
///     .action(action)
///     .tool("npm:renovate")
///     .executable("renovate-config-validator")
///     .context(&context)
///     .call()
///     .await?;
///
/// println!("{}", validator.program());
/// # Ok(())
/// # }
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Node {}

#[bon::bon]
impl Node {
    /// Returns the program of the Node that the project pins
    ///
    /// The lookup takes the first Node that the project pins and that the
    /// requirements of the action accept. A command of the program starts in
    /// the root of the context. An action that runs Node itself, such as its
    /// test runner, calls this lookup.
    ///
    /// The lookup returns the error of [`Tool::resolve_toolchain`] as it is,
    /// because a wrapper adds nothing to that error. The error already gives
    /// the toolchain `node` and the repair.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolchainError`] when mise does not run, or when the
    /// project pins no Node that the requirements of the action accept. It
    /// also returns that error when mise did not install the pinned Node, or
    /// when that Node has no program `node`.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aonyx_rakko_node::Node;
    /// use rakko::{Action, Context};
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let node = Node::resolve().action(action).context(&context).call().await?;
    ///
    /// println!("{} {}", node.program(), node.version());
    /// # Ok(())
    /// # }
    /// ```
    #[builder]
    pub async fn resolve<A: Action>(
        action: &A,
        context: &Context,
    ) -> Result<Tool, ResolveToolchainError> {
        Tool::resolve_toolchain()
            .action(action)
            .toolchain(NODE)
            .executable(NODE)
            .context(context)
            .call()
            .await
    }

    /// Returns a program of a tool that the action declares, on the Node
    /// that the project pins
    ///
    /// The lookup finds the program of the tool at the version that the
    /// declarations of the action give, as [`Tool::resolve`] does. So a pin
    /// of the tool in the project does not change the answer. Then it finds
    /// Node, as [`resolve`][resolve] does. A command of the program starts in
    /// the root of the context, with the directory of that Node first on its
    /// path.
    ///
    /// The lookup finds the tool before Node. So when both are missing, the
    /// error is about the tool.
    ///
    /// # Errors
    ///
    /// Returns [`UnresolvedTool`][tool] when the action does not declare the
    /// tool. It also returns that error when mise reports no such program of
    /// the tool at that version, for example because nothing installed it.
    /// Returns [`UnresolvedNode`][node] when the project pins no Node that the
    /// requirements of the action accept, or when mise did not install that
    /// Node. It also returns that error when the directory of that Node cannot
    /// be an entry of `PATH`. Both errors also occur when mise does not run.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aonyx_rakko_node::Node;
    /// use rakko::{Action, Context};
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let markdownlint = Node::resolve_tool()
    ///     .action(action)
    ///     .tool("npm:markdownlint-cli")
    ///     .executable("markdownlint")
    ///     .context(&context)
    ///     .call()
    ///     .await?;
    ///
    /// println!("{}", markdownlint.program());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [node]: ResolveNodeToolError::UnresolvedNode
    /// [resolve]: Node::resolve
    /// [tool]: ResolveNodeToolError::UnresolvedTool
    #[builder]
    pub async fn resolve_tool<A: Action>(
        action: &A,
        #[builder(into)] tool: MiseTool,
        #[builder(into)] executable: Executable,
        context: &Context,
    ) -> Result<Tool, ResolveNodeToolError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(tool)
            .executable(executable)
            .context(context)
            .call()
            .await
            .map_err(|source| ResolveNodeToolError::UnresolvedTool { source })?;
        let node = Self::resolve()
            .action(action)
            .context(context)
            .call()
            .await
            .map_err(|source| ResolveNodeToolError::UnresolvedNode { source })?;

        tool.with_toolchain(&node)
            .map_err(|source| ResolveNodeToolError::UnresolvedNode { source })
    }

    /// Returns a program of a toolchain that the project pins, on the Node
    /// that the project pins
    ///
    /// Some toolchains of a project are packages for Node, such as
    /// `npm:typescript`. The lookup finds Node, as [`resolve`][resolve] does.
    /// Then it finds the program of the toolchain, as
    /// [`Tool::resolve_toolchain`] does. It takes the first version that the
    /// project pins and that the requirements of the action accept. A command of the program starts in
    /// the root of the context, with the directory of that Node first on its
    /// path.
    ///
    /// The lookup finds Node before the toolchain. So when both are missing,
    /// the error is about Node.
    ///
    /// # Errors
    ///
    /// Returns [`UnresolvedNode`][node] when the project pins no Node that the
    /// requirements of the action accept, or when mise did not install that
    /// Node. It also returns that error when the directory of that Node cannot
    /// be an entry of `PATH`. Returns [`UnresolvedToolchain`][toolchain] when
    /// the project pins no version of the toolchain that the requirements
    /// accept. It also returns that error when mise reports no such program of
    /// that version, for example because nothing installed it. Both errors also occur when mise
    /// does not run.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aonyx_rakko_node::Node;
    /// use rakko::{Action, Context};
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let tsc = Node::resolve_toolchain()
    ///     .action(action)
    ///     .toolchain("npm:typescript")
    ///     .executable("tsc")
    ///     .context(&context)
    ///     .call()
    ///     .await?;
    ///
    /// println!("{} {}", tsc.program(), tsc.version());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [node]: ResolveNodeToolchainError::UnresolvedNode
    /// [resolve]: Node::resolve
    /// [toolchain]: ResolveNodeToolchainError::UnresolvedToolchain
    #[builder]
    pub async fn resolve_toolchain<A: Action>(
        action: &A,
        #[builder(into)] toolchain: MiseTool,
        #[builder(into)] executable: Executable,
        context: &Context,
    ) -> Result<Tool, ResolveNodeToolchainError> {
        let node = Self::resolve()
            .action(action)
            .context(context)
            .call()
            .await
            .map_err(|source| ResolveNodeToolchainError::UnresolvedNode { source })?;
        let tool = Tool::resolve_toolchain()
            .action(action)
            .toolchain(toolchain)
            .executable(executable)
            .context(context)
            .call()
            .await
            .map_err(|source| ResolveNodeToolchainError::UnresolvedToolchain { source })?;

        tool.with_toolchain(&node)
            .map_err(|source| ResolveNodeToolchainError::UnresolvedNode { source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Node>();
    }
}
