//! What the crate reports when it cannot resolve the requirement of a tool

use kawauso_process::error::RunCommandError;
use kawauso_process::invocation::Program;
use rakko::{Executable, MiseTool, ToolVersion, VersionRange};
use thiserror::Error;

/// An error that occurs when the crate resolves the requirement of a tool
///
/// The variants separate what a user can do about the failure. Mise that
/// does not run is a broken environment, and every tool of the project fails
/// with it. An install of the missing version repairs a version of a tool
/// that mise did not install. A toolchain that the project pins outside the
/// range of the action is a decision of the project. Only a new pin of the
/// project repairs it.
///
/// No variant means that the tool ran. Nothing started, so there is no exit
/// status and no output to read.
///
/// A later release can add a variant for another reason why a requirement
/// cannot be resolved. So a match on this error outside this crate needs an
/// arm for a variant that it does not know.
///
/// # Examples
///
/// A toolchain that the project does not pin leaves the sentence of mise in
/// the error:
///
/// ```
/// use rakko::MiseTool;
/// use rakko_tool::ResolveRequirementError;
///
/// let error = ResolveRequirementError::UnresolvedToolchain {
///     tool: MiseTool::new("rust"),
///     details: "no version is set".to_owned(),
/// };
///
/// assert_eq!(
///     error.to_string(),
///     "mise reports no version of `rust` that the project pins: no version is set"
/// );
/// ```
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ResolveRequirementError {
    /// The range of versions that the action accepts is not a range
    ///
    /// The range uses the syntax of a version requirement of Cargo. A text in
    /// another syntax is a mistake of the action, and no change of the
    /// project repairs it.
    #[error("failed to read the range `{range}` that the action accepts for `{tool}`")]
    InvalidRange {
        /// The tool that the requirement is for
        tool: MiseTool,

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
    #[error("failed to ask mise about `{tool}`")]
    MiseUnavailable {
        /// The tool that the requirement is for
        tool: MiseTool,

        /// The cause of the failure
        source: RunCommandError,
    },

    /// The requirement asks for a kind of version that this release does not
    /// know
    ///
    /// A later release of the contract can add a kind of version to a
    /// requirement. This release of the crate cannot resolve that kind. So
    /// the run stops, and it does not start a version that the action can
    /// refuse.
    #[error(
        "the requirement of `{tool}` asks for a kind of version that this release of \
         rakko-tool does not know, so update rakko-tool"
    )]
    UnknownRequirement {
        /// The tool that the requirement is for
        tool: MiseTool,
    },

    /// The directory of a toolchain cannot be an entry of `PATH`
    ///
    /// A tool that runs on a toolchain finds the program of the toolchain
    /// through the variable `PATH`. An entry of `PATH` cannot hold the
    /// separator of the entries, such as `:` on Unix. A program without a
    /// directory gives no entry either. Without the entry, the tool runs on a
    /// toolchain that the environment chose, which can be a version outside
    /// the range of the action. So the run stops. A location of the installs
    /// of mise without that character repairs the failure.
    #[error(
        "the toolchain `{}` has no directory that can be an entry of `PATH`",
        program.get().display()
    )]
    UnplaceableToolchain {
        /// The program of the toolchain that mise reported
        program: Program,
    },

    /// Mise reports no version of the toolchain for the project
    ///
    /// No configuration inside the project pins the toolchain, or mise did
    /// not read what the project pins, for example because nobody trusts its
    /// configuration. A pin in the global configuration of the user does not
    /// count. A pin in a directory above the project does not count either. A
    /// toolchain is a decision of the project, so a pin in the configuration
    /// of the project repairs the failure.
    #[error("mise reports no version of `{tool}` that the project pins: {details}")]
    UnresolvedToolchain {
        /// The toolchain that the requirement is for
        tool: MiseTool,

        /// What mise said about the toolchain
        details: String,
    },

    /// The version of the toolchain that the project pins cannot be compared
    /// with a range
    ///
    /// A range compares versions with three numbers, such as `22.4.0`. Mise
    /// gives the version that a pin resolves to. A pin that holds no such
    /// version, such as a pin of a branch or of the Rust channel `stable`,
    /// has nothing to compare. An added pin with a version number repairs the
    /// failure.
    #[error(
        "failed to read the version `{version}` of `{tool}` that the project pins, so add a \
         pin with a version number, such as `1.99.1`"
    )]
    UnreadableVersion {
        /// The toolchain that the requirement is for
        tool: MiseTool,

        /// The version that mise gave for the pin of the project
        version: ToolVersion,

        /// The cause of the failure
        source: semver::Error,
    },

    /// Mise listed the pins of a toolchain in a form that the crate does not
    /// read
    ///
    /// The crate reads the JSON list of mise. A release of mise that changes
    /// that list breaks the reading, and the run stops instead of guessing
    /// which version the project pins.
    #[error("failed to read the versions of `{tool}` that mise lists for the project")]
    UnrecognizedReport {
        /// The toolchain that the requirement is for
        tool: MiseTool,

        /// The cause of the failure
        source: serde_json::Error,
    },

    /// Mise reports no program of the tool at the version
    ///
    /// Nothing installed that version of the tool, or the tool provides no
    /// program by that name. Rakko installs nothing, so the run stops here.
    /// The details carry what mise wrote, and mise gives the install that
    /// repairs the failure.
    #[error("mise reports no `{executable}` of `{tool}` {version}: {details}")]
    UnresolvedVersion {
        /// The tool that the requirement is for
        tool: MiseTool,

        /// The program of the tool that the action starts
        executable: Executable,

        /// The version of the tool that the resolution asked for
        version: ToolVersion,

        /// What mise said about the tool
        details: String,
    },

    /// The project pins a version of the toolchain that the action does not
    /// accept
    ///
    /// The toolchain is a decision of the project, so Rakko does not choose
    /// another version. A pin of the project in the range repairs the
    /// failure.
    #[error(
        "the project pins `{tool}` {version}, outside the range `{range}` that the action accepts"
    )]
    VersionOutOfRange {
        /// The toolchain that the requirement is for
        tool: MiseTool,

        /// The version that the project pins
        version: ToolVersion,

        /// The range of versions that the action accepts
        range: VersionRange,
    },
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Shows that the requirement error is Send and Sync
    ///
    /// An action puts the error in the outcome of a run, and that outcome
    /// holds an error that another thread can read. This test holds the error
    /// to the auto traits that make this possible. It does so because a field
    /// of a later version could take them away without a word from the
    /// compiler.
    #[test]
    fn resolve_requirement_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ResolveRequirementError>();
    }

    #[test]
    fn resolve_requirement_error_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ResolveRequirementError>();
    }
}
