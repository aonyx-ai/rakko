typed_fields::name! {
    /// The version of a tool that an entry of a lockfile records
    ///
    /// The value is the text of the lockfile, such as `1.8.1` or
    /// `nightly-2026-10-11`. Mise resolves it, and the action compares it as
    /// text.
    Version
}
