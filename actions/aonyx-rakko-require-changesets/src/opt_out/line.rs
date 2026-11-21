//! A line of the body that tries to opt out without a reason

typed_fields::name! {
    /// A line of the body of a pull request that starts an opt-out and gives
    /// no reason
    ///
    /// The value is the line without the spaces at its ends. The check fails
    /// with the line, so that the author sees which line opted nothing out.
    OptOutLine
}
