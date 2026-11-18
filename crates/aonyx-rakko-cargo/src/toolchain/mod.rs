//! The Rust toolchain that a job runs on
//!
//! Mise pins the Rust toolchains of a project, and it can pin more than one.
//! A toolchain is a decision of the project, so an action states the
//! range of toolchains that it accepts. This module asks mise which of
//! the pins of the project the range accepts. A run then names that
//! toolchain instead of whatever rustup calls by the name of a channel on
//! the machine.

use bon::bon;
use rakko::{
    Action, Context, MiseTool, RequiredVersion, ToolRequirement, ToolchainRequirement, VersionRange,
};
use rakko_tool::{ResolveRequirementError, ResolveToolchainError, Tool};

use crate::cargo::{CARGO, RUST};
use crate::version::RustVersion;

/// The range of the nightly toolchains that the crate accepts
///
/// The range starts at the release of Rust 1.85, the oldest stable
/// toolchain that the crate accepts, so a nightly is never older than it.
const NIGHTLY: &str = ">=nightly-2025-02-20";

typed_fields::name! {
    /// A Rust toolchain, by the name that rustup knows it by
    ///
    /// Mise installs a channel such as `nightly` as a dated toolchain, and
    /// `nightly-2026-08-11` is the name that rustup knows. A run selects the
    /// toolchain by that name. So it reaches the toolchain that the project
    /// pinned and not whatever rustup calls `nightly` on the machine.
    Toolchain
}

#[bon]
impl Toolchain {
    /// Returns the argument that selects this toolchain for cargo
    ///
    /// The proxy of rustup reads a toolchain from its first argument when
    /// that argument starts with a plus sign. It takes that argument over
    /// the environment that mise set for the default toolchain.
    pub(crate) fn argument(&self) -> String {
        format!("+{}", self.get())
    }

    /// Returns the requirement of the nightly toolchain that a job runs on
    ///
    /// The project pins the nightly by its date, such as
    /// `nightly-2026-11-02`, and the requirement holds the range of the dates
    /// that the crate accepts. An action that runs a job on the nightly
    /// returns it from its declaration of tools.
    ///
    /// # Examples
    ///
    /// ```
    /// use aonyx_rakko_cargo::Toolchain;
    ///
    /// let requirement = Toolchain::nightly();
    ///
    /// assert_eq!(requirement.tool().get(), "rust");
    /// ```
    pub fn nightly() -> ToolRequirement {
        Self::requirement(VersionRange::new(NIGHTLY))
    }

    /// Returns what an action that runs a job on the nightly requires of the
    /// Rust toolchains of the project
    ///
    /// The project pins the nightly by its date, such as
    /// `nightly-2026-11-02`, and the requirement accepts the dates from the
    /// release of Rust 1.85 on. An action that needs the unstable options of
    /// the nightly returns it from [`Action::toolchains`]. Then
    /// [`resolve`][resolve] selects the nightly that the project pins, and
    /// `doctor` reports a project that pins none.
    ///
    /// # Examples
    ///
    /// ```
    /// use aonyx_rakko_cargo::Toolchain;
    ///
    /// let requirement = Toolchain::nightly_requirement();
    ///
    /// assert_eq!(requirement.versions().get(), ">=nightly-2025-02-20");
    /// ```
    ///
    /// [resolve]: Toolchain::resolve
    pub fn nightly_requirement() -> ToolchainRequirement {
        ToolchainRequirement::builder()
            .toolchain(RUST)
            .versions(VersionRange::new(NIGHTLY))
            .build()
    }

    /// Returns the Rust toolchain that the project pins for a job of an
    /// action
    ///
    /// The lookup reads what the action requires of `rust` from
    /// [`Action::toolchains`], such as
    /// [`nightly_requirement`][nightly]. A job that must run on one release
    /// gives that release with `release`, such as the `rust-version` of a
    /// manifest. The lookup then accepts every patch of the release. So the
    /// version `1.88` accepts the pins `1.88.0` and `1.88.1`, and the version
    /// `1.88.0` accepts the pin `1.88.0` alone. The lookup takes the first
    /// toolchain that the project pins and that all of these accept. It does
    /// so as [`Tool::resolve_toolchain`] does. Without a requirement and a release,
    /// it takes the first Rust toolchain that the project pins.
    ///
    /// A pin in the global configuration of the user does not count, and
    /// neither does a pin in a directory above the project. The lookup starts
    /// processes, so a caller that needs the toolchain more than once keeps
    /// the answer for the length of the run.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolchainError`] when mise does not run, or when the
    /// project pins no toolchain that the requirements accept. It returns the
    /// same error when nothing installed the toolchain that it pins. The error
    /// says which pin to change, or that the pins need an install.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Action, Context};
    /// use aonyx_rakko_cargo::Toolchain;
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let toolchain = Toolchain::resolve()
    ///     .action(action)
    ///     .context(&context)
    ///     .call()
    ///     .await?;
    ///
    /// println!("{toolchain}");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [nightly]: Toolchain::nightly_requirement
    #[builder]
    pub async fn resolve<A: Action>(
        action: &A,
        release: Option<&RustVersion>,
        context: &Context,
    ) -> Result<Self, ResolveToolchainError> {
        let tool = Tool::resolve_toolchain()
            .action(action)
            .toolchain(RUST)
            .executable(CARGO)
            .maybe_accepting(release.map(Self::release_range))
            .context(context)
            .call()
            .await?;

        Ok(Self::of(&tool))
    }

    /// Returns the range that accepts every patch of one release of Rust
    ///
    /// A version such as `1.88` accepts the pins `1.88.0` and `1.88.1`. The
    /// version `1.88.0` accepts the pin `1.88.0` alone.
    fn release_range(version: &RustVersion) -> VersionRange {
        VersionRange::new(format!("={}", version.get()))
    }

    /// Returns the toolchain that the project pins and that a requirement
    /// accepts
    ///
    /// The lookup takes the first toolchain that the project pins and that
    /// the range of the requirement accepts, as
    /// [`Tool::resolve_requirement`][resolve] does. A pin in the global
    /// configuration of the user does not count, and neither does a pin in a
    /// directory above the project.
    ///
    /// The lookup starts processes, so a caller that needs the toolchain
    /// more than once keeps the answer for the length of the run.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveRequirementError`] when mise does not run or when
    /// the project pins no toolchain that the range accepts. It returns the same
    /// error when nothing installed the toolchain that it pins.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::{Context, ProjectRoot};
    /// use aonyx_rakko_cargo::Toolchain;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root(ProjectRoot::new(std::env::temp_dir())).build();
    ///
    /// let toolchain = Toolchain::resolve_requirement()
    ///     .requirement(&Toolchain::nightly())
    ///     .context(&context)
    ///     .call()
    ///     .await?;
    ///
    /// assert!(toolchain.get().starts_with("nightly-"));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [resolve]: rakko_tool::Tool::resolve_requirement
    #[builder]
    pub async fn resolve_requirement(
        requirement: &ToolRequirement,
        context: &Context,
    ) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement()
            .requirement(requirement)
            .context(context)
            .call()
            .await?;

        Ok(Self::of(&tool))
    }

    /// Returns the toolchain that a tool of a Rust requirement resolved to
    ///
    /// Rustup knows a toolchain by the version that mise resolved. So the
    /// version of the tool is the name of the toolchain.
    pub(crate) fn of(tool: &Tool) -> Self {
        Self::new(tool.version().get())
    }

    /// Returns the requirement of a Rust toolchain in a range
    fn requirement(range: VersionRange) -> ToolRequirement {
        ToolRequirement::builder()
            .tool(MiseTool::new(RUST))
            .executable(CARGO)
            .version(RequiredVersion::Range(range))
            .build()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn argument_puts_a_plus_sign_in_front_of_the_name() {
        let toolchain = Toolchain::new("nightly-2026-08-11");

        let argument = toolchain.argument();

        assert_eq!(argument, "+nightly-2026-08-11");
    }

    #[test]
    fn release_range_accepts_the_patches_of_the_version() {
        let range = Toolchain::release_range(&RustVersion::new("1.88"));

        assert_eq!(range, VersionRange::new("=1.88"));
    }
}
