# Rakko

`rakko` is a placeholder for the crate that will tie the toolkit together.
Until then, this specification has one requirement, so that the specification
tooling has a crate to check.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Placeholder

rakko[placeholder.add]
The crate MUST provide a function that returns the sum of two unsigned 64-bit
integers.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
