//! One package of a workspace

mod documented_names;
mod name;
mod publication;

use bon::Builder;
use getset::{CopyGetters, Getters};

pub use self::documented_names::DocumentedNames;
pub use self::name::PackageName;
pub use self::publication::Publication;

/// One package of a workspace, as a root names it
///
/// A job selects the packages of a workspace by name. A job that documents the
/// workspace also needs to know which packages it must document apart from the
/// others. A job that asks a registry about the packages needs to know which
/// of them cargo can publish. A package carries all three, so a caller reads
/// them from the root instead of asking cargo again.
///
/// # Examples
///
/// ```
/// use aonyx_rakko_cargo::{CargoPackage, DocumentedNames, PackageName};
///
/// let package = CargoPackage::builder()
///     .name(PackageName::new("otter"))
///     .documented_names(DocumentedNames::Unique)
///     .build();
///
/// assert_eq!(package.name().get(), "otter");
/// ```
#[derive(Clone, Eq, PartialEq, Hash, Debug, Builder, CopyGetters, Getters)]
pub struct CargoPackage {
    /// The name that the manifest of the package states
    #[getset(get = "pub")]
    name: PackageName,

    /// Whether the package shares a crate name with another package of the
    /// workspace
    #[getset(get_copy = "pub")]
    documented_names: DocumentedNames,

    /// Whether the manifest of the package lets cargo publish it
    #[builder(default)]
    #[getset(get_copy = "pub")]
    publication: Publication,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_package_is_send_sync_and_unpin() {
        fn assert_send_sync_and_unpin<T: Send + Sync + Unpin>() {}

        assert_send_sync_and_unpin::<CargoPackage>();
    }
}
