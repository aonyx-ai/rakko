//! The cargo-udeps that a project runs
//!
//! This module holds the program of cargo-udeps that mise installed at the
//! version that the crate declares.

use getset::Getters;
use rakko::{Action, Context};
use rakko_tool::{ResolveToolError, Tool};

/// The name that mise knows the tool by
const CARGO_UDEPS: &str = "cargo:cargo-udeps";

/// The program of the tool
const PROGRAM: &str = "cargo-udeps";

/// The cargo-udeps that a project runs
///
/// The value holds the program that mise installed at the version that the
/// crate declares. The pin of the project does not change it, so a run
/// reaches the version that the tests of the crate cover. Nothing here
/// installs a tool: provisioning is the job of mise, and a cargo-udeps that
/// mise does not report stops the caller.
///
/// Cargo-udeps is a plugin of cargo, so a run starts the program with the
/// cargo that its builds use.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub(crate) struct Udeps {
    /// The program that mise installed at the declared version
    #[getset(get = "pub(crate)")]
    tool: Tool,
}

#[bon::bon]
impl Udeps {
    /// Returns the cargo-udeps that mise installed at the version that the
    /// action declares
    ///
    /// The action declares the pin of the mise file of the crate, so a pin of
    /// cargo-udeps in the project does not change the answer.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolError`] when the action does not declare
    /// cargo-udeps, or when mise does not run. It also returns one when mise
    /// reports no cargo-udeps at that version, for example because nothing
    /// installed it.
    #[builder]
    pub(crate) async fn resolve<A: Action>(
        action: &A,
        context: &Context,
    ) -> Result<Self, ResolveToolError> {
        let tool = Tool::resolve()
            .action(action)
            .tool(CARGO_UDEPS)
            .executable(PROGRAM)
            .context(context)
            .call()
            .await?;

        Ok(Self { tool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udeps_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Udeps>();
    }
}
