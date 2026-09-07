use std::path::PathBuf;

use rakko_cargo::{DiscoverRootsError, ResolveToolchainError};
use rakko_tool::{ResolveToolError, RunCommandError};
use thiserror::Error;

/// An error that stops a run of the action before it has an answer
///
/// Nothing here is a problem of the project. A file that is not formatted
/// and a file that rustfmt cannot parse travel as findings in the outcome
/// of the run. The variants of this error describe a run whose answer cannot
/// be trusted, and such a run stops instead of reporting one.
#[derive(Debug, Error)]
pub enum FormatRustError {
    /// Cargo did not run
    ///
    /// The program was resolved and did not start, or it started and its
    /// output could not be read. Nothing of the project was examined.
    #[error("failed to run cargo")]
    CargoUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Rustfmt reported a path that the project root does not contain
    ///
    /// A finding names its path relative to the project root, and a path
    /// outside the root has no such name. Rustfmt formats the files of the
    /// project, so this points at a report that the action misread.
    #[error("rustfmt reported a path outside the project: {}", path.display())]
    ForeignPath {
        /// The path that rustfmt reported
        path: PathBuf,
    },

    /// Rustfmt does not honor the configuration of the project
    ///
    /// Rustfmt warns about an option that it does not know, and about an
    /// option that its channel does not support, and then formats without
    /// the option. A run without the option quietly does what the project
    /// asked it not to do, so the action stops instead of reporting what
    /// such a run found.
    #[error(
        "rustfmt does not honor the configuration and would have formatted without it: {details}"
    )]
    RejectedConfiguration {
        /// What rustfmt reported about the configuration
        details: String,
    },

    /// The workspaces of the project could not be discovered
    ///
    /// A run formats every workspace of the project, and a run that does not
    /// know them all would hide every problem of the missing one behind a
    /// green result.
    #[error("failed to discover the workspaces of the project")]
    UndiscoveredRoots {
        /// The cause of the failure
        source: DiscoverRootsError,
    },

    /// Rustfmt wrote a report that the action does not recognize
    ///
    /// The shape of the report belongs to a version of rustfmt. A run that
    /// ended without success and named no problem wrote something the
    /// action could not read, and an answer built on such a report would
    /// hide problems behind a green result.
    #[error("rustfmt wrote a report in {} that the action does not recognize: {stderr}", root.display())]
    UnrecognizedReport {
        /// The workspace root that rustfmt worked on
        root: PathBuf,

        /// What rustfmt wrote to its standard error stream
        stderr: String,
    },

    /// Mise reported no cargo for the project
    ///
    /// The project pins no Rust toolchain, or nothing installed the pin yet.
    /// The action installs nothing, so the run stops here.
    #[error("failed to resolve cargo")]
    UnresolvedTool {
        /// The cause of the failure
        source: ResolveToolError,
    },

    /// Mise reported no nightly toolchain for the project
    ///
    /// Rustfmt honors the unstable options of a configuration only on the
    /// nightly channel, so the action runs on the nightly toolchain that the
    /// project pins. The project pins none, or nothing installed the pin
    /// yet, and the action installs nothing.
    #[error("failed to resolve the nightly toolchain")]
    UnresolvedToolchain {
        /// The cause of the failure
        source: ResolveToolchainError,
    },
}
