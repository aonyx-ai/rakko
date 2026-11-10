//! Types for the external tools that an action runs

mod executable;
mod mise_tool;
mod platform;
mod version;

use std::collections::BTreeSet;

use bon::bon;
use getset::Getters;

pub use self::executable::Executable;
pub use self::mise_tool::MiseTool;
pub use self::platform::{Architecture, OperatingSystem, Platform};
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
/// A requirement of an exact version can also give the platforms where the
/// tool has a build. Mise then installs the tool only on those platforms. A
/// requirement without such a list needs the tool on every platform. A
/// toolchain gets no list, because the project pins its toolchains. So the
/// builder does not let a requirement of a range carry platforms.
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

    /// The platforms where the tool has a build, or `None` for every
    /// platform
    platforms: Option<BTreeSet<Platform>>,
}

#[bon]
impl ToolRequirement {
    /// Creates a requirement from a tool, its program, and its versions
    ///
    /// The builder sets the versions with `version`. For a tool that has a
    /// build only on some platforms, it sets the exact version and the
    /// platforms together with `exact_version_on`. So only a requirement of
    /// an exact version can carry platforms, and a toolchain cannot. Without
    /// platforms, the requirement needs the tool on every platform.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{RequiredVersion, ToolRequirement, ToolVersion};
    ///
    /// let requirement = ToolRequirement::builder()
    ///     .tool("npm:prettier")
    ///     .executable("prettier")
    ///     .version(RequiredVersion::Exact(ToolVersion::new("3.3.0")))
    ///     .build();
    ///
    /// assert_eq!(requirement.tool().get(), "npm:prettier");
    /// ```
    ///
    /// A tool with builds for Linux and for macOS on ARM only:
    ///
    /// ```
    /// use rakko::{Architecture, OperatingSystem, Platform, ToolRequirement, ToolVersion};
    ///
    /// let requirement = ToolRequirement::builder()
    ///     .tool("github:aonyx-ai/stiles")
    ///     .executable("stiles")
    ///     .exact_version_on(
    ///         ToolVersion::new("0.6.0"),
    ///         [
    ///             Platform::new(OperatingSystem::Linux),
    ///             Platform::new(OperatingSystem::Macos).with_architecture(Architecture::Arm64),
    ///         ],
    ///     )
    ///     .build();
    ///
    /// assert_eq!(requirement.platforms().map(|platforms| platforms.len()), Some(2));
    /// ```
    ///
    /// A toolchain cannot carry platforms, so this does not compile:
    ///
    /// ```compile_fail
    /// use std::collections::BTreeSet;
    ///
    /// use rakko::{OperatingSystem, Platform, RequiredVersion, ToolRequirement, VersionRange};
    ///
    /// let requirement = ToolRequirement::builder()
    ///     .tool("node")
    ///     .executable("node")
    ///     .version(RequiredVersion::Range(VersionRange::new(">=22")))
    ///     .platforms(BTreeSet::from([Platform::new(OperatingSystem::Linux)]))
    ///     .build();
    /// ```
    #[builder]
    pub fn new(
        #[builder(into)] tool: MiseTool,
        #[builder(into)] executable: Executable,
        version: RequiredVersion,
        #[builder(setters(vis = ""))] platforms: Option<BTreeSet<Platform>>,
    ) -> Self {
        Self {
            tool,
            executable,
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
    /// use rakko::{RequiredVersion, ToolRequirement, ToolVersion};
    ///
    /// let requirement = ToolRequirement::builder()
    ///     .tool("taplo")
    ///     .executable("taplo")
    ///     .version(RequiredVersion::Exact(ToolVersion::new("0.10.0")))
    ///     .build();
    ///
    /// assert_eq!(requirement.platforms(), None);
    /// ```
    #[must_use]
    pub fn platforms(&self) -> Option<&BTreeSet<Platform>> {
        self.platforms.as_ref()
    }
}

impl<S: tool_requirement_builder::State> ToolRequirementBuilder<S> {
    /// Sets the exact version of the tool and the platforms where the tool
    /// has a build
    ///
    /// Mise then installs the tool only on those platforms. The method takes
    /// the place of `version`, so a requirement of a range, which a
    /// toolchain declares, cannot carry platforms. Give at least one
    /// platform, because a tool without a platform runs nowhere.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{OperatingSystem, Platform, RequiredVersion, ToolRequirement, ToolVersion};
    ///
    /// let requirement = ToolRequirement::builder()
    ///     .tool("zizmor")
    ///     .executable("zizmor")
    ///     .exact_version_on(ToolVersion::new("1.32.0"), [Platform::new(OperatingSystem::Linux)])
    ///     .build();
    ///
    /// assert_eq!(
    ///     requirement.version(),
    ///     &RequiredVersion::Exact(ToolVersion::new("1.32.0"))
    /// );
    /// ```
    pub fn exact_version_on(
        self,
        version: ToolVersion,
        platforms: impl Into<BTreeSet<Platform>>,
    ) -> ToolRequirementBuilder<
        tool_requirement_builder::SetPlatforms<tool_requirement_builder::SetVersion<S>>,
    >
    where
        S::Version: tool_requirement_builder::IsUnset,
        S::Platforms: tool_requirement_builder::IsUnset,
    {
        self.version(RequiredVersion::Exact(version))
            .platforms(platforms.into())
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
    fn platforms_returns_the_given_platforms() {
        let requirement = ToolRequirement::builder()
            .tool("taplo")
            .executable("taplo")
            .exact_version_on(
                ToolVersion::new("0.10.0"),
                [Platform::new(OperatingSystem::Windows)],
            )
            .build();

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
