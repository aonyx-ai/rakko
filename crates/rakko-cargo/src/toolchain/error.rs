use rakko_tool::RunCommandError;
use thiserror::Error;

use crate::toolchain::{Channel, Toolchain};

/// An error that occurs when the crate looks for a Rust toolchain
///
/// The variants separate what an action can do about the failure. Mise that
/// does not run is a broken environment. A channel that the project does not
/// pin, and a pin that nothing installed, are gaps in the provisioning of
/// one project, and the message names the step that closes each of them. A
/// report that the crate cannot read points at a mise that changed its
/// output.
///
/// No variant means that cargo ran. Nothing of the project was examined.
#[derive(Debug, Error)]
pub enum ResolveToolchainError {
    /// Mise did not run
    ///
    /// No program answers to the name `mise`, or the operating system refused
    /// to start it. The canonical way to start a harness enters the
    /// environment of mise first, so a run that reports this failure ran
    /// outside that environment.
    #[error("failed to ask mise which `{channel}` toolchain the project pins")]
    MiseUnavailable {
        /// The channel that the crate looked for
        channel: Channel,

        /// The cause of the failure
        source: RunCommandError,
    },

    /// Mise pins the toolchain, and nothing installed it
    ///
    /// The project names the channel in its `mise.toml`, and mise resolved
    /// it to a toolchain, but that toolchain is absent from the machine.
    /// Rakko installs nothing, so the run stops here.
    #[error("mise pins the `{channel}` toolchain as `{toolchain}`, and nothing installed it")]
    UninstalledToolchain {
        /// The channel that the crate looked for
        channel: Channel,

        /// The toolchain that mise resolved the channel to
        toolchain: Toolchain,
    },

    /// The project pins no toolchain of the channel
    ///
    /// Mise listed the Rust toolchains of the project, and none of them
    /// belongs to the channel. The pin is missing from `mise.toml`.
    #[error("the project pins no `{channel}` toolchain")]
    UnpinnedToolchain {
        /// The channel that the crate looked for
        channel: Channel,
    },

    /// Mise wrote a report that the crate does not recognize
    ///
    /// Mise ended without success, or it answered in a shape that the crate
    /// cannot read. The details carry what mise wrote, or what could not be
    /// read from it.
    #[error("mise wrote a report about the Rust toolchains that the crate cannot read: {details}")]
    UnreadableReport {
        /// The channel that the crate looked for
        channel: Channel,

        /// What mise wrote, or what could not be read
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
    fn resolve_toolchain_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ResolveToolchainError>();
    }
}
