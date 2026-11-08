typed_fields::name! {
    /// The range of versions of a toolchain that an action accepts
    ///
    /// The text uses the syntax of a version requirement of Cargo, such as
    /// `>=22, <27`. A bare version such as `22.4` accepts every compatible
    /// version from `22.4.0` on, as it does in a manifest of Cargo.
    ///
    /// Nothing reads the text when the range is created. The resolution of a
    /// requirement reads it, and a text that is not a range stops the
    /// resolution with an error.
    VersionRange
}
