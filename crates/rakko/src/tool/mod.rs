//! Types for the external tools that an action runs

mod executable;
pub(crate) mod mise_file;
mod mise_tool;
mod platform;
mod toolchain_requirement;
mod tools;
mod version;

use std::collections::BTreeSet;

use bon::bon;
use getset::Getters;

pub use self::executable::Executable;
pub use self::mise_file::{MiseText, ToolPin};
pub use self::mise_tool::MiseTool;
pub(crate) use self::platform::Host;
pub use self::platform::{Architecture, OperatingSystem, Platform};
pub use self::toolchain_requirement::ToolchainRequirement;
pub use self::tools::Tools;
pub use self::version::{ToolVersion, VersionRange};

/// A tool that an action runs, at the exact version that a mise file pins
///
/// The machinery reads the requirements of an action before any run, from
/// [`Action::tools`][tools]. A requirement holds these parts:
///
/// - the identifier of the tool in the configuration of mise,
/// - the exact version of the tool, and
/// - the platforms where the tool has a build, or none for every platform.
///
/// Mise installs the tool. A resolver asks mise for a program of the tool at
/// that version.
///
/// Every requirement comes from a pin of a mise file. An action that wraps a
/// tool pins it in the mise file of its crate, and
/// [`bundled_tools!`][bundled] reads the pin when the crate compiles. No
/// other code creates a requirement. So an action declares only a tool that
/// a mise file pins. A requirement holds no program, because one tool can
/// provide several programs. The action gives the program when it runs the
/// tool.
///
/// A requirement can give the platforms where the tool has a build. Mise then
/// installs the tool only on those platforms. On every other platform, a run
/// of the action through [`ErasedAction`][erased] or [`BoundAction`][bound]
/// skips before it starts. The reason gives the tool and the platform. A call
/// of [`Action::run`][run] on the type of the action does not skip.
///
/// A toolchain, such as Rust or Node.js, is no tool of the action. The project
/// pins it, and an action that needs some of its versions declares a
/// [`ToolchainRequirement`].
///
/// [bound]: crate::BoundAction
/// [bundled]: crate::bundled_tools
/// [erased]: crate::ErasedAction
/// [run]: crate::Action::run
/// [tools]: crate::Action::tools
///
/// # Examples
///
/// ```
/// const TOOLS: rakko::Tools = rakko::__tools_of_mise_file!("[tools]\ntaplo = \"0.10.0\"\n");
///
/// let requirement = &TOOLS.requirements()[0];
///
/// assert_eq!(requirement.tool().get(), "taplo");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct ToolRequirement {
    /// The tool as the configuration of mise identifies it
    #[getset(get = "pub")]
    tool: MiseTool,

    /// The exact version of the tool that the action runs
    #[getset(get = "pub")]
    version: ToolVersion,

    /// The platforms where the tool has a build, or `None` for every
    /// platform
    platforms: Option<BTreeSet<Platform>>,
}

#[bon]
impl ToolRequirement {
    /// Creates the requirement of a pin of a mise file
    ///
    /// The requirement holds the exact version and the platforms of the pin.
    #[builder]
    pub(crate) fn declared(
        tool: MiseTool,
        version: ToolVersion,
        platforms: Option<BTreeSet<Platform>>,
    ) -> Self {
        Self {
            tool,
            version,
            platforms,
        }
    }

    /// Returns the platforms where the tool has a build, or `None` when the
    /// requirement needs the tool on every platform
    ///
    /// # Examples
    ///
    /// ```
    /// const TOOLS: rakko::Tools = rakko::__tools_of_mise_file!("[tools]\ntaplo = \"0.10.0\"\n");
    ///
    /// assert_eq!(TOOLS.requirements()[0].platforms(), None);
    /// ```
    #[must_use]
    pub fn platforms(&self) -> Option<&BTreeSet<Platform>> {
        self.platforms.as_ref()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Returns the requirement of a tool on every platform
    fn requirement() -> ToolRequirement {
        ToolRequirement::declared()
            .tool(MiseTool::new("npm:markdownlint-cli"))
            .version(ToolVersion::new("0.50.0"))
            .call()
    }

    #[test]
    fn platforms_returns_the_given_platforms() {
        let requirement = ToolRequirement::declared()
            .tool(MiseTool::new("taplo"))
            .version(ToolVersion::new("0.10.0"))
            .platforms(BTreeSet::from([Platform::new(OperatingSystem::Windows)]))
            .call();

        assert_eq!(
            requirement.platforms(),
            Some(&BTreeSet::from([Platform::new(OperatingSystem::Windows)]))
        );
    }

    #[test]
    fn platforms_without_a_list_returns_none() {
        let requirement = requirement();

        assert_eq!(requirement.platforms(), None);
    }

    #[test]
    fn tool_requirement_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ToolRequirement>();
    }

    #[test]
    fn tool_returns_the_given_tool() {
        let requirement = requirement();

        assert_eq!(requirement.tool().get(), "npm:markdownlint-cli");
    }

    #[test]
    fn version_returns_the_given_version() {
        let requirement = requirement();

        assert_eq!(requirement.version(), &ToolVersion::new("0.50.0"));
    }
}
