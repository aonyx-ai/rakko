typed_fields::name! {
    /// The name of a package of a workspace
    ///
    /// The name is the one that the manifest of the package states, such as
    /// `rakko-cargo`, and it is how a job selects the package that cargo
    /// works on. It is not the crate name of a target of the package, which
    /// replaces every hyphen with an underscore and which the manifest can
    /// change for each target.
    PackageName
}
