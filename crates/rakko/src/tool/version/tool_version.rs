//! One exact version of a tool

typed_fields::name! {
    /// One exact version of a tool, as mise writes it
    ///
    /// The text is the version that a mise file pins, such as `0.10.0`. Mise
    /// reads it, so it follows the rules of mise for a version and not a rule
    /// of Rakko.
    ToolVersion
}
