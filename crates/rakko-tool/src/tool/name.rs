typed_fields::name! {
    /// The name of an external tool
    ///
    /// The name is the one that mise knows the tool by, which is the name of
    /// the program that a contributor types in a terminal. It is not the name
    /// of the package that installed the program: mise installs
    /// `markdownlint-cli` from npm, and the tool that it provides is
    /// `markdownlint`.
    ToolName
}
