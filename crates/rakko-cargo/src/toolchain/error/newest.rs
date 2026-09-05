use rakko_tool::RunCommandError;
use thiserror::Error;

use crate::toolchain::Toolchain;

/// An error that occurs when the crate looks for the toolchain that a project
/// builds with
///
/// The variants separate what an action can do about the failure. Mise that
/// does not run is a broken environment. A project that pins every toolchain
/// through a channel, and a pin that nothing installed, are gaps in the
/// provisioning of one project, and the message names the step that closes
/// each of them. A report that the crate cannot read points at a mise that
/// changed its output.
///
/// No variant carries a channel, because the question names none: it asks
/// for the toolchain of the project and not for the toolchain of a channel.
///
/// No variant means that cargo ran. Nothing of the project was examined.
#[derive(Debug, Error)]
pub enum ResolveNewestToolchainError {
    /// Mise did not run
    ///
    /// No program answers to the name `mise`, or the operating system refused
    /// to start it. The canonical way to start a harness enters the
    /// environment of mise first, so a run that reports this failure ran
    /// outside that environment.
    #[error("failed to ask mise which Rust toolchains the project pins")]
    MiseUnavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Mise pins the toolchain, and nothing installed it
    ///
    /// The project pins the toolchain by its version, and that toolchain is
    /// absent from the machine. Rakko installs nothing, so the run stops
    /// here.
    #[error("the project pins `{toolchain}`, and nothing installed it")]
    UninstalledToolchain {
        /// The toolchain that the project pins
        toolchain: Toolchain,
    },

    /// The project pins no toolchain by its own version
    ///
    /// Every Rust pin of the project names a channel that mise resolves to
    /// another toolchain, such as `nightly`. A channel says which toolchain
    /// a job of its own needs, and it does not say which toolchain the
    /// project builds with, so the question has no answer.
    #[error("the project pins no Rust toolchain by an exact version")]
    UnpinnedToolchain,

    /// Mise wrote a report that the crate does not recognize
    ///
    /// Mise ended without success, or it answered in a shape that the crate
    /// cannot read. The details carry what mise wrote, or what could not be
    /// read from it.
    #[error("mise wrote a report about the Rust toolchains that the crate cannot read: {details}")]
    UnreadableReport {
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
    fn resolve_newest_toolchain_error_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ResolveNewestToolchainError>();
    }
}
