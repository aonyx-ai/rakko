use thiserror::Error;

use crate::part::{Platform, ToolName, Version};

/// An error that keeps the action from reading a lockfile of mise
///
/// The action reads the entries of a lockfile to compare them, and a lockfile
/// that it reads only in part would hide gaps behind a green result. Each
/// variant names what in the lockfile has a shape that the action does not
/// know.
#[derive(Debug, Error)]
pub enum ReadLockfileError {
    /// An entry of a tool records a platform that is not a table
    #[error(
        "the entry of {tool} {version} records the platform {platform} in a shape that is not a table"
    )]
    MisshapenPlatform {
        /// The tool of the entry
        tool: ToolName,

        /// The version of the entry
        version: Version,

        /// The platform whose record is not a table
        platform: Platform,
    },

    /// The entries of a tool are not a list of tables
    #[error("the entries of {tool} are not a list of tables")]
    MisshapenTool {
        /// The tool whose entries have another shape
        tool: ToolName,
    },

    /// The tools of the lockfile are not a table
    #[error("the tools of the lockfile are not a table")]
    MisshapenTools,

    /// An entry of a tool names no version
    #[error("an entry of {tool} names no version")]
    MissingVersion {
        /// The tool of the entry
        tool: ToolName,
    },

    /// The lockfile is not TOML
    #[error("failed to parse the lockfile as TOML")]
    UnparsableToml {
        /// The cause of the failure
        source: toml_edit::de::Error,
    },
}
