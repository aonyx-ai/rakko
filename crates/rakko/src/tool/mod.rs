//! Types for the external tools that an action runs

mod executable;
pub(crate) mod mise_file;
mod mise_tool;
mod platform;
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
pub use self::tools::Tools;
pub use self::version::{RequiredVersion, ToolVersion, VersionRange};

/// A tool that an action runs, and the versions that it accepts
///
/// The machinery reads the requirements of an action before any run, from
/// [`Action::tools`][tools]. A requirement holds these parts:
///
/// - the identifier of the tool in the configuration of mise,
/// - the versions of the tool that the action accepts, and
/// - the platforms where the tool has a build, for an exact version.
///
/// Mise installs the tool. A resolver asks mise for a program of the tool at
/// a version that the requirement accepts.
///
/// An action that wraps a tool pins it in its mise file, and
/// [`bundled_tools!`][bundled] reads the pin. The requirement of such a pin
/// holds the exact version and the platforms of the pin. It holds no
/// program, because one tool can provide several programs. The action gives
/// the program when it runs the tool. A requirement that the builder creates also
/// holds the program that the action starts, and it can hold the range of
/// versions of a toolchain.
///
/// A requirement of an exact version can give the platforms where the tool
/// has a build. Mise then installs the tool only on those platforms. On every
/// other platform, a run of the action through [`ErasedAction`][erased] or
/// [`BoundAction`][bound] skips before it starts. The reason gives the tool
/// and the platform. A call of [`Action::run`][run] on the type of the action
/// does not skip. A requirement without such a list needs the tool on every
/// platform. A toolchain gets no list, because the project pins its
/// toolchains. So the builder does not let a requirement of a range carry
/// platforms.
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
/// use rakko::{Executable, MiseTool, RequiredVersion, ToolRequirement, ToolVersion};
///
/// let requirement = ToolRequirement::builder()
///     .tool(MiseTool::new("npm:markdownlint-cli"))
///     .executable(Executable::new("markdownlint"))
///     .version(RequiredVersion::Exact(ToolVersion::new("0.50.0")))
///     .build();
///
/// assert_eq!(
///     requirement.executable().map(Executable::get),
///     Some("markdownlint")
/// );
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Getters)]
pub struct ToolRequirement {
    /// The tool as the configuration of mise identifies it
    #[getset(get = "pub")]
    tool: MiseTool,

    /// The program of the tool that the action starts, or `None` for the
    /// requirement of a pin, whose program the action gives when it runs
    executable: Option<Executable>,

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
            executable: Some(executable),
            version,
            platforms,
        }
    }

    /// Creates the requirement of a pin of a mise file
    ///
    /// The requirement holds the exact version and the platforms of the pin,
    /// and no program.
    #[builder]
    pub(crate) fn declared(
        tool: MiseTool,
        version: ToolVersion,
        platforms: Option<BTreeSet<Platform>>,
    ) -> Self {
        Self {
            tool,
            executable: None,
            version: RequiredVersion::Exact(version),
            platforms,
        }
    }

    /// Returns the program of the tool that the action starts, or `None` for
    /// the requirement of a pin of a mise file
    ///
    /// The requirement of a pin holds no program, because one tool can
    /// provide several programs. The action gives the program when it runs
    /// the tool.
    ///
    /// # Examples
    ///
    /// ```
    /// use rakko::{Executable, RequiredVersion, ToolRequirement, ToolVersion};
    ///
    /// let requirement = ToolRequirement::builder()
    ///     .tool("npm:prettier")
    ///     .executable("prettier")
    ///     .version(RequiredVersion::Exact(ToolVersion::new("3.3.0")))
    ///     .build();
    ///
    /// assert_eq!(requirement.executable().map(Executable::get), Some("prettier"));
    /// ```
    #[must_use]
    pub fn executable(&self) -> Option<&Executable> {
        self.executable.as_ref()
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

        assert_eq!(
            requirement.executable(),
            Some(&Executable::new("markdownlint"))
        );
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
