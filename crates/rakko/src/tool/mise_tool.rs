//! The identifier of a tool in the configuration of mise

typed_fields::name! {
    /// The identifier of a tool in the configuration of mise
    ///
    /// The identifier is the key of the tool in the `[tools]` table of a mise
    /// file, with the backend that installs it, such as `taplo` or
    /// `npm:markdownlint-cli`. It is not always the name of the program that
    /// the tool provides. For example, the tool `npm:markdownlint-cli`
    /// provides the program `markdownlint`.
    MiseTool
}
