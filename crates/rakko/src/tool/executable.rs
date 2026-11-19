//! The name of a program that a tool provides

typed_fields::name! {
    /// The name of a program that a tool provides
    ///
    /// The name is the one that a contributor types in a terminal, such as
    /// `markdownlint`. One tool can provide several programs, so the action
    /// gives the program that it starts when it resolves the tool.
    Executable
}
