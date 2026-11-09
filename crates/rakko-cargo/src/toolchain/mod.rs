//! The Rust toolchain that a job runs on
//!
//! Mise pins the Rust toolchains of a project, and it can pin more than one.
//! A toolchain is a decision of the project, so an action states the range
//! of toolchains that it accepts, and this module asks mise which of the
//! pins of the project the range accepts. A run then names that toolchain
//! instead of whatever rustup calls by the name of a channel on the machine.

use rakko::{Context, MiseTool, RequiredVersion, ToolRequirement, VersionRange};
use rakko_tool::{ResolveRequirementError, Tool};

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
    /// toolchain by that name, so it reaches the toolchain that the project
    /// pinned and not whatever rustup calls `nightly` on the machine.
    Toolchain
}

impl Toolchain {
    /// Returns the argument that selects this toolchain for cargo
    ///
    /// The proxy of rustup reads a toolchain from its first argument when
    /// that argument starts with a plus sign, and it takes that argument
    /// over the environment that mise set for the default toolchain.
    pub(crate) fn argument(&self) -> String {
        format!("+{}", self.get())
    }

    /// Returns the requirement of the nightly toolchain that a job runs on
    ///
    /// The project pins the nightly by its date, such as
    /// `nightly-2026-11-02`, and the requirement holds the range of the dates
    /// that the crate accepts. An action that runs a job on the nightly
    /// returns it from its declaration of tools.
    pub fn nightly() -> ToolRequirement {
        Self::requirement(VersionRange::new(NIGHTLY))
    }

    /// Returns the requirement of the toolchain of one release of Rust
    ///
    /// The range accepts every patch of the release, so a version such as
    /// `1.88` accepts the pins `1.88.0` and `1.88.1`, and the version
    /// `1.88.0` accepts the pin `1.88.0` alone. The version comes from the
    /// project, such as the `rust-version` of a manifest, so an action
    /// builds this requirement during a run and cannot declare it.
    pub fn release(version: &RustVersion) -> ToolRequirement {
        Self::requirement(VersionRange::new(format!("={}", version.get())))
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
    /// Returns a [`ResolveRequirementError`] when mise does not run, when the
    /// project pins no toolchain that the range accepts, or when nothing
    /// installed the toolchain that it pins.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko::Context;
    /// use rakko_cargo::Toolchain;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let toolchain = Toolchain::resolve_requirement(&Toolchain::nightly(), &context).await?;
    ///
    /// println!("{toolchain}");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [resolve]: rakko_tool::Tool::resolve_requirement
    pub async fn resolve_requirement(
        requirement: &ToolRequirement,
        context: &Context,
    ) -> Result<Self, ResolveRequirementError> {
        let tool = Tool::resolve_requirement(requirement, context).await?;

        Ok(Self::of(&tool))
    }

    /// Returns the toolchain that a tool of a Rust requirement resolved to
    ///
    /// Rustup knows a toolchain by the version that mise resolved, so the
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
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    #[test]
    fn argument_puts_a_plus_sign_in_front_of_the_name() {
        let toolchain = Toolchain::new("nightly-2026-08-11");

        let argument = toolchain.argument();

        assert_eq!(argument, "+nightly-2026-08-11");
    }

    #[test]
    fn release_accepts_the_patches_of_the_version() {
        let requirement = Toolchain::release(&RustVersion::new("1.88"));

        assert_eq!(
            requirement.version(),
            &RequiredVersion::Range(VersionRange::new("=1.88"))
        );
    }
}
