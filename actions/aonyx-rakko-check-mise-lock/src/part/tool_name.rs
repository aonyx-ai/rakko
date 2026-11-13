//! The name of the tool of a part

typed_fields::name! {
    /// The name of a tool, as a lockfile of mise writes it
    ///
    /// The name is the key of the tool in the lockfile, such as `jq` or
    /// `cargo:cargo-deny`. It carries the backend when the configuration of
    /// the project names one.
    ToolName
}
