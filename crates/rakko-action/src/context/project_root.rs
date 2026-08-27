typed_fields::path! {
    /// The root directory of the project that the action runs in
    ///
    /// A [`Context`](crate::Context) carries the project root, and the
    /// [`Layout`](crate::Layout) derives its defaults from it. All paths that
    /// an action reads or writes are relative to this directory.
    ProjectRoot
}
