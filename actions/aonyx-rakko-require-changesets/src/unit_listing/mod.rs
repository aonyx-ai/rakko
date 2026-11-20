//! The release units that Otto lists, and the rule that gives a file to one

use std::path::{Path, PathBuf};

use getset::Getters;

use crate::release_unit::ReleaseUnit;

/// The release units of a project, with the directories that no unit owns
///
/// Otto lists the units, and beside them the unowned directories. An unowned
/// directory is below a crate of a unit and holds a manifest. It holds no crate
/// of a unit, as for a fixture or an internal crate. Every path is relative to
/// the top of the repository.
#[derive(Clone, Eq, PartialEq, Debug, Default, bon::Builder, Getters)]
pub(crate) struct UnitListing {
    /// The units, in the order that Otto lists them
    #[builder(default)]
    #[getset(get = "pub(crate)")]
    units: Vec<ReleaseUnit>,

    /// The directories below the crates that belong to no unit
    #[builder(default)]
    #[getset(get = "pub(crate)")]
    unowned: Vec<PathBuf>,
}

impl UnitListing {
    /// Returns the unit that owns a file, or `None` when no unit owns it
    ///
    /// The rule is the one that Otto gives for its list. The manifest of a unit
    /// belongs to that unit, also when it is in no crate, as for a lockstep
    /// workspace. Any other file belongs to the nearest directory above it
    /// among the directories of the crates and the unowned directories. A
    /// crate directory gives its unit, and an unowned directory gives none. A
    /// file with no such directory above it belongs to no unit.
    ///
    /// The root of a unit does not decide. In a lockstep workspace at the top
    /// of a repository, every file is under the root. A change to `.github/`
    /// alone still changes no unit, when no crate sits at the top of the
    /// repository. The comparison uses the
    /// components of the paths and not their text, so the directory
    /// `crates/tool` is not above `crates/tool-extra/Cargo.toml`.
    pub(crate) fn owner(&self, path: &Path) -> Option<&ReleaseUnit> {
        let Self { units, unowned } = self;

        if let Some(unit) = units.iter().find(|unit| unit.manifest() == path) {
            return Some(unit);
        }

        let crates = units.iter().flat_map(|unit| {
            unit.crates()
                .iter()
                .map(move |directory| (directory.as_path(), Some(unit)))
        });
        let others = unowned.iter().map(|directory| (directory.as_path(), None));

        crates
            .chain(others)
            .filter(|(directory, _)| path.starts_with(directory))
            .max_by_key(|(directory, _)| directory.components().count())
            .and_then(|(_, unit)| unit)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Returns the name of the owner of a file in a listing, or `None`
    fn owner_of(listing: &UnitListing, path: &str) -> Option<String> {
        listing
            .owner(Path::new(path))
            .map(|unit| unit.name().get().to_owned())
    }

    /// Returns a listing of the independent crates `alpha` and `alpha-extra`,
    /// with a fixture below `alpha` that belongs to no unit
    fn independent() -> UnitListing {
        UnitListing::builder()
            .units(vec![
                ReleaseUnit::builder()
                    .name("alpha")
                    .root("crates/alpha")
                    .manifest("crates/alpha/Cargo.toml")
                    .crates(vec![PathBuf::from("crates/alpha")])
                    .build(),
                ReleaseUnit::builder()
                    .name("alpha-extra")
                    .root("crates/alpha-extra")
                    .manifest("crates/alpha-extra/Cargo.toml")
                    .crates(vec![PathBuf::from("crates/alpha-extra")])
                    .build(),
            ])
            .unowned(vec![PathBuf::from("crates/alpha/tests/fixture")])
            .build()
    }

    /// Returns a listing of a lockstep workspace at the top of a repository,
    /// with the crates `demo` and `demo-core`
    fn lockstep() -> UnitListing {
        UnitListing::builder()
            .units(vec![
                ReleaseUnit::builder()
                    .name("demo")
                    .root("")
                    .manifest("Cargo.toml")
                    .crates(vec![
                        PathBuf::from("crates/demo"),
                        PathBuf::from("crates/demo-core"),
                    ])
                    .build(),
            ])
            .build()
    }

    #[test]
    fn owner_of_a_file_at_the_top_of_a_lockstep_workspace_is_none() {
        let owner = owner_of(&lockstep(), "Cargo.lock");

        assert_eq!(owner, None);
    }

    #[test]
    fn owner_of_a_file_in_a_crate_is_its_unit() {
        let owner = owner_of(&independent(), "crates/alpha/src/lib.rs");

        assert_eq!(owner.as_deref(), Some("alpha"));
    }

    #[test]
    fn owner_of_a_file_in_a_crate_of_a_lockstep_workspace_is_the_workspace() {
        let owner = owner_of(&lockstep(), "crates/demo-core/src/lib.rs");

        assert_eq!(owner.as_deref(), Some("demo"));
    }

    #[test]
    fn owner_of_a_file_in_a_crate_with_a_longer_name_is_that_crate() {
        let owner = owner_of(&independent(), "crates/alpha-extra/src/lib.rs");

        assert_eq!(owner.as_deref(), Some("alpha-extra"));
    }

    #[test]
    fn owner_of_a_file_in_a_directory_that_extends_the_name_of_a_crate_is_none() {
        let listing = UnitListing::builder()
            .units(vec![independent().units()[0].clone()])
            .build();

        let owner = owner_of(&listing, "crates/alpha-extra/Cargo.toml");

        assert_eq!(owner, None);
    }

    #[test]
    fn owner_of_a_file_in_an_unowned_directory_is_none() {
        let owner = owner_of(&independent(), "crates/alpha/tests/fixture/Cargo.toml");

        assert_eq!(owner, None);
    }

    #[test]
    fn owner_of_a_file_outside_every_crate_is_none() {
        let owner = owner_of(&independent(), "Cargo.lock");

        assert_eq!(owner, None);
    }

    #[test]
    fn owner_of_a_workflow_of_a_lockstep_workspace_at_the_top_is_none() {
        let owner = owner_of(&lockstep(), ".github/workflows/ci.yml");

        assert_eq!(owner, None);
    }

    #[test]
    fn owner_of_the_manifest_of_a_lockstep_workspace_is_the_workspace() {
        let owner = owner_of(&lockstep(), "Cargo.toml");

        assert_eq!(owner.as_deref(), Some("demo"));
    }

    #[test]
    fn unit_listing_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<UnitListing>();
    }
}
