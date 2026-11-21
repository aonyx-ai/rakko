//! The JSON that `otto list-units --json` writes

use std::path::PathBuf;

use serde::Deserialize;

use crate::release_unit::ReleaseUnit;
use crate::unit_listing::UnitListing;

/// The answer of Otto, with one key for each way that a command ends
///
/// The key `reached` holds the list, `stopped` holds a stop that an operator
/// repairs, and `error` holds a failure of the command.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Answer {
    /// Otto listed the units
    Reached(Listed),

    /// Otto stopped at a condition of the repository
    Stopped(Diagnostic),

    /// Otto could not reach a list
    Error(Diagnostic),
}

/// The list of the release units
#[derive(Debug, Deserialize)]
pub(super) struct Listed {
    /// The units, in the order that Otto lists them
    units: Vec<Unit>,

    /// The directories below the crates that belong to no unit
    unowned: Vec<PathBuf>,
}

impl Listed {
    /// Returns the listing of the units and of the unowned directories
    pub(super) fn into_listing(self) -> UnitListing {
        let Self { units, unowned } = self;

        UnitListing::builder()
            .units(units.into_iter().map(Unit::into_release_unit).collect())
            .unowned(unowned)
            .build()
    }
}

/// One release unit of the list
///
/// Otto also writes the scheme of a unit, and the name of each crate. The
/// check needs neither, so it reads only the fields below.
#[derive(Debug, Deserialize)]
struct Unit {
    /// The name of the unit
    name: String,

    /// The root of the unit, relative to the top of the repository
    root: PathBuf,

    /// The manifest that declares the version of the unit
    manifest: PathBuf,

    /// The crates that the unit releases
    crates: Vec<Crate>,
}

impl Unit {
    /// Returns the release unit that the entry of the list describes
    fn into_release_unit(self) -> ReleaseUnit {
        let Self {
            name,
            root,
            manifest,
            crates,
        } = self;

        ReleaseUnit::builder()
            .name(name)
            .root(root)
            .manifest(manifest)
            .crates(
                crates
                    .into_iter()
                    .map(|Crate { directory }| directory)
                    .collect(),
            )
            .build()
    }
}

/// One crate of a unit in the list
#[derive(Debug, Deserialize)]
struct Crate {
    /// The directory of the crate, relative to the top of the repository
    directory: PathBuf,
}

/// The message of a stop or of a failure
#[derive(Debug, Deserialize)]
pub(super) struct Diagnostic {
    /// The text that says what happened
    message: String,
}

impl Diagnostic {
    /// Returns the text of the diagnostic
    pub(super) fn message(&self) -> &str {
        &self.message
    }
}

/// Returns the answer of the JSON that Otto wrote
///
/// # Errors
///
/// Returns the error of [`serde_json`] when the text is no answer of Otto.
pub(super) fn answer(text: &str) -> Result<Answer, serde_json::Error> {
    serde_json::from_str(text)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    #[test]
    fn answer_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Answer>();
    }

    #[test]
    fn answer_of_a_failure_holds_its_message() {
        let answer = answer(
            r#"{"error":{"next_action":null,"message":"failed to find the top of the repository","help":null,"log":null}}"#,
        )
        .unwrap();

        assert!(
            matches!(answer, Answer::Error(diagnostic) if diagnostic.message() == "failed to find the top of the repository")
        );
    }

    #[test]
    fn answer_of_a_list_holds_each_unit_with_its_manifest_and_its_crates() {
        let answer = answer(
            r#"{"reached":{"units":[{"name":"rakko","scheme":"independent","root":"crates/rakko","manifest":"crates/rakko/Cargo.toml","crates":[{"name":"rakko","directory":"crates/rakko"}]}],"unowned":[]}}"#,
        )
        .unwrap();

        let Answer::Reached(listed) = answer else {
            panic!("expected a list, got {answer:?}");
        };
        assert_eq!(
            listed.into_listing().units(),
            &[ReleaseUnit::builder()
                .name("rakko")
                .root("crates/rakko")
                .manifest("crates/rakko/Cargo.toml")
                .crates(vec![PathBuf::from("crates/rakko")])
                .build()]
        );
    }

    #[test]
    fn answer_of_a_list_holds_the_unowned_directories() {
        let answer =
            answer(r#"{"reached":{"units":[],"unowned":["crates/rakko/tests/fixture"]}}"#).unwrap();

        let Answer::Reached(listed) = answer else {
            panic!("expected a list, got {answer:?}");
        };
        assert_eq!(
            listed.into_listing().unowned(),
            &[PathBuf::from("crates/rakko/tests/fixture")]
        );
    }

    #[test]
    fn answer_of_a_stop_holds_its_message() {
        let answer = answer(
            r#"{"stopped":{"reason":"two units","next_action":"resolve_duplicate_name","warnings":[],"message":"the release units in a/x and in b/x have the same name dup","help":"give one of the two crates a new name","log":null}}"#,
        )
        .unwrap();

        assert!(
            matches!(answer, Answer::Stopped(diagnostic) if diagnostic.message() == "the release units in a/x and in b/x have the same name dup")
        );
    }

    #[test]
    fn crate_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Crate>();
    }

    #[test]
    fn diagnostic_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Diagnostic>();
    }

    #[test]
    fn listed_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Listed>();
    }

    #[test]
    fn unit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<Unit>();
    }
}
