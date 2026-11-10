//! The reason why an action did not examine a project

typed_fields::name! {
    /// The reason why an action did not examine a project
    ///
    /// A skipped outcome carries a reason that tells a reader why the action
    /// examined nothing. An example is a tool that had no file to examine.
    /// Another example is a tool without a build for the platform of the run.
    SkipReason
}
