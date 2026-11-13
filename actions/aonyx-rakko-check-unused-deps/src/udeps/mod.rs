//! The cargo-udeps that a project runs
//!
//! This module holds the version of cargo-udeps that the crate declares and
//! the program that mise installed at that version.

use getset::Getters;
use rakko::{Context, RequiredVersion, ToolRequirement, ToolVersion};
use rakko_tool::{MiseFile, ResolveRequirementError, Tool};

/// The name that mise knows the tool by
const CARGO_UDEPS: &str = "cargo:cargo-udeps";

/// The program of the tool
const PROGRAM: &str = "cargo-udeps";

/// The version of cargo-udeps that the crate runs
///
/// The version comes from the mise file of the crate, which the build
/// embeds, so a file without the pin fails the build.
const VERSION: &str =
    MiseFile::new(include_str!("../../mise.rakko.toml")).pinned_version(CARGO_UDEPS);

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

impl Udeps {
    /// Returns the requirement of the cargo-udeps that the crate runs
    ///
    /// The requirement holds the exact version that the crate pins.
    pub(crate) fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool(CARGO_UDEPS)
            .executable(PROGRAM)
            .version(RequiredVersion::Exact(ToolVersion::new(VERSION)))
            .build()
    }

    /// Returns the cargo-udeps that mise installed at the version of the
    /// crate
    ///
    /// The lookup asks mise for the version that [`requirement`][requirement]
    /// holds, so a pin of cargo-udeps in the project does not change the
    /// answer.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run, or when
    /// it reports no cargo-udeps at that version, for example because nothing
    /// installed it.
    ///
    /// [requirement]: Udeps::requirement
    pub(crate) async fn resolve(context: &Context) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(&Self::requirement())
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
