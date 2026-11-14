//! The code that a tool gave back when it ended

typed_fields::number! {
    /// The code that a tool gave back when it ended
    ///
    /// Zero means success for most tools. Each tool gives other codes a meaning
    /// of its own, so the code is for a reader and not for a decision.
    ToolExitCode
}
