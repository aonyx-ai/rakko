//! The description of one action, one command, or one group

typed_fields::name! {
    /// The description of one action, one command, or one group
    ///
    /// The description is one sentence that tells a user what the action, the
    /// command, or the group does. A projection shows the sentence where a user looks
    /// for help, such as in the list of the commands of a harness.
    ///
    /// An author does not construct a description. The derive of
    /// [`Documented`] takes it from the first paragraph of the doc comment of
    /// the type.
    ///
    /// [`Documented`]: crate::Documented
    Description
}
