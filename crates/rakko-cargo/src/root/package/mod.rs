/// Whether the documented targets of a package share a crate name
mod documented_names;
/// The name of a package of a workspace
mod name;

use getset::{CopyGetters, Getters};

pub use self::documented_names::DocumentedNames;
pub use self::name::PackageName;

/// One package of a workspace, as a root names it
///
/// A job selects the packages of a workspace by name, and a job that
/// documents the workspace also needs to know which packages it must
/// document apart from the others. A package carries both, so a caller reads
/// them from the root instead of asking cargo again.
#[derive(Clone, Eq, PartialEq, Hash, Debug, CopyGetters, Getters)]
pub struct CargoPackage {
    /// The name that the manifest of the package states
    #[getset(get = "pub")]
    name: PackageName,

    /// Whether the package shares a crate name with another package of the
    /// workspace
    #[getset(get_copy = "pub")]
    documented_names: DocumentedNames,
}

impl CargoPackage {
    /// Creates a package from its name and whether it shares a crate name
    /// with another package of the workspace
    pub fn new(name: PackageName, documented_names: DocumentedNames) -> Self {
        Self {
            name,
            documented_names,
        }
    }
}
