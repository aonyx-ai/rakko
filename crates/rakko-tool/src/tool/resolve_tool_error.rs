//! What the crate reports when it cannot resolve a declared tool

use kawauso_process::error::RunCommandError;
use rakko::{Executable, MiseTool, ToolVersion};
use thiserror::Error;

/// An error that occurs when the crate resolves a tool that an action
/// declares
///
/// The variants separate what a user can do about the failure. An action
/// that asks for a tool that it does not declare is a mistake of the action,
/// and its author repairs it. Mise that does not run is a broken
/// environment, and every tool of the project fails with it. An install
/// repairs a version that mise did not install. A program that the installed
/// tool does not provide is again a mistake of the action.
///
/// No variant means that the tool ran. Nothing started, so there is no exit
/// status and no output to read.
///
/// A later release can add a variant for another reason why a tool cannot be
/// resolved. So a match on this error outside this crate needs an arm for a
/// variant that it does not know.
///
/// # Examples
///
/// ```
/// use rakko::{Executable, MiseTool, ToolVersion};
/// use rakko_tool::ResolveToolError;
///
/// let error = ResolveToolError::MissingExecutable {
///     tool: MiseTool::new("npm:renovate"),
///     version: ToolVersion::new("44.240.0"),
///     executable: Executable::new("renovate-validator"),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "the tool `npm:renovate` 44.240.0 provides no program `renovate-validator`"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveToolError {
    /// The action declares the tool at more than one version
    ///
    /// Two sources of the tools of the action pin the same tool at different
    /// versions. The sources can be the mise file of the action and that of a
    /// helper crate. The lookup cannot tell which version the run means, so it
    /// starts neither. One pin of the tool repairs the failure. Only a run
    /// finds the failure, because `doctor` does not compare the versions that
    /// one action declares.
    #[error("the action declares `{tool}` at more than one version")]
    AmbiguousVersion {
        /// The tool that the action asked for
        tool: MiseTool,

        /// The versions that the action declares for the tool
        versions: Vec<ToolVersion>,
    },

    /// Mise did not run
    ///
    /// No program answers to the name `mise`, or the operating system refused
    /// to start it. The canonical way to start a harness enters the
    /// environment of mise first. A run that reports this failure therefore ran
    /// outside that environment. Every tool of the project is out of reach.
    #[error("failed to ask mise about `{tool}`")]
    MiseUnavailable {
        /// The tool that the action asked for
        tool: MiseTool,

        /// The cause of the failure
        source: RunCommandError,
    },

    /// The installed tool provides no program by that name
    ///
    /// Mise installed the tool at the declared version, and the tool has no
    /// program of the name that the action gave. A tool can provide several
    /// programs, such as `renovate-config-validator` of the tool
    /// `npm:renovate`. The program that the action starts repairs the
    /// failure.
    #[error("the tool `{tool}` {version} provides no program `{executable}`")]
    MissingExecutable {
        /// The tool that the action asked for
        tool: MiseTool,

        /// The version of the tool that the action declares
        version: ToolVersion,

        /// The program that the action asked for
        executable: Executable,
    },

    /// The action does not declare the tool
    ///
    /// A run asks only for the tools that the action declares. So a project
    /// can install every tool that a run needs before the run starts.
    /// The tool is in no mise file that the action returns. A toolchain of
    /// the project is no declared tool either, and
    /// [`Tool::resolve_toolchain`][toolchain] resolves it. A pin of the tool
    /// in the mise file of the crate, or the tools of the helper crate that
    /// pins it, repairs the failure.
    ///
    /// [toolchain]: crate::Tool::resolve_toolchain
    #[error("the action declares no tool `{tool}`")]
    UndeclaredTool {
        /// The tool that the action asked for
        tool: MiseTool,
    },

    /// Mise did not install the tool at the declared version
    ///
    /// Rakko installs nothing, so the run stops here. The details carry what
    /// mise wrote, and mise gives the install that repairs the failure.
    #[error("mise reports no install of `{tool}` {version}: {details}")]
    UninstalledVersion {
        /// The tool that the action asked for
        tool: MiseTool,

        /// The version of the tool that the action declares
        version: ToolVersion,

        /// What mise said about the tool
        details: String,
    },
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn missing_executable_gives_the_tool_the_version_and_the_program() {
        let error = ResolveToolError::MissingExecutable {
            tool: MiseTool::new("shfmt"),
            version: ToolVersion::new("3.14.1"),
            executable: Executable::new("shformat"),
        };

        assert_eq!(
            error.to_string(),
            "the tool `shfmt` 3.14.1 provides no program `shformat`"
        );
    }

    #[test]
    fn resolve_tool_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ResolveToolError>();
    }

    #[test]
    fn undeclared_tool_gives_the_tool() {
        let error = ResolveToolError::UndeclaredTool {
            tool: MiseTool::new("taplo"),
        };

        assert_eq!(error.to_string(), "the action declares no tool `taplo`");
    }
}
