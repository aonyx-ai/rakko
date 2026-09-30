# Rakko TypeScript

`rakko-typescript` is the bundle that any TypeScript project mounts: the
formatter, the linter, the type checker, and the test runner. A bundle is a
crate that exports a list of actions, so a project adopts the whole set with
one dependency and one line in its harness, and a release of the bundle rolls
a change across every project that mounts it.

Membership is the decision that this crate carries. Every TypeScript project
formats, lints, type checks, and tests its code, whether it publishes a
package or ships an application. No check applies to only one kind of project
today, so the bundle has no variants. A variant can come with the first check
that only one kind of project runs.

The bundle contains no other bundle, and it does not contain the baseline: the
two sit side by side, so a TypeScript project mounts two names.

The bundle adds nothing to what it exports. It runs no action, it examines no
project, and it checks no tool, because each action already stops when the tool
that it needs is missing. It installs no package that the project depends on
either, because the package manager of the project does that, and a missing
package shows as a failure of the action that needs it.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Actions

The list that the bundle exports is what a harness mounts, so the names in it
are the commands that a project gains.

typescript[actions]
The bundle MUST export the actions `check-typescript`, `format-typescript`,
`lint-typescript`, and `test-typescript`.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
