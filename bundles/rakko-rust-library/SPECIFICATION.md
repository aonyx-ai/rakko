# Rakko Rust Library

`rakko-rust-library` is the bundle that a Rust project mounts when it publishes
a library: everything that any Rust project runs, and the checks that guard the
promises a library makes to whoever depends on it. A bundle is a crate that
exports a list of actions, so a project adopts the whole set with one dependency
and one line in its harness, and a release of the bundle rolls a change across
every project that mounts it.

Membership is the decision that this crate carries. A published library states
a floor for each of its dependencies, a `rust-version` that its consumers read
as a fact, and examples in its documentation that whoever adopts it copies.
Each of the three is a promise that only a check can keep honest, and a project
that ships a binary makes none of them. The three therefore belong here and not
to the core.

The bundle contains `rakko-rust`, because a library is a Rust project first and
formats, lints, tests, and audits what it depends on like any other. It
contains no other bundle, and it does not contain the baseline: the two
families sit side by side, so a project without Rust mounts the baseline alone
and a Rust project mounts two names.

A project mounts this bundle or `rakko-rust-binary`, never both. Both carry the
core, and a harness stops when two actions have one name. A workspace that
publishes a library and ships a binary mounts this bundle, which exports every
action of the other one.

The bundle adds nothing to what it exports. It runs no action, it examines no
project, and it checks no tool, because each action already stops when the tool
that it needs is missing.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Actions

The list that the bundle exports is what a harness mounts, so the names in it
are the commands that a project gains. The core comes first in the list,
because the bundle adds its own actions to the list of the core.

rustlibrary[actions]
The bundle MUST export every action of the bundle `rakko-rust`, and then the
actions `check-minimal-deps`, `check-msrv`, and `test-rust-docs`.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
