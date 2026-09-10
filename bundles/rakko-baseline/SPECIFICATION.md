# Rakko Baseline

`rakko-baseline` is the bundle that any project mounts: the formatters and the
linters for the files that a repository holds, whatever language it is written
in. A bundle is a crate that exports a list of actions, so a project adopts the
whole set with one dependency and one line in its harness, and a release of the
bundle rolls a change across every project that mounts it.

Membership is the decision that this crate carries. JSON, Markdown, TOML, YAML,
and the workflows of GitHub Actions are the files that a repository holds
before it holds a line of code, so they belong to every project of the fleet.
Nothing that belongs to one language belongs here. The bundle therefore
contains no other bundle, and no bundle contains it: a project without Rust
mounts it alone, and a Rust project mounts it beside a Rust bundle.

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

baseline[actions]
The bundle MUST export the actions `format-json`, `format-markdown`,
`format-toml`, `format-yaml`, `lint-github-actions`, `lint-markdown`,
`lint-toml`, and `lint-yaml`.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
