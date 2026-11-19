//! The machinery that the actions which run on Node share
//!
//! Many tools of project maintenance are packages for [Node], such as
//! prettier, markdownlint, and the compiler of TypeScript. Such a tool starts
//! the program `node` by its name, so it runs on the first Node that it finds
//! on the path. That Node can be another release than the one that the
//! project pins. [Mise] also puts no Node on the path of a tool that it
//! installed from npm.
//!
//! Node is a toolchain of the project, so the project pins it, and an action
//! does not pin it. This crate finds the Node that the project pins. It finds
//! the program that runs on that Node, and it puts the directory of that Node
//! first on the path of the program. So the program runs on the pinned Node.
//! An action then gives only what is special about its tool.
//!
//! [`Node`] holds three lookups. The first finds Node itself. The second finds
//! a tool that the action declares. The third finds a toolchain of the project
//! that runs on Node, such as TypeScript. Each lookup returns a
//! [`Tool`][tool].
//!
//! The lookup of Node fails with the
//! [`ResolveToolchainError`][toolchain-error] of the lookup of any toolchain.
//! The lookup of a tool fails with a [`ResolveNodeToolError`], and the lookup
//! of a toolchain fails with a [`ResolveNodeToolchainError`]. Each of these two
//! errors tells Node from the program that runs on it. An action wraps the
//! error of its lookup in one variant of its own error.
//!
//! The crate declares no tools of its own and requires no range of Node. An
//! action that needs certain releases of Node declares a
//! [`ToolchainRequirement`][requirement] from
//! [`Action::toolchains`][toolchains], and each lookup applies it.
//!
//! # Asynchronous Runtime
//!
//! A lookup asks mise and waits for it. A [Tokio] runtime drives that, and a
//! lookup panics without one.
//!
//! # Examples
//!
//! An action resolves prettier on the Node of the project and runs it:
//!
//! ```no_run
//! use aonyx_rakko_node::Node;
//! use rakko::{Action, Context};
//! use rakko_tool::{Launch, ShownStreams};
//!
//! # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
//! let context = Context::builder().root("/home/otter/project").build();
//!
//! let prettier = Node::resolve_tool()
//!     .action(action)
//!     .tool("npm:prettier")
//!     .executable("prettier")
//!     .context(&context)
//!     .call()
//!     .await?;
//!
//! let execution = prettier
//!     .invocation()
//!     .arg("--check")
//!     .arg(".")
//!     .launch_with(context.tool_output(), ShownStreams::Both)
//!     .await?;
//!
//! println!("{}", execution.status());
//! # Ok(())
//! # }
//! ```
//!
//! [mise]: https://mise.jdx.dev
//! [node]: https://nodejs.org
//! [requirement]: rakko::ToolchainRequirement
//! [tokio]: https://tokio.rs
//! [tool]: rakko_tool::Tool
//! [toolchain-error]: rakko_tool::ResolveToolchainError
//! [toolchains]: rakko::Action::toolchains

mod node;

pub use self::node::{Node, ResolveNodeToolError, ResolveNodeToolchainError};
