//! What the crate reports when it cannot resolve a toolchain of the project

use kawauso_process::error::RunCommandError;
use kawauso_process::invocation::Program;
use rakko::{Executable, MiseTool, ToolVersion, VersionRange};
use thiserror::Error;

/// An error that occurs when the crate resolves a toolchain that the project
/// pins
///
/// The variants separate what a user can do about the failure. A toolchain is
/// a decision of the project. So a change of the pins of the project repairs
/// a pin that is missing, or a pin that a requirement does not accept. An
/// install repairs a pin that mise did not install, and the pins stay as they
/// are. Mise that does not run is a broken environment, and every tool of
/// the project fails with it. A requirement that is not a range is a mistake
/// of the action.
///
/// No variant means that the toolchain ran. Nothing started, so there is no
/// exit status and no output to read.
///
/// A later release can add a variant for another reason why a toolchain
/// cannot be resolved. So a match on this error outside this crate needs an
/// arm for a variant that it does not know.
///
/// # Examples
///
/// ```
/// use rakko::{MiseTool, ToolVersion, VersionRange};
/// use rakko_tool::ResolveToolchainError;
///
/// let error = ResolveToolchainError::UnmetRequirement {
///     toolchain: MiseTool::new("node"),
///     version: ToolVersion::new("20.19.0"),
///     range: VersionRange::new("^22.18 || >=23.6"),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "the project pins `node` 20.19.0, which the range `^22.18 || >=23.6` does not \
///      accept, so change the pin of `node` to a version that it accepts"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveToolchainError {
    /// A requirement of the action gives a range that is not a range
    ///
    /// The range uses the syntax of a version requirement of Cargo, with
    /// alternatives joined by `||`. A text in another syntax is a mistake of
    /// the action, and no change of the project repairs it.
    #[error("failed to read the range `{range}` that the action requires of `{toolchain}`")]
    InvalidRange {
        /// The toolchain that the requirement is for
        toolchain: MiseTool,

        /// The text that the action gave as the range
        range: VersionRange,

        /// The cause of the failure
        source: semver::Error,
    },

    /// Mise did not run
    ///
    /// No program answers to the name `mise`, or the operating system refused
    /// to start it. The canonical way to start a harness enters the
    /// environment of mise first. A run that reports this failure therefore ran
    /// outside that environment. Every tool of the project is out of reach.
    #[error("failed to ask mise about `{toolchain}`")]
    MiseUnavailable {
        /// The toolchain that the action asked for
        toolchain: MiseTool,

        /// The cause of the failure
        source: RunCommandError,
    },

    /// Mise installed the toolchain, and the toolchain provides no program by
    /// that name
    ///
    /// A toolchain can provide several programs, such as `cargo` and `rustc`
    /// of `rust`. The program that the action starts repairs the failure.
    #[error("the toolchain `{toolchain}` {version} provides no program `{executable}`")]
    MissingExecutable {
        /// The toolchain that the action asked for
        toolchain: MiseTool,

        /// The version of the toolchain that the project pins
        version: ToolVersion,

        /// The program that the action asked for
        executable: Executable,
    },

    /// Mise did not install the version of the toolchain that the project
    /// pins
    ///
    /// The pin is right, so it does not change. Rakko installs nothing, so
    /// the run stops here. An install of the pins of the project, such as
    /// `mise install`, repairs the failure. The details carry what mise wrote.
    #[error(
        "mise did not install `{toolchain}` {version}, which the project pins, so install the \
         pins of the project, for example with `mise install`: {details}"
    )]
    UninstalledToolchain {
        /// The toolchain that the action asked for
        toolchain: MiseTool,

        /// The version that the project pins
        version: ToolVersion,

        /// What mise said about the toolchain
        details: String,
    },

    /// The project pins versions of the toolchain, and a requirement of the
    /// action accepts none of them
    ///
    /// The toolchain is a decision of the project, so Rakko does not choose
    /// another version. A pin of the project that every requirement accepts
    /// repairs the failure. The error gives the first pin that a requirement
    /// refused, and the range of that requirement. The range can also come
    /// from the run, such as the `rust-version` of a manifest. Then a change
    /// of that source can repair the failure too.
    #[error(
        "the project pins `{toolchain}` {version}, which the range `{range}` does not accept, so \
         change the pin of `{toolchain}` to a version that it accepts"
    )]
    UnmetRequirement {
        /// The toolchain that the requirement is for
        toolchain: MiseTool,

        /// The version that the project pins
        version: ToolVersion,

        /// The range of versions that the requirement accepts
        range: VersionRange,
    },

    /// The directory of the program of the toolchain cannot be an entry of
    /// `PATH`
    ///
    /// A tool that starts its toolchain by name, such as a package for Node,
    /// finds the program of the toolchain through the variable `PATH`. An
    /// entry of `PATH` cannot hold the separator of the entries, such as `:`
    /// on Unix. A program without a directory gives no entry either. Without
    /// the entry, the tool runs on a toolchain that the environment chose
    /// instead of the one that the project pins. So the run stops. A location
    /// of the installs of mise without that character repairs the failure.
    #[error(
        "the toolchain `{}` has no directory that can be an entry of `PATH`",
        program.get().display()
    )]
    UnplaceableToolchain {
        /// The program of the toolchain that mise reported
        program: Program,
    },

    /// Mise reports no version of the toolchain that the project pins
    ///
    /// No configuration inside the project pins the toolchain, or mise did not
    /// read what the project pins, for example because nobody trusts its
    /// configuration. A pin in the global configuration of the user does not
    /// count, and neither does a pin in a directory above the project. For a
    /// requirement of a dated channel, such as `>=nightly-2025-02-20`, only a
    /// pin of that channel with a date counts. A pin in the configuration of
    /// the project repairs the failure.
    #[error(
        "mise reports no version of `{toolchain}` that the project pins and that the action \
         accepts, so pin such a version in the configuration of the project: {details}"
    )]
    UnpinnedToolchain {
        /// The toolchain that the action asked for
        toolchain: MiseTool,

        /// What mise said about the toolchain, or what the requirement needs
        details: String,
    },

    /// The version of the toolchain that the project pins cannot be compared
    /// with a requirement
    ///
    /// A range compares versions with three numbers, such as `22.4.0`. A pin
    /// that resolves to no such version, such as the Rust channel `stable`,
    /// has nothing to compare. A pin with a version number repairs the
    /// failure.
    #[error(
        "failed to read the version `{version}` of `{toolchain}` that the project pins, so pin \
         a version with a number, such as `1.99.1`"
    )]
    UnreadableVersion {
        /// The toolchain that the requirement is for
        toolchain: MiseTool,

        /// The version that mise gave for the pin of the project
        version: ToolVersion,

        /// The cause of the failure
        source: semver::Error,
    },

    /// Mise listed the pins of the toolchain in a form that the crate does
    /// not read
    ///
    /// The crate reads the JSON list of mise. A release of mise that changes
    /// that list breaks the reading, and the run stops instead of guessing
    /// which version the project pins.
    #[error("failed to read the versions of `{toolchain}` that mise lists for the project")]
    UnrecognizedReport {
        /// The toolchain that the action asked for
        toolchain: MiseTool,

        /// The cause of the failure
        source: serde_json::Error,
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
    fn resolve_toolchain_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ResolveToolchainError>();
    }

    #[test]
    fn uninstalled_toolchain_says_to_install_the_pins() {
        let error = ResolveToolchainError::UninstalledToolchain {
            toolchain: MiseTool::new("node"),
            version: ToolVersion::new("22.20.0"),
            details: "node@22.20.0 is not installed".to_owned(),
        };

        assert!(error.to_string().contains("mise install"), "got {error}");
    }

    #[test]
    fn unpinned_toolchain_says_to_pin_a_version() {
        let error = ResolveToolchainError::UnpinnedToolchain {
            toolchain: MiseTool::new("node"),
            details: "no configuration inside the project pins a version".to_owned(),
        };

        assert!(
            error.to_string().contains("so pin such a version"),
            "got {error}"
        );
    }
}
