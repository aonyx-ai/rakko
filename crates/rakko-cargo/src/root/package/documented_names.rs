/// Whether the documented targets of a package share a crate name with the
/// documented targets of another package
///
/// Cargo documents the targets of a workspace in parallel, and it writes the
/// documentation of a target to a directory that the crate name of the target
/// names. When a documented target of one package has the crate name of a
/// documented target of another package, two runs of rustdoc write one
/// directory at once, and one of them can fail. A caller that documents the
/// workspace documents such a package apart from the others.
///
/// The value answers for the documentation only. A target counts when cargo
/// documents it with every feature on, and a target that cargo does not
/// document writes no directory. Two targets of one package do not count
/// either: cargo skips a binary that carries the name of the library of its
/// own package, and the rarer collisions inside one package are left out. A
/// binary that stays out of the documentation can still share its name with
/// another target in the directory of a build, and this value says nothing
/// about that.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum DocumentedNames {
    /// No other package of the workspace documents a target under a crate
    /// name of the package
    Unique,

    /// Another package of the workspace documents a target under a crate
    /// name of the package
    Shared,
}
