//! The Rust toolchain that a job runs on
//!
//! Mise pins the Rust toolchains of a project, and it can pin more than one.
//! A toolchain is a decision of the project. A run selects one of the pins,
//! such as the stable release or the dated nightly. An action can state what
//! it requires of them. This module asks mise which pin the selection
//! and the requirements accept. A run then names that toolchain instead of
//! whatever rustup calls by the name of a channel on the machine.

mod selection;

use bon::bon;
use rakko::{Action, Context, ToolchainRequirement};
use rakko_tool::{ResolveToolchainError, Tool};

use self::selection::DATED_NIGHTLIES;
pub use self::selection::ToolchainSelection;
use crate::cargo::{CARGO, RUST};

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

    /// Returns what an action that needs the unstable options of the nightly
    /// requires of the Rust toolchains of the project
    ///
    /// A stable cargo refuses an unstable option with an error about the
    /// option, which does not say what the project must pin. The requirement
    /// accepts every nightly that the project pins with a date, such as
    /// `nightly-2026-11-02`. An action returns it from
    /// [`Action::toolchains`]. Then `doctor` reports a project that pins
    /// none, and a run of the action stops with an error that asks for such
    /// a pin. A lookup of Rust for the action that gives no selection then
    /// runs on the nightly, so the action states the nightly only once.
    ///
    /// # Examples
    ///
    /// ```
    /// use aonyx_rakko_cargo::Toolchain;
    ///
    /// let requirement = Toolchain::nightly_requirement();
    ///
    /// assert_eq!(requirement.versions().get(), "nightly-*");
    /// ```
    pub fn nightly_requirement() -> ToolchainRequirement {
        ToolchainRequirement::builder()
            .toolchain(RUST)
            .versions(rakko::VersionRange::new(DATED_NIGHTLIES))
            .build()
    }

    /// Returns the Rust toolchain that the project pins for a job of an
    /// action
    ///
    /// The lookup reads what the action requires of `rust` from
    /// [`Action::toolchains`], such as [`nightly_requirement`][nightly]. A
    /// selection picks the kind of pin that the job runs on. An example is the
    /// release that the `rust-version` of a manifest declares.
    /// Without a selection, the job runs on the dated nightly when the action
    /// requires one, and on the stable release otherwise. The lookup takes
    /// the first toolchain that the project pins and that the selection and
    /// all the requirements accept, as [`Tool::resolve_toolchain`] does.
    ///
    /// A pin in the global configuration of the user does not count, and
    /// neither does a pin in a directory above the project. The lookup starts
    /// processes, so a caller that needs the toolchain more than once keeps
    /// the answer for the length of the run.
    ///
    /// # Errors
    ///
    /// Returns a [`ResolveToolchainError`] when mise does not run, or when the
    /// project pins no toolchain that the selection and the requirements
    /// accept. It returns the same error when nothing installed the toolchain
    /// that it pins. The error says which pin to change, or that the pins
    /// need an install.
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
    /// use aonyx_rakko_cargo::{RustVersion, Toolchain, ToolchainSelection};
    ///
    /// # async fn run(action: &impl Action) -> Result<(), Box<dyn std::error::Error>> {
    /// let context = Context::builder().root("/home/otter/project").build();
    ///
    /// let toolchain = Toolchain::resolve()
    ///     .action(action)
    ///     .selection(ToolchainSelection::Release(RustVersion::new("1.88")))
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
        selection: Option<ToolchainSelection>,
        context: &Context,
    ) -> Result<Self, ResolveToolchainError> {
        let tool = Self::tool()
            .action(action)
            .maybe_selection(selection.as_ref())
            .context(context)
            .call()
            .await?;

        Ok(Self::of(&tool))
    }

    /// Returns the cargo of the Rust toolchain that a selection and the
    /// requirements of an action accept
    ///
    /// Without a selection, the requirements of the action decide which kind
    /// of pin the run takes, as [`ToolchainSelection`] says.
    ///
    /// # Errors
    ///
    /// Returns the error of [`Tool::resolve_toolchain`].
    #[builder]
    pub(crate) async fn tool<A: Action>(
        action: &A,
        selection: Option<&ToolchainSelection>,
        context: &Context,
    ) -> Result<Tool, ResolveToolchainError> {
        Tool::resolve_toolchain()
            .action(action)
            .toolchain(RUST)
            .executable(CARGO)
            .accepting(ToolchainSelection::range_of(
                selection,
                &action.toolchains(),
            ))
            .context(context)
            .call()
            .await
    }

    /// Returns the toolchain that a tool of a Rust requirement resolved to
    ///
    /// Rustup knows a toolchain by the version that mise resolved. So the
    /// version of the tool is the name of the toolchain.
    pub(crate) fn of(tool: &Tool) -> Self {
        Self::new(tool.version().get())
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
}
