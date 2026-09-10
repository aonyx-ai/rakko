# Rakko Rust

`rakko-rust` is the bundle that any Rust project mounts: the formatter, the
linter, the test runner, the checks that guard what the project depends on,
and the build of its internal documentation. A bundle is a crate that exports
a list of actions, so a project adopts the whole set with one dependency and
one line in its harness, and a release of the bundle rolls a change across
every project that mounts it.

Membership is the decision that this crate carries. Every Rust project formats,
lints, tests, audits what it depends on, and builds the documentation that its
own contributors read, whether the project publishes a library or ships a
binary. The three checks that guard the promises a published library makes —
its version floors, its minimum supported Rust version, and the examples in its
documentation — belong to the Rust library bundle instead, because a binary
makes none of those promises.

The bundle is therefore a core that a second bundle contains rather than a
bundle that a project mounts on its own. It contains no other bundle, and it
does not contain the baseline: the two families sit side by side, so a project
without Rust mounts the baseline alone and a Rust project mounts two names.

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

rust[actions]
The bundle MUST export the actions `build-internal-docs`,
`check-dependencies`, `check-latest-deps`, `check-unused-deps`, `format-rust`,
`lint-rust`, and `test-rust`.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
