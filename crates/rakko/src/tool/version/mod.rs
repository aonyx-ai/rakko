//! Types for the versions of a tool that an action runs

mod tool_version;
mod version_range;

pub use self::tool_version::ToolVersion;
pub use self::version_range::VersionRange;

/// The versions of a tool that an action runs
///
/// An action pins the exact version of a tool that it wraps, so that every
/// run uses the version that the tests of the action cover. A toolchain is a
/// tool that the code of the project builds or runs with, such as Node.js or
/// TypeScript. The project pins its toolchains. So an action states only the
/// range of versions that it accepts.
///
/// A later release can add a kind of requirement. So a match on
/// `RequiredVersion` outside this crate needs an arm for a kind that it does
/// not know.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[non_exhaustive]
pub enum RequiredVersion {
    /// The action runs this version of the tool, whatever the project pins
    Exact(ToolVersion),
    /// The action runs the version of the toolchain that the project pins,
    /// when that version is in this range
    Range(VersionRange),
}
