//! What the crate reports when it cannot name a toolchain
//!
//! A caller asks for a toolchain in one of two ways: by the channel that the
//! project pins for a job, or by asking for the toolchain that the project
//! builds with. The two questions can fail for different reasons, and each
//! of them carries the context that its own question knows, so this module
//! holds one error per question.

/// The error of the question for the toolchain that a project builds with
mod newest;
/// The error of the question for the toolchain of a channel
mod resolve;

pub use self::newest::ResolveNewestToolchainError;
pub use self::resolve::ResolveToolchainError;
