//! The version of a toolchain that a project pins
//!
//! A toolchain is a decision of the project, so the project pins its
//! version, and an action states only the range of versions that it accepts.
//! This module asks mise which versions the configuration of the project
//! pins, in the order of its pins, and picks the first one that the range
//! accepts.
//!
//! A range compares the versions of releases, such as `>=1.85`, or the dates
//! of one channel, such as `>=nightly-2025-02-20`. Rustup names a dated
//! toolchain of a channel by its date, so a nightly toolchain has a date
//! and no version of its own to compare. A range can hold alternatives,
//! joined by `||`, such as `^22.18 || >=23.6`, and it accepts a pin that one
//! of them accepts.

use std::path::{Path, PathBuf};

use kawauso_process::{Execution, Invocation};
use rakko::{MiseTool, ProjectRoot, ToolVersion, VersionRange};
use semver::{Version, VersionReq};
use serde::Deserialize;

use super::{CURRENT, MISE, diagnosis};
use crate::tool::ResolveRequirementError;

/// The subcommand of mise that lists the versions of a tool
const LIST: &str = "ls";

/// The flag that limits a list to the versions that a configuration pins
const PINNED: &str = "--current";

/// The flag that asks for a list in JSON, with the source of each pin
const JSON: &str = "--json";

/// The details of a toolchain that only a configuration outside the project
/// pins, or that nothing pins
const NO_PROJECT_PIN: &str = "no configuration inside the project pins a version";

/// One version of a tool that mise lists, with the file that pins it
#[derive(Debug, Deserialize)]
struct Pin {
    /// The version that the pin resolves to
    version: String,

    /// The version as the configuration writes it, such as `nightly`
    ///
    /// Mise can resolve a bare channel to a dated version that the machine
    /// holds, so only this field shows whether the project pins a date.
    #[serde(default)]
    requested_version: Option<String>,

    /// Where the pin comes from, when it comes from a file
    #[serde(default)]
    source: Option<Source>,
}

/// The origin of a pin that mise lists
#[derive(Debug, Deserialize)]
struct Source {
    /// The configuration file that holds the pin
    #[serde(default)]
    path: Option<PathBuf>,
}

/// Returns the first version of a toolchain that the project pins and that
/// the range accepts
///
/// A range of dates reads only the pins of its channel that the
/// configuration writes with a date, and it passes over every other pin of
/// the toolchain. A range of releases reads every pin.
///
/// Only a pin in a configuration file inside the root of the project counts.
/// Mise also applies the global configuration of the user and the files of
/// the directories above the project. These files are different on each
/// machine, so a run on another machine then chooses another version. A local
/// mise file inside the project, such as `mise.local.toml`, counts. So a
/// developer can override the pin of the project on their own machine.
///
/// # Errors
///
/// Returns [`InvalidRange`][invalid] when the range is not a range,
/// [`MiseUnavailable`][unavailable] when mise does not run,
/// [`UnrecognizedReport`][unrecognized] when mise lists the pins in a form
/// that the crate does not read, and [`UnresolvedToolchain`][unresolved]
/// when no configuration inside the project pins the toolchain, or no dated
/// version of the channel of a range of dates. When the project pins
/// versions and the range accepts none of them, returns
/// [`VersionOutOfRange`][outside] for the first pin with three numbers or a
/// date to compare, or [`UnreadableVersion`][unreadable] when a range of
/// releases finds no pin with three numbers.
///
/// [invalid]: ResolveRequirementError::InvalidRange
/// [outside]: ResolveRequirementError::VersionOutOfRange
/// [unavailable]: ResolveRequirementError::MiseUnavailable
/// [unreadable]: ResolveRequirementError::UnreadableVersion
/// [unrecognized]: ResolveRequirementError::UnrecognizedReport
/// [unresolved]: ResolveRequirementError::UnresolvedToolchain
pub(super) async fn pinned_toolchain(
    tool: &MiseTool,
    range: &VersionRange,
    root: &ProjectRoot,
) -> Result<ToolVersion, ResolveRequirementError> {
    let accepted =
        Accepted::parse(range).map_err(|source| ResolveRequirementError::InvalidRange {
            tool: tool.clone(),
            range: range.clone(),
            source,
        })?;

    let project = project_pins(tool, root).await?;

    let current = ask(tool, root, &[CURRENT, tool.get()]).await?;

    let output = current.stdout().to_string_lossy();
    let ordered = output.split_whitespace().filter_map(|version| {
        project
            .iter()
            .find(|pin| pin.version == version)
            .map(|pin| (version, pin.requested_version.as_deref().unwrap_or(version)))
    });

    let mut outside: Option<&str> = None;
    let mut unreadable: Option<(&str, semver::Error)> = None;

    for (pin, requested) in ordered {
        match accepted.read(pin, requested) {
            Reading::Accepted => return Ok(ToolVersion::new(pin)),
            Reading::Outside => {
                outside.get_or_insert(pin);
            }
            Reading::Unreadable(source) => {
                unreadable.get_or_insert((pin, source));
            }
            Reading::Foreign => {}
        }
    }

    match (outside, unreadable) {
        (Some(pin), _) => Err(ResolveRequirementError::VersionOutOfRange {
            tool: tool.clone(),
            version: ToolVersion::new(pin),
            range: range.clone(),
        }),
        (None, Some((pin, source))) => Err(ResolveRequirementError::UnreadableVersion {
            tool: tool.clone(),
            version: ToolVersion::new(pin),
            source,
        }),
        (None, None) => Err(ResolveRequirementError::UnresolvedToolchain {
            tool: tool.clone(),
            details: accepted.unpinned(),
        }),
    }
}

/// The text that joins the alternatives of a range
const ALTERNATIVE: &str = "||";

/// The versions that a range accepts, in the form that the pins are read in
///
/// A range compares the versions of releases, such as `>=1.85`. A range can
/// also compare the dates of a channel, such as `>=nightly-2025-02-20`. Every
/// comparator of such a range gives the same channel and a date. A pin of
/// that channel then carries a date as well, such as `nightly-2026-11-02`,
/// and the range compares the dates.
///
/// A range holds one or more alternatives, and it accepts a pin that one of
/// them accepts. The alternatives of a range of dates give the same channel.
#[derive(Debug)]
enum Accepted {
    /// A range of the versions of releases
    Releases(Vec<VersionReq>),

    /// A range of the dates of one channel
    Dated {
        /// The channel that every comparator of the range gives
        channel: String,

        /// The alternatives of the range of the dates, with each date read
        /// as a version
        dates: Vec<VersionReq>,
    },
}

/// What a range says about one pin
#[derive(Debug)]
enum Reading {
    /// The range accepts the pin
    Accepted,

    /// The pin has a version or a date to compare, and the range does not
    /// accept it
    Outside,

    /// The pin has no version to compare with a range of releases
    Unreadable(semver::Error),

    /// The pin belongs to no channel that a range of dates compares, so the
    /// range says nothing about it
    Foreign,
}

impl Accepted {
    /// Returns the range that a text gives
    ///
    /// The text holds one or more alternatives, joined by `||`. Each
    /// alternative uses the syntax of a version requirement of Cargo. A text
    /// whose every comparator gives a date of one channel is a range of dates.
    /// Every other text is a range of releases.
    ///
    /// # Errors
    ///
    /// Returns the error of the reading when the text is no range, for
    /// example because an alternative is empty, or because one alternative
    /// gives dates and another gives releases.
    fn parse(range: &VersionRange) -> Result<Self, semver::Error> {
        let alternatives: Vec<&str> = range.get().split(ALTERNATIVE).collect();

        if let Some((channel, texts)) = dated_alternatives(&alternatives) {
            let dates = texts
                .iter()
                .map(|text| VersionReq::parse(text))
                .collect::<Result<_, _>>()?;

            return Ok(Self::Dated { channel, dates });
        }

        let releases = alternatives
            .iter()
            .map(|text| VersionReq::parse(text))
            .collect::<Result<_, _>>()?;

        Ok(Self::Releases(releases))
    }

    /// Returns what the range says about a pin
    ///
    /// A range of releases reads the version that the pin resolves to, so a
    /// pin such as `1.88` reads as `1.88.0`. A range of dates reads the pin
    /// as the configuration writes it. Mise can resolve a bare `nightly` to
    /// a dated version that one machine holds and another does not, so only
    /// a pin that writes the date counts on every machine.
    fn read(&self, pin: &str, requested: &str) -> Reading {
        match self {
            Self::Releases(accepted) => match Version::parse(pin) {
                Ok(version) if accepted.iter().any(|range| range.matches(&version)) => {
                    Reading::Accepted
                }
                Ok(_) => Reading::Outside,
                Err(source) => Reading::Unreadable(source),
            },
            Self::Dated { channel, dates } => match date_of(requested, channel) {
                Some(date) if dates.iter().any(|range| range.matches(&date)) => Reading::Accepted,
                Some(_) => Reading::Outside,
                None => Reading::Foreign,
            },
        }
    }

    /// Returns the details of a toolchain that the project does not pin for
    /// the range
    fn unpinned(&self) -> String {
        match self {
            Self::Releases(_) => NO_PROJECT_PIN.to_owned(),
            Self::Dated { channel, .. } => format!(
                "no configuration inside the project pins a dated `{channel}` version, such as \
                 `{channel}-2026-11-02`"
            ),
        }
    }
}

/// Returns the channel of a range of dates, and each alternative with each
/// date written as a version, when every comparator of every alternative
/// gives a date of one channel
///
/// An alternative without a date, or with another channel than the others,
/// makes the text no range of dates.
fn dated_alternatives(alternatives: &[&str]) -> Option<(String, Vec<String>)> {
    let mut channel: Option<String> = None;
    let mut texts = Vec::new();

    for alternative in alternatives {
        let (name, text) = dated(alternative)?;

        if channel.as_ref().is_some_and(|known| *known != name) {
            return None;
        }
        channel = Some(name);

        texts.push(text);
    }

    channel.map(|channel| (channel, texts))
}

/// Returns the channel of a range of dates, and the range with each date
/// written as a version, when every comparator of the text gives a date of
/// one channel
///
/// A comparator such as `>=nightly-2025-02-20` gives the channel `nightly`
/// and the date `2025.2.20`. The numbers of a version carry no leading
/// zeros, so the date loses them. A text with a comparator that gives no
/// date, or with two channels, is no range of dates.
fn dated(range: &str) -> Option<(String, String)> {
    let mut channel: Option<&str> = None;
    let mut comparators = Vec::new();

    for comparator in range.split(',') {
        let comparator = comparator.trim();
        let start = comparator
            .find(|character: char| character.is_ascii_alphabetic())
            .unwrap_or(comparator.len());
        let (operator, version) = comparator.split_at(start);
        let (name, date) = channel_date(version)?;

        if channel.is_some_and(|known| known != name) {
            return None;
        }
        channel = Some(name);

        comparators.push(format!("{operator}{date}"));
    }

    channel.map(|channel| (channel.to_owned(), comparators.join(", ")))
}

/// Returns the date of a pin of a channel, read as a version, when the pin
/// is a dated version of that channel
///
/// Rustup gives a dated toolchain the name of its channel and its date, such
/// as `nightly-2026-11-02`. A pin of another channel, and a pin without a
/// date, answer `None`.
fn date_of(pin: &str, channel: &str) -> Option<Version> {
    let (name, date) = channel_date(pin)?;

    if name != channel {
        return None;
    }

    Version::parse(&date).ok()
}

/// Returns the channel and the date of a dated name, with the date written
/// as a version
///
/// The name is a channel of letters, a hyphen, and a date in the form
/// `YYYY-MM-DD`, such as `nightly-2026-11-02`.
fn channel_date(name: &str) -> Option<(&str, String)> {
    let (channel, date) = name.split_once('-')?;

    if channel.is_empty() || !channel.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return None;
    }

    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };

    let year = number(year, 4)?;
    let month = number(month, 2)?;
    let day = number(day, 2)?;

    Some((channel, format!("{year}.{month}.{day}")))
}

/// Returns the number that a part of a date gives, when the part has the
/// width of its place and only digits
fn number(part: &str, width: usize) -> Option<u32> {
    if part.len() != width || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    part.parse().ok()
}

/// Returns the pins of a toolchain that a configuration inside the project
/// holds
///
/// The list of mise gives the file of each pin, but it sorts the versions,
/// so the caller takes their order from another answer of mise.
///
/// # Errors
///
/// Returns [`MiseUnavailable`][unavailable] when mise does not run,
/// [`UnresolvedToolchain`][unresolved] when mise ends without success, and
/// [`UnrecognizedReport`][unrecognized] when the list is not the JSON that
/// the crate reads.
///
/// [unavailable]: ResolveRequirementError::MiseUnavailable
/// [unrecognized]: ResolveRequirementError::UnrecognizedReport
/// [unresolved]: ResolveRequirementError::UnresolvedToolchain
async fn project_pins(
    tool: &MiseTool,
    root: &ProjectRoot,
) -> Result<Vec<Pin>, ResolveRequirementError> {
    let listing = ask(tool, root, &[LIST, PINNED, JSON, tool.get()]).await?;

    if !listing.status().success() {
        return Err(ResolveRequirementError::UnresolvedToolchain {
            tool: tool.clone(),
            details: diagnosis(&listing),
        });
    }

    let pins: Vec<Pin> = serde_json::from_slice(listing.stdout().get()).map_err(|source| {
        ResolveRequirementError::UnrecognizedReport {
            tool: tool.clone(),
            source,
        }
    })?;

    let inside = canonical(root.get());

    let project: Vec<Pin> = pins
        .into_iter()
        .filter(|pin| {
            pin.source
                .as_ref()
                .and_then(|source| source.path.as_deref())
                .is_some_and(|path| canonical(path).starts_with(&inside))
        })
        .collect();

    Ok(project)
}

/// Runs mise with the arguments in the root of the project
///
/// # Errors
///
/// Returns [`MiseUnavailable`][unavailable] when mise does not run.
///
/// [unavailable]: ResolveRequirementError::MiseUnavailable
async fn ask(
    tool: &MiseTool,
    root: &ProjectRoot,
    arguments: &[&str],
) -> Result<Execution, ResolveRequirementError> {
    Invocation::new(MISE)
        .args(arguments.iter().copied())
        .in_directory(root.get())
        .run()
        .await
        .map_err(|source| ResolveRequirementError::MiseUnavailable {
            tool: tool.clone(),
            source,
        })
}

/// Returns the path with its links resolved, or the path as it is when it
/// does not resolve
///
/// Mise gives the file of a pin with its links resolved, such as the
/// temporary directory of macOS under `/private`, so the root of the project
/// needs the same form before a comparison.
fn canonical(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    // An assertion in a test panics by design. A `# Panics` section on every
    // test would repeat that and give the reader no information.
    #![allow(clippy::missing_panics_doc)]

    use super::*;

    /// Returns what a range says about a pin that the configuration writes
    /// as the version that it resolves to
    fn read(range: &str, pin: &str) -> Reading {
        Accepted::parse(&VersionRange::new(range))
            .expect("the test gives a valid range")
            .read(pin, pin)
    }

    #[test]
    fn dated_with_a_channel_that_holds_a_digit_is_no_range_of_dates() {
        let range = dated(">=n1ghtly-2025-02-20");

        assert_eq!(range, None);
    }

    #[test]
    fn dated_with_a_comparator_of_another_channel_is_no_range_of_dates() {
        let range = dated(">=nightly-2025-02-20, <beta-2026-01-01");

        assert_eq!(range, None);
    }

    #[test]
    fn dated_with_a_comparator_without_a_date_is_no_range_of_dates() {
        let range = dated(">=nightly-2025-02-20, <1.90");

        assert_eq!(range, None);
    }

    #[test]
    fn dated_with_dates_of_one_channel_writes_each_date_as_a_version() {
        let range = dated(">=nightly-2025-02-20, <nightly-2026-01-01");

        assert_eq!(
            range,
            Some(("nightly".to_owned(), ">=2025.2.20, <2026.1.1".to_owned()))
        );
    }

    #[test]
    fn dated_alternatives_of_two_channels_are_no_range_of_dates() {
        let range = dated_alternatives(&[">=nightly-2025-02-20", ">=beta-2026-01-01"]);

        assert_eq!(range, None);
    }

    #[test]
    fn parse_with_a_bare_channel_reports_an_invalid_range() {
        let range = Accepted::parse(&VersionRange::new("nightly"));

        assert!(range.is_err(), "expected an invalid range, got {range:?}");
    }

    #[test]
    fn parse_with_an_alternative_of_dates_beside_one_of_releases_reports_an_invalid_range() {
        let range = Accepted::parse(&VersionRange::new(">=nightly-2025-02-20 || >=1.85"));

        assert!(range.is_err(), "expected an invalid range, got {range:?}");
    }

    #[test]
    fn parse_with_an_empty_alternative_reports_an_invalid_range() {
        let range = Accepted::parse(&VersionRange::new(">=22 ||"));

        assert!(range.is_err(), "expected an invalid range, got {range:?}");
    }

    #[test]
    fn read_with_a_bare_channel_that_resolves_to_a_date_reports_it_foreign() {
        let range = Accepted::parse(&VersionRange::new(">=nightly-2025-02-20"))
            .expect("the test gives a valid range");

        let reading = range.read("nightly-2026-11-02", "nightly");

        assert!(matches!(reading, Reading::Foreign), "got {reading:?}");
    }

    #[test]
    fn read_with_a_date_in_a_range_of_dates_accepts_the_pin() {
        let reading = read(">=nightly-2025-02-20", "nightly-2026-11-02");

        assert!(matches!(reading, Reading::Accepted), "got {reading:?}");
    }

    #[test]
    fn read_with_a_date_in_the_second_alternative_of_dates_accepts_the_pin() {
        let reading = read(
            "<nightly-2025-01-01 || >=nightly-2026-01-01",
            "nightly-2026-11-02",
        );

        assert!(matches!(reading, Reading::Accepted), "got {reading:?}");
    }

    #[test]
    fn read_with_a_date_outside_a_range_of_dates_reports_it_outside() {
        let reading = read(">=nightly-2025-02-20", "nightly-2024-12-31");

        assert!(matches!(reading, Reading::Outside), "got {reading:?}");
    }

    #[test]
    fn read_with_a_dated_pin_in_a_range_of_releases_reports_it_unreadable() {
        let reading = read(">=1.85", "nightly-2026-11-02");

        assert!(matches!(reading, Reading::Unreadable(_)), "got {reading:?}");
    }

    #[test]
    fn read_with_a_pin_of_another_channel_in_a_range_of_dates_reports_it_foreign() {
        let reading = read(">=nightly-2025-02-20", "beta-2026-11-02");

        assert!(matches!(reading, Reading::Foreign), "got {reading:?}");
    }

    #[test]
    fn read_with_a_pin_whose_date_has_four_parts_reports_it_foreign() {
        let reading = read(">=nightly-2025-02-20", "nightly-2026-11-02-01");

        assert!(matches!(reading, Reading::Foreign), "got {reading:?}");
    }

    #[test]
    fn read_with_a_pin_whose_date_has_no_leading_zeros_reports_it_foreign() {
        let reading = read(">=nightly-2025-02-20", "nightly-2026-1-2");

        assert!(matches!(reading, Reading::Foreign), "got {reading:?}");
    }

    #[test]
    fn read_with_a_pin_whose_date_holds_a_sign_reports_it_foreign() {
        let reading = read(">=nightly-2025-02-20", "nightly-2026-+1-02");

        assert!(matches!(reading, Reading::Foreign), "got {reading:?}");
    }

    #[test]
    fn read_with_a_pin_without_a_date_in_a_range_of_dates_reports_it_foreign() {
        let reading = read(">=nightly-2025-02-20", "1.99.1");

        assert!(matches!(reading, Reading::Foreign), "got {reading:?}");
    }

    #[test]
    fn read_with_a_release_between_two_alternatives_reports_it_outside() {
        let reading = read("^22.18 || >=23.6", "23.5.0");

        assert!(matches!(reading, Reading::Outside), "got {reading:?}");
    }

    #[test]
    fn read_with_a_release_in_a_range_of_releases_accepts_the_pin() {
        let reading = read(">=1.85", "1.99.1");

        assert!(matches!(reading, Reading::Accepted), "got {reading:?}");
    }

    #[test]
    fn read_with_a_release_in_the_second_alternative_accepts_the_pin() {
        let reading = read("^22.18 || >=23.6", "23.6.0");

        assert!(matches!(reading, Reading::Accepted), "got {reading:?}");
    }

    #[test]
    fn read_with_a_release_outside_a_range_of_releases_reports_it_outside() {
        let reading = read(">=1.85", "1.84.1");

        assert!(matches!(reading, Reading::Outside), "got {reading:?}");
    }

    #[test]
    fn unpinned_of_a_range_of_dates_names_the_channel() {
        let range = Accepted::parse(&VersionRange::new(">=nightly-2025-02-20"))
            .expect("the test gives a valid range");

        assert!(range.unpinned().contains("`nightly`"));
    }
}
