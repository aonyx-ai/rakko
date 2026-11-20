//! The body of a pull request

typed_fields::name! {
    /// The text that the author of a pull request wrote below its title
    ///
    /// The body holds the opt-out lines of the check. It is empty when the
    /// author wrote none.
    Body
}
