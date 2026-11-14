//! The text of one line that a tool wrote

typed_fields::name! {
    /// The text of one line that a tool wrote
    ///
    /// The text holds no character that ends a line. Bytes that are not valid
    /// UTF-8 arrive as the replacement character, so a line of a tool that
    /// writes another encoding is still readable.
    ToolText
}
