//! The Rust toolchain that a job runs on
//!
//! Mise pins the Rust toolchains of a project, and it can pin more than one.
//! This module asks mise which toolchain a channel resolved to, so that a
//! run names the toolchain that the project pinned instead of whatever
//! rustup calls by the name of the channel on the machine.

/// The channel that a project pins
mod channel;
/// What the crate reports when it cannot name a toolchain
mod error;

use rakko_action::ProjectRoot;
use rakko_tool::{Execution, Invocation, RunCommandError};
use serde::Deserialize;

pub use self::channel::Channel;
pub use self::error::{ResolveNewestToolchainError, ResolveToolchainError};
use crate::version::RustVersion;

/// The program that reports which toolchains a project pins
///
/// The operating system finds it with the rules of the platform. The
/// canonical way to start a harness enters the environment of mise first, so
/// a run that reaches an action reaches mise as well.
const MISE: &str = "mise";

/// The arguments that ask mise for the Rust toolchains of the project
///
/// Mise lists the versions of one tool that the project pins, and the JSON
/// form carries the version that a pin resolved to next to the pin as the
/// project wrote it.
const LIST: [&str; 4] = ["ls", "--current", "--json", "rust"];

/// The details of a run of mise that ended without success and wrote nothing
const NO_DIAGNOSIS: &str = "mise wrote nothing about it";

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
    // cargo[impl run.toolchain]
    pub fn argument(&self) -> String {
        format!("+{}", self.get())
    }

    /// Returns the toolchain that mise installed for a channel of a project
    ///
    /// The lookup asks mise about the project whose root the caller names,
    /// so the pin of the project answers, whatever the machine holds beside
    /// it. A pin that names the channel and a pin that names a dated
    /// toolchain of the channel both answer.
    ///
    /// The lookup starts a process, so a caller that needs the toolchain
    /// more than once keeps the answer for the length of the run.
    ///
    /// # Errors
    ///
    /// Returns [`MiseUnavailable`][unavailable] when mise does not run,
    /// [`UnpinnedToolchain`][unpinned] when the project pins no toolchain of
    /// the channel, [`UninstalledToolchain`][uninstalled] when the pin
    /// resolved to a toolchain that nothing installed, and
    /// [`UnreadableReport`][unreadable] when mise answered in a shape that
    /// the crate cannot read. Rakko installs nothing, so a toolchain that no
    /// one installed is a failure and not a step that this method takes.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    /// use rakko_cargo::{Channel, Toolchain};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let root = ProjectRoot::new("/home/otter/project".into());
    ///
    /// let toolchain = Toolchain::resolve(Channel::new("nightly"), &root).await?;
    ///
    /// println!("{}", toolchain.argument());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [unavailable]: ResolveToolchainError::MiseUnavailable
    /// [uninstalled]: ResolveToolchainError::UninstalledToolchain
    /// [unpinned]: ResolveToolchainError::UnpinnedToolchain
    /// [unreadable]: ResolveToolchainError::UnreadableReport
    // cargo[impl toolchain.resolve]
    // cargo[impl toolchain.uninstalled]
    // cargo[impl toolchain.unpinned]
    // cargo[impl toolchain.report]
    pub async fn resolve(
        channel: Channel,
        root: &ProjectRoot,
    ) -> Result<Self, ResolveToolchainError> {
        let pins = report(root).await.map_err(|failure| match failure {
            PinsFailure::Unavailable { source } => ResolveToolchainError::MiseUnavailable {
                channel: channel.clone(),
                source,
            },
            PinsFailure::Unreadable { details } => ResolveToolchainError::UnreadableReport {
                channel: channel.clone(),
                details,
            },
        })?;

        select(&pins, channel)
    }

    /// Returns the toolchain that a project builds with
    ///
    /// A project pins the toolchain of its builds by an exact version, next
    /// to the channels that it pins for a job that needs one, such as
    /// `nightly`. Mise resolves a channel to another name, so the pins that
    /// name their own version are the candidates, and the newest of them is
    /// the toolchain of the project.
    ///
    /// Mise lists the pins in an order of its own, so the choice reads the
    /// versions and not the order. It reads each part of a version as a
    /// number, because `1.9` comes before `1.88` as text and after it as a
    /// version.
    ///
    /// The lookup starts a process, so a caller that needs the toolchain
    /// more than once keeps the answer for the length of the run.
    ///
    /// # Errors
    ///
    /// Returns [`MiseUnavailable`][unavailable] when mise does not run,
    /// [`UnpinnedToolchain`][unpinned] when every Rust pin of the project
    /// names a channel, [`UninstalledToolchain`][uninstalled] when the
    /// newest pin is absent from the machine, and
    /// [`UnreadableReport`][unreadable] when mise answered in a shape that
    /// the crate cannot read. Rakko installs nothing, so a toolchain that no
    /// one installed is a failure and not a step that this method takes.
    ///
    /// # Panics
    ///
    /// Panics when no Tokio runtime drives the future. The runtime waits for
    /// mise, and the method has no way to ask without one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rakko_action::ProjectRoot;
    /// use rakko_cargo::Toolchain;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let root = ProjectRoot::new("/home/otter/project".into());
    ///
    /// let toolchain = Toolchain::newest(&root).await?;
    ///
    /// println!("{}", toolchain.argument());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [uninstalled]: ResolveNewestToolchainError::UninstalledToolchain
    /// [unavailable]: ResolveNewestToolchainError::MiseUnavailable
    /// [unpinned]: ResolveNewestToolchainError::UnpinnedToolchain
    /// [unreadable]: ResolveNewestToolchainError::UnreadableReport
    // cargo[impl toolchain.newest]
    // cargo[impl toolchain.unversioned]
    // cargo[impl toolchain.absent]
    pub async fn newest(root: &ProjectRoot) -> Result<Self, ResolveNewestToolchainError> {
        let pins = report(root).await.map_err(|failure| match failure {
            PinsFailure::Unavailable { source } => {
                ResolveNewestToolchainError::MiseUnavailable { source }
            }
            PinsFailure::Unreadable { details } => {
                ResolveNewestToolchainError::UnreadableReport { details }
            }
        })?;

        newest_of(&pins)
    }
}

/// What went wrong when mise listed the Rust pins of a project
///
/// The question has two callers, and each of them reports the failure in the
/// error of its own question. This enum carries what happened, and the
/// caller adds the context that its question knows.
#[derive(Debug)]
enum PinsFailure {
    /// Mise did not run
    Unavailable {
        /// The cause of the failure
        source: RunCommandError,
    },

    /// Mise ended without success, or answered in a shape the crate does not
    /// know
    Unreadable {
        /// What mise wrote, or what could not be read
        details: String,
    },
}

/// One Rust toolchain that mise lists for a project
///
/// Mise writes more than these fields, and the reading ignores the rest, so a
/// field that a new version adds does not break it.
#[derive(Deserialize)]
struct Pin {
    /// The toolchain that the pin resolved to, as rustup knows it
    version: String,

    /// The pin as the project wrote it, such as `nightly`
    requested_version: String,

    /// Whether the toolchain is present on the machine
    installed: bool,
}

impl Pin {
    /// Returns the version that the pin resolved to, when it resolved to one
    ///
    /// A project pins a toolchain by a version, such as `1.98.0` or `1.85`,
    /// and mise resolves both to a version of the compiler. It resolves a
    /// channel such as `nightly` to a dated toolchain instead, whose name is
    /// no version, and such a pin says nothing about which compiler the
    /// project builds with.
    // cargo[impl toolchain.newest]
    fn version(&self) -> Option<RustVersion> {
        RustVersion::parse(&self.version)
    }

    /// Returns whether the pin belongs to the channel
    ///
    /// A project writes the channel itself, or it writes a dated toolchain
    /// of the channel, and the second form is the first with a date behind
    /// a hyphen.
    fn belongs_to(&self, channel: &Channel) -> bool {
        self.requested_version == channel.get()
            || self
                .requested_version
                .strip_prefix(channel.get())
                .is_some_and(|rest| rest.starts_with('-'))
    }
}

/// Asks mise which Rust toolchains a project pins
///
/// # Errors
///
/// Returns [`Unavailable`][unavailable] when mise does not run, and
/// [`Unreadable`][unreadable] when it ended without success or answered in a
/// shape that the crate does not recognize.
///
/// [unavailable]: PinsFailure::Unavailable
/// [unreadable]: PinsFailure::Unreadable
async fn report(root: &ProjectRoot) -> Result<Vec<Pin>, PinsFailure> {
    let execution = Invocation::new(MISE)
        .args(LIST)
        .in_directory(root.get())
        .run()
        .await
        .map_err(|source| PinsFailure::Unavailable { source })?;

    pins(&execution).map_err(|details| PinsFailure::Unreadable { details })
}

/// Returns the toolchains that mise listed
///
/// # Errors
///
/// Returns what mise wrote when it ended without success, and what could not
/// be read when its answer has a shape that the crate does not recognize.
fn pins(execution: &Execution) -> Result<Vec<Pin>, String> {
    if !execution.status().success() {
        let diagnosis = execution.stderr().to_string_lossy();
        let text = diagnosis.trim();

        return Err(if text.is_empty() {
            NO_DIAGNOSIS.to_owned()
        } else {
            text.to_owned()
        });
    }

    serde_json::from_slice(execution.stdout().get()).map_err(|error| error.to_string())
}

/// Returns the newest toolchain that the pins of a project name by an exact
/// version
///
/// A pin that mise resolved to a name which is no version is no candidate,
/// because such a pin names a channel and says nothing about which compiler
/// the project builds with.
///
/// # Errors
///
/// Returns [`UnpinnedToolchain`][unpinned] when no pin names its own
/// version, and [`UninstalledToolchain`][uninstalled] when the newest of
/// them is absent from the machine.
///
/// [uninstalled]: ResolveNewestToolchainError::UninstalledToolchain
/// [unpinned]: ResolveNewestToolchainError::UnpinnedToolchain
// cargo[impl toolchain.newest]
// cargo[impl toolchain.unversioned]
// cargo[impl toolchain.absent]
fn newest_of(pins: &[Pin]) -> Result<Toolchain, ResolveNewestToolchainError> {
    let versioned: Vec<(&Pin, RustVersion)> = pins
        .iter()
        .filter_map(|pin| pin.version().map(|version| (pin, version)))
        .collect();

    let Some(newest) = RustVersion::highest(versioned.iter().map(|(_, version)| version.clone()))
    else {
        return Err(ResolveNewestToolchainError::UnpinnedToolchain);
    };

    let toolchain = Toolchain::new(newest.get());

    if !versioned
        .iter()
        .any(|(pin, version)| version == &newest && pin.installed)
    {
        return Err(ResolveNewestToolchainError::UninstalledToolchain { toolchain });
    }

    Ok(toolchain)
}

/// Returns the toolchain of the channel among the pins of a project
///
/// # Errors
///
/// Returns [`UnpinnedToolchain`][unpinned] when no pin belongs to the
/// channel, and [`UninstalledToolchain`][uninstalled] when the pin resolved
/// to a toolchain that is absent from the machine.
///
/// [uninstalled]: ResolveToolchainError::UninstalledToolchain
/// [unpinned]: ResolveToolchainError::UnpinnedToolchain
// cargo[impl toolchain.resolve]
// cargo[impl toolchain.uninstalled]
// cargo[impl toolchain.unpinned]
fn select(pins: &[Pin], channel: Channel) -> Result<Toolchain, ResolveToolchainError> {
    let Some(pin) = pins.iter().find(|pin| pin.belongs_to(&channel)) else {
        return Err(ResolveToolchainError::UnpinnedToolchain { channel });
    };

    let toolchain = Toolchain::new(pin.version.clone());

    if !pin.installed {
        return Err(ResolveToolchainError::UninstalledToolchain { channel, toolchain });
    }

    Ok(toolchain)
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns a pin that the tests select from
    fn pin(version: &str, requested: &str, installed: bool) -> Pin {
        Pin {
            version: version.to_owned(),
            requested_version: requested.to_owned(),
            installed,
        }
    }

    /// Returns the pins of a project with a default and a nightly toolchain
    fn pins() -> Vec<Pin> {
        vec![
            pin("1.98.0", "1.98.0", true),
            pin("nightly-2026-08-11", "nightly", true),
        ]
    }

    // cargo[verify run.toolchain]
    #[test]
    fn argument_puts_a_plus_sign_in_front_of_the_name() {
        let toolchain = Toolchain::new("nightly-2026-08-11");

        let argument = toolchain.argument();

        assert_eq!(argument, "+nightly-2026-08-11");
    }

    // cargo[verify toolchain.absent]
    #[test]
    fn newest_of_a_pin_that_nothing_installed_names_the_toolchain() {
        let pins = vec![pin("1.98.0", "1.98.0", false)];

        let toolchain = newest_of(&pins);

        assert!(
            matches!(
                toolchain,
                Err(ResolveNewestToolchainError::UninstalledToolchain { ref toolchain })
                    if toolchain.get() == "1.98.0"
            ),
            "expected an uninstalled toolchain, got {toolchain:?}"
        );
    }

    // cargo[verify toolchain.newest]
    #[test]
    fn newest_of_pins_ignores_a_channel_that_mise_resolved() {
        let toolchain = newest_of(&pins());

        assert_eq!(toolchain.ok(), Some(Toolchain::new("1.98.0")));
    }

    // cargo[verify toolchain.newest]
    #[test]
    fn newest_of_a_pin_that_mise_completed_names_the_toolchain_it_resolved_to() {
        let pins = vec![pin("1.85.1", "1.85", true)];

        let toolchain = newest_of(&pins);

        assert_eq!(toolchain.ok(), Some(Toolchain::new("1.85.1")));
    }

    // cargo[verify toolchain.newest]
    #[test]
    fn newest_of_pins_in_ascending_order_answers_the_highest() {
        let pins = vec![pin("1.88.0", "1.88.0", true), pin("1.98.0", "1.98.0", true)];

        let toolchain = newest_of(&pins);

        assert_eq!(toolchain.ok(), Some(Toolchain::new("1.98.0")));
    }

    // cargo[verify toolchain.newest]
    #[test]
    fn newest_of_pins_in_descending_order_answers_the_highest() {
        let pins = vec![pin("1.98.0", "1.98.0", true), pin("1.88.0", "1.88.0", true)];

        let toolchain = newest_of(&pins);

        assert_eq!(toolchain.ok(), Some(Toolchain::new("1.98.0")));
    }

    // cargo[verify toolchain.newest]
    #[test]
    fn newest_of_pins_reads_a_part_as_a_number_and_not_as_text() {
        let pins = vec![pin("1.9.0", "1.9.0", true), pin("1.88.0", "1.88.0", true)];

        let toolchain = newest_of(&pins);

        assert_eq!(toolchain.ok(), Some(Toolchain::new("1.88.0")));
    }

    // cargo[verify toolchain.unversioned]
    #[test]
    fn newest_of_pins_that_only_name_channels_reports_no_pin() {
        let pins = vec![pin("nightly-2026-08-11", "nightly", true)];

        let toolchain = newest_of(&pins);

        assert!(
            matches!(
                toolchain,
                Err(ResolveNewestToolchainError::UnpinnedToolchain)
            ),
            "expected no pin by a version, got {toolchain:?}"
        );
    }

    // cargo[verify toolchain.resolve]
    #[test]
    fn select_a_channel_that_a_dated_pin_names_answers_that_toolchain() {
        let pins = vec![pin("nightly-2026-08-11", "nightly-2026-08-11", true)];

        let toolchain = select(&pins, Channel::new("nightly"));

        assert_eq!(toolchain.ok(), Some(Toolchain::new("nightly-2026-08-11")));
    }

    // cargo[verify toolchain.unpinned]
    #[test]
    fn select_a_channel_that_prefixes_another_pin_does_not_match_it() {
        let toolchain = select(&pins(), Channel::new("night"));

        assert!(
            matches!(
                toolchain,
                Err(ResolveToolchainError::UnpinnedToolchain { .. })
            ),
            "expected an unpinned channel, got {toolchain:?}"
        );
    }

    // cargo[verify toolchain.uninstalled]
    #[test]
    fn select_a_pin_that_nothing_installed_names_the_toolchain() {
        let pins = vec![pin("nightly-2026-08-11", "nightly", false)];

        let toolchain = select(&pins, Channel::new("nightly"));

        assert!(
            matches!(
                toolchain,
                Err(ResolveToolchainError::UninstalledToolchain { ref toolchain, .. })
                    if toolchain.get() == "nightly-2026-08-11"
            ),
            "expected an uninstalled toolchain, got {toolchain:?}"
        );
    }

    // cargo[verify toolchain.resolve]
    #[test]
    fn select_a_pinned_channel_answers_the_toolchain_it_resolved_to() {
        let toolchain = select(&pins(), Channel::new("nightly"));

        assert_eq!(toolchain.ok(), Some(Toolchain::new("nightly-2026-08-11")));
    }

    // cargo[verify toolchain.unpinned]
    #[test]
    fn select_an_unpinned_channel_names_the_channel() {
        let toolchain = select(&pins(), Channel::new("beta"));

        assert!(
            matches!(
                toolchain,
                Err(ResolveToolchainError::UnpinnedToolchain { ref channel })
                    if channel.get() == "beta"
            ),
            "expected an unpinned channel, got {toolchain:?}"
        );
    }
}
