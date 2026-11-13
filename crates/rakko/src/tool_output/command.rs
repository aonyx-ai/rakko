//! The command that started a tool

typed_fields::name! {
    /// The command that started a tool, as a reader reads it
    ///
    /// The text holds the program and its arguments, such as
    /// `cargo build --workspace`. It is for a person who reads the output of a
    /// run. No program parses it, and a run never starts it again.
    ToolCommand
}
