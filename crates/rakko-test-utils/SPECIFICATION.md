# Rakko Test Utils

`rakko-test-utils` holds what the tests of the other crates need and what none
of them should write twice. A crate depends on it as a development dependency,
so nothing that a project runs carries it.

The crate exists because a test compares a path, and a path reads differently
on each platform. Windows separates the parts of a path with one character
and every other platform with another, and Windows calls a path absolute only
when it names a volume as well. A test that wrote a path as a literal would
therefore describe one path on one platform and another path, or no absolute
path at all, on the next. Repeating each literal per platform would double
every test that names a file.

The answer is to write a literal once, in the one form that a reader knows,
and to build the path of the platform from its parts.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Paths

A literal names the parts of a path between separators, and the crate joins
those parts the way the platform joins them. A literal that opens with a
separator names an absolute path, and the crate opens the answer with what
the platform reads as the root of the file system.

testutils[path.relative]
The crate MUST return the path that the parts of a literal name, joined with
the separator of the platform.

testutils[path.absolute]
A literal that opens with a separator MUST produce a path that the platform
reads as absolute.

testutils[path.text]
The crate MUST return the text of such a path as well, because a program
writes a path as text, and a test that compares a report compares text.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
