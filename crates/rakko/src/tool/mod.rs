//! Types for the external tools that an action runs

mod executable;
mod mise_tool;
mod version;

use bon::bon;
use getset::Getters;

pub use self::executable::Executable;
pub use self::mise_tool::MiseTool;
pub use self::version::{RequiredVersion, ToolVersion, VersionRange};

/// A tool that an action runs, and the versions that it accepts
///
/// An action declares a requirement for each external program that it runs.
/// The requirement holds three parts:
///
/// - the identifier of the tool in the configuration of mise,
/// - the program of that tool that the action starts, and
/// - the versions of the tool that the action accepts.
///
/// Mise installs the tool. A resolver asks mise for the program at a version
/// that the requirement accepts.
///
/// # Examples
///
/// ```
/// use rakko::{Executable, MiseTool, RequiredVersion, ToolRequirement, ToolVersion};
///
/// let requirement = ToolRequirement::builder()
///     .tool(MiseTool::new("npm:markdownlint-cli"))
///     .executable(Executable::new("markdownlint"))
///     .version(RequiredVersion::Exact(ToolVersion::new("0.50.0")))
///     .build();
///
/// assert_eq!(requirement.executable().get(), "markdownlint");
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct ToolRequirement {
    /// The tool as the configuration of mise identifies it
    #[getset(get = "pub")]
    tool: MiseTool,

    /// The program of the tool that the action starts
    #[getset(get = "pub")]
    executable: Executable,

    /// The versions of the tool that the action accepts
    #[getset(get = "pub")]
    version: RequiredVersion,
}

#[bon]
impl ToolRequirement {
    /// Creates a requirement from a tool, its program, and its versions
    #[builder]
    pub fn new(
        #[builder(into)] tool: MiseTool,
        #[builder(into)] executable: Executable,
        version: RequiredVersion,
    ) -> Self {
        Self {
            tool,
            executable,
            version,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    fn requirement() -> ToolRequirement {
        ToolRequirement::builder()
            .tool("npm:markdownlint-cli")
            .executable("markdownlint")
            .version(RequiredVersion::Exact(ToolVersion::new("0.50.0")))
            .build()
    }

    #[test]
    fn executable_returns_the_given_program() {
        let requirement = requirement();

        assert_eq!(requirement.executable().get(), "markdownlint");
    }

    #[test]
    fn requirement_is_send_and_sync() {
        fn assert_send_and_sync<T: Send + Sync>() {}

        assert_send_and_sync::<ToolRequirement>();
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
    fn version_returns_the_given_versions() {
        let requirement = requirement();

        assert_eq!(
            requirement.version(),
            &RequiredVersion::Exact(ToolVersion::new("0.50.0"))
        );
    }
}
