# Rakko Action

`rakko-action` is the contract crate of Rakko. Every action and every harness
depends on it, so it carries only what all of them share. Today that is the
name that identifies an action, and the context that an action reads when it
runs.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Name

A name identifies an action. The command-line projection turns a name into a
subcommand, so a name holds only the characters that a subcommand can carry. A
registry also finds an action by its name, so two names that a reader sees as
the same must be the same value.

action[name.accepts]
The crate MUST accept a name that starts with a lowercase ASCII letter, that
ends with a lowercase ASCII letter or an ASCII digit, and that holds only
lowercase ASCII letters, ASCII digits, and hyphens between the two.

action[name.text]
A name MUST show the text that it was made from.

action[name.empty]
The crate MUST refuse a name that has no characters.

action[name.start]
The crate MUST refuse a name whose first character is not a lowercase ASCII
letter. The refusal MUST report that character.

action[name.character]
The crate MUST refuse a name that holds a character that is not a lowercase
ASCII letter, an ASCII digit, or a hyphen. The refusal MUST report that
character and its position.

action[name.hyphens]
The crate MUST refuse a name that holds two hyphens next to each other. The
refusal MUST report the position of the second hyphen.

action[name.end]
The crate MUST refuse a name whose last character is a hyphen.

## Layout

A layout tells an action where the directories of a project are. Each
directory has a default that comes from the project root. A project that keeps
a directory somewhere else gives the path instead, so that a layout can
describe an unusual project and a test can point a directory at a temporary
directory.

action[layout.config]
A layout MUST give the directory that holds the configuration of the tools of
a project. The default MUST be the `.config` directory in the project root.

action[layout.cache]
A layout MUST give the directory that an action writes disposable data to. The
default MUST be the `target/rakko` directory in the project root.

action[layout.override]
A layout MUST accept a path for a directory in place of the default of that
directory.

## Context

A context holds the data that an action reads when it runs. It stays small,
because every action receives it, and because an action that reads less is
easier to schedule.

action[context.root]
A context MUST give the root directory of the project that the action runs in.

action[context.layout]
A context MUST give the layout of that project.

action[context.derived]
A context that is made without a layout MUST get the layout that comes from
its project root.

action[context.send]
A context MUST be safe to move to a different thread.

action[context.sync]
A context MUST be safe to share with a different thread.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
