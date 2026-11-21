//! A release unit of Otto, as the action reads it

mod name;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use getset::Getters;

pub(crate) use self::name::UnitName;

/// The directory under the root of a unit that holds its changesets
const CHANGESETS: &str = ".changeset";

/// The extension of the file of a changeset
const EXTENSION: &str = "md";

/// The file of the changeset directory that describes the format of a
/// changeset instead of a change
const README: &str = "README.md";

/// A part of a repository that Otto releases with one version
///
/// Otto finds the units, and the action takes each one as Otto lists it. A
/// unit has a name, which a line in the body of a pull request uses to opt it
/// out. Its root holds the `.changeset` directory with its changesets. Its
/// manifest declares its version, and its crates are the directories of the
/// crates that it releases. Every path is relative to the top of the
/// repository, and the root is empty for a unit at the top. The listing of
/// the units decides which unit owns a file.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, bon::Builder, Getters)]
pub(crate) struct ReleaseUnit {
    /// The name of the unit, which is also the name in the tags of its releases
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    name: UnitName,

    /// The root of the unit, relative to the top of the repository
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    root: PathBuf,

    /// The manifest that declares the version of the unit, relative to the top
    /// of the repository
    #[builder(into)]
    #[getset(get = "pub(crate)")]
    manifest: PathBuf,

    /// The directories of the crates of the unit, relative to the top of the
    /// repository
    #[getset(get = "pub(crate)")]
    crates: Vec<PathBuf>,
}

impl ReleaseUnit {
    /// Returns whether a file of the repository is a changeset of the unit
    ///
    /// A changeset is a file with the extension `md` directly in the
    /// `.changeset` directory of the root, as Otto reads it. The `README.md`
    /// of that directory describes the format, and it is no changeset.
    pub(crate) fn is_changeset(&self, path: &Path) -> bool {
        path.parent() == Some(self.root.join(CHANGESETS).as_path())
            && path.extension() == Some(OsStr::new(EXTENSION))
            && path.file_name() != Some(OsStr::new(README))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_panics_doc,
        reason = "An assertion in a test panics by design. A `# Panics` section on every test would repeat that and give the reader no information."
    )]

    use super::*;

    /// Returns a unit of the given root
    fn unit(root: &str) -> ReleaseUnit {
        ReleaseUnit::builder()
            .name("alpha")
            .root(root)
            .manifest(Path::new(root).join("Cargo.toml"))
            .crates(vec![PathBuf::from(root)])
            .build()
    }

    #[test]
    fn is_changeset_of_a_file_below_the_changeset_directory_is_false() {
        let is_changeset =
            unit("crates/alpha").is_changeset(Path::new("crates/alpha/.changeset/old/add-it.md"));

        assert!(!is_changeset);
    }

    #[test]
    fn is_changeset_of_a_file_without_the_extension_is_false() {
        let is_changeset =
            unit("crates/alpha").is_changeset(Path::new("crates/alpha/.changeset/add-it.txt"));

        assert!(!is_changeset);
    }

    #[test]
    fn is_changeset_of_a_markdown_file_in_the_changeset_directory_is_true() {
        let is_changeset =
            unit("crates/alpha").is_changeset(Path::new("crates/alpha/.changeset/add-it.md"));

        assert!(is_changeset);
    }

    #[test]
    fn is_changeset_of_a_markdown_file_of_a_unit_at_the_top_is_true() {
        let is_changeset = unit("").is_changeset(Path::new(".changeset/add-it.md"));

        assert!(is_changeset);
    }

    #[test]
    fn is_changeset_of_the_changeset_of_another_unit_is_false() {
        let is_changeset =
            unit("crates/alpha").is_changeset(Path::new("crates/beta/.changeset/add-it.md"));

        assert!(!is_changeset);
    }

    #[test]
    fn is_changeset_of_the_readme_is_false() {
        let is_changeset =
            unit("crates/alpha").is_changeset(Path::new("crates/alpha/.changeset/README.md"));

        assert!(!is_changeset);
    }

    #[test]
    fn release_unit_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<ReleaseUnit>();
    }
}
