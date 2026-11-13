//! The name of a package of a workspace

typed_fields::name! {
    /// The name of a package of a workspace
    ///
    /// The name is the one that the manifest of the package states, such as
    /// `aonyx-rakko-cargo`. It is how a job selects the package that cargo
    /// works on. It is not the crate name of a target of the package. The crate name
    /// replaces every hyphen with an underscore, and the manifest can change
    /// it for each target.
    PackageName
}
