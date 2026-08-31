use kawauso_process::error::RunCommandError;
use thiserror::Error;

use crate::tool::ToolName;

/// An error that occurs when the crate looks for an external tool
///
/// The variants separate what an action can do about the failure. Mise that
/// does not run is a broken environment, and every tool of the project fails
/// with it. A tool that mise does not report is a gap in the pins of one
/// project, and the message carries what mise said, because mise names the
/// step that closes the gap.
///
/// Neither variant means that the tool ran. Nothing started, so there is no
/// exit status and no output to read.
#[derive(Debug, Error)]
pub enum ResolveToolError {
    /// Mise did not run
    ///
    /// No program answers to the name `mise`, or the operating system refused
    /// to start it. The canonical way to start a harness enters the
    /// environment of mise first, so a run that reports this failure ran
    /// outside that environment, and every tool of the project is out of
    /// reach.
    #[error("failed to ask mise where `{tool}` is")]
    MiseUnavailable {
        /// The tool that the crate looked for
        tool: ToolName,

        /// The cause of the failure
        source: RunCommandError,
    },

    /// Mise reports no location for the tool
    ///
    /// The project pins no such tool, or it pins the tool and nothing
    /// installed it yet. Rakko installs nothing, so the run stops here, and
    /// the details carry what mise wrote about the tool.
    #[error("mise reports no location for `{tool}`: {details}")]
    UnresolvedTool {
        /// The tool that the crate looked for
        tool: ToolName,

        /// What mise said about the tool
        details: String,
    },
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    // An action puts the error in the outcome of a run, and that outcome
    // holds an error that another thread can read. This test holds the error
    // to the auto traits that make this possible, because a field of a later
    // version could take them away without a word from the compiler.
    #[test]
    fn resolve_tool_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ResolveToolError>();
    }
}
