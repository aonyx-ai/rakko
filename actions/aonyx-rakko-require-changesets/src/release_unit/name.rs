//! The name of a release unit

typed_fields::name! {
    /// The name of a release unit of Otto
    ///
    /// Otto gives each unit a name that no other unit of the repository has,
    /// because the tag of a release starts with it. A line in the body of a
    /// pull request uses the name to opt the unit out of the check. The
    /// release branch of Otto is `release/` and the name.
    UnitName
}
