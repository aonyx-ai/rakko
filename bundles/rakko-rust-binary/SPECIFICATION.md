# Rakko Rust Binary

`rakko-rust-binary` is the bundle that a Rust project mounts when it ships a
binary: the formatter, the linter, the test runner, the checks that guard what
the project depends on, and the build of its internal documentation. A bundle
is a crate that exports a list of actions, so a project adopts the whole set
with one dependency and one line in its harness, and a release of the bundle
rolls a change across every project that mounts it.

Membership is the decision that this crate carries, and today it is the
membership of the core. A binary publishes no library, so the three checks that
guard the promises of a published library belong to `rakko-rust-library`
instead. Nothing else is left that only a binary runs.

The crate exists although it equals the core it contains. A binary project
names what it is when it mounts this bundle, and a check that only a binary
runs then reaches every such project as a release of this bundle, without an
edit to the harness of each one.

The bundle contains `rakko-rust`. It contains no other bundle, and it does not
contain the baseline: the two families sit side by side, so a project without
Rust mounts the baseline alone and a Rust project mounts two names.

A project mounts this bundle or `rakko-rust-library`, never both. Both carry
the core, and a harness stops when two actions have one name. A workspace that
publishes a library and ships a binary mounts the library bundle, which exports
every action of this one.

The bundle adds nothing to what it exports. It runs no action, it examines no
project, and it checks no tool, because each action already stops when the tool
that it needs is missing.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Actions

The list that the bundle exports is what a harness mounts, so the names in it
are the commands that a project gains.

rustbinary[actions]
The bundle MUST export every action of the bundle `rakko-rust`, and no other
action.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
