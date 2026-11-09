//! The range of versions of a toolchain that an action accepts

typed_fields::name! {
    /// The range of versions of a toolchain that an action accepts
    ///
    /// The text uses the syntax of a version requirement of Cargo, such as
    /// `>=22, <27`. A bare version such as `22.4` accepts every compatible
    /// version from `22.4.0` on, as it does in a manifest of Cargo.
    ///
    /// A toolchain can also have dated versions of a channel, such as the
    /// Rust nightly `nightly-2026-11-02`. A range of such versions uses the
    /// same syntax with a channel and a date in place of each version, such
    /// as `>=nightly-2025-02-20`. Every comparator gives the same channel,
    /// and the range accepts only the versions of that channel that a
    /// configuration writes with their date.
    ///
    /// A range can hold alternatives, joined by `||`, such as
    /// `^22.18 || >=23.6`. The range accepts a version that one of the
    /// alternatives accepts. The alternatives of a range of dated versions
    /// give the same channel.
    ///
    /// Nothing reads the text when the range is created. The resolution of a
    /// requirement reads it, and a text that is not a range stops the
    /// resolution with an error.
    VersionRange
}
