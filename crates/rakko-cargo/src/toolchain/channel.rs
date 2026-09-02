typed_fields::name! {
    /// The channel of a Rust toolchain that a project pins
    ///
    /// The channel is the name that a project writes in its `mise.toml`,
    /// such as `nightly`. Mise installs a channel as a dated toolchain, and
    /// rustup knows the toolchain by that date, so a channel is the question
    /// and a [`Toolchain`] is the answer.
    ///
    /// [`Toolchain`]: crate::Toolchain
    Channel
}
