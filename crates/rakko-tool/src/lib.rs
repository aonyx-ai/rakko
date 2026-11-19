//! The external tools that an action runs
//!
//! An action does the work of an external tool, and it starts that tool as a
//! subprocess. This crate finds the tool and describes the command that runs
//! it, so that every action reaches its tool in the same way.
//!
//! [mise] answers where a tool is.
//! The crate searches no path of its own and installs nothing.
//! A tool that mise did not install stops the action with an error that
//! gives the name of the tool.
//!
//! An action declares the tools that it runs. A crate that wraps a tool pins
//! the exact version of the tool in a mise file. The action returns the pins
//! of that file, or of the file of a helper crate, from the methods of
//! [`Action`][action] that declare its tools. [`Tool::resolve`] finds a
//! program of a declared tool at the version of its pin. The pin of the project does
//! not change it, so a run starts the version that the tests of the action
//! cover. The lookup takes the declarations of the action, and it refuses a
//! tool that they do not hold. So a project can install every tool that a run
//! needs before the run starts.
//!
//! A toolchain, such as Rust or Node, belongs to the project, and the project
//! pins it. [`Tool::resolve_toolchain`] finds a program of the toolchain at the
//! first version that the project pins and that every requirement of the
//! action accepts, from [`Action::toolchains`][toolchains]. An action without
//! a requirement runs on the first pin. So the action writes no check of its
//! own, and an unmet requirement stops only that action. A tool that starts
//! its toolchain by name, such as a package for Node, runs on the toolchain
//! that [`Tool::with_toolchain`] gives it.
//!
//! The crate describes a command and starts nothing until the caller asks for
//! a run. An action can therefore write the command to a log, or name it in
//! an error, before anything runs.
//!
//! An action runs the command with [`launch_with`][launch-with] and the
//! [`ToolOutput`][output] of its context. The run sends the start of the tool,
//! each line that it writes, and its end to that tool output. So the harness
//! can show the tool to a reader while it runs. The action still receives the
//! full capture of the tool. A tool that writes a machine report, such as
//! JSON, to one stream shows only the other one, as [`ShownStreams`] says. [`launch`][launch] runs the command without a
//! tool output. On Windows, a tool that a package manager for Node installed
//! starts through a launcher. Both runs start that launcher again when the
//! operating system refused it a file for a moment.
//!
//! # Asynchronous Runtime
//!
//! A run starts a program and waits for it, and a [Tokio] runtime drives that
//! work. Both the search for a tool and the run of a command need one, and
//! they panic without it.
//!
//! # Examples
//!
//! ```no_run
//! use rakko::{Action, Context};
//! use rakko_tool::{Launch, ShownStreams, Tool};
//!
//! # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
//! let context = Context::builder().root("/home/otter/project").build();
//!
//! let tool = Tool::resolve()
//!     .action(action)
//!     .tool("npm:prettier")
//!     .executable("prettier")
//!     .context(&context)
//!     .call()
//!     .await?;
//!
//! let execution = tool
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
//! [action]: rakko::Action
//! [launch]: Launch::launch
//! [launch-with]: Launch::launch_with
//! [output]: rakko::ToolOutput
//! [mise]: https://mise.jdx.dev
//! [toolchains]: rakko::Action::toolchains
//! [tokio]: https://tokio.rs

mod launch;
mod seal;
mod tool;

pub use kawauso_process::error::RunCommandError;
pub use kawauso_process::invocation::Program;
pub use kawauso_process::{Execution, Invocation};

pub use self::launch::{Launch, ShownStreams};
pub use self::tool::{ResolveToolError, ResolveToolchainError, Tool};
