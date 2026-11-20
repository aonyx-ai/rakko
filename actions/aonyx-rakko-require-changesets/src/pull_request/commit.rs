//! A commit of a pull request

typed_fields::name! {
    /// The hash of a commit that the event of a pull request gives
    ///
    /// The event gives the head commit of the base branch and the head commit
    /// of the pull request. Git compares the two to find what the pull request
    /// changes.
    Commit
}
