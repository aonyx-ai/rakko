//! The reason that an author gives for a missing changeset

typed_fields::name! {
    /// The reason that an author gives for a change without a changeset
    ///
    /// The reason is the text after the colon of an opt-out line, without the
    /// spaces at its ends. It is never empty, because a line without a reason
    /// opts nothing out. The check shows it beside each unit that it accepted
    /// without a changeset, so that the reviewer reads why.
    Reason
}
