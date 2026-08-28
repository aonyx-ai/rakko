# Rakko CLI

`rakko-cli` is the command-line projection of Rakko. It turns the actions that
a harness mounts into commands: the command tree, the help text of each
command, and the flags that every command shares. A harness depends on this
crate, and an action never does, so the command-line framework stays out of the
crate that every action depends on.

The crate is a placeholder. Until it holds the projection, this specification
has one requirement, so that the specification tooling has a crate to check.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Placeholder

cli[placeholder.add]
The crate MUST provide a function that returns the sum of two unsigned 64-bit
integers.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
