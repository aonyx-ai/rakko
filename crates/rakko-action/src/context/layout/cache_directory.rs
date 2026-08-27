typed_fields::path! {
    /// The directory that an action writes disposable data to
    ///
    /// The default cache directory is the `target/rakko` directory in the
    /// project root. An action does not need to clean this directory up; the
    /// harness or the user manages its lifecycle.
    CacheDirectory
}
