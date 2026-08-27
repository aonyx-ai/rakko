# Rakko Action

`rakko-action` is the contract crate of Rakko. Every action and every harness
depends on it, so it carries only what all of them share. Today that is the
`Action` trait that every action implements, the name that identifies an
action, the context that an action reads when it runs, and the outcome that
the run returns.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Name

A name identifies an action. The command-line projection turns a name into a
subcommand, so a name holds only the characters that a subcommand can carry. A
registry also finds an action by its name, so two names that a reader sees as
the same must be the same value. An action states its name in its code, so a
name that does not satisfy the rules is a defect in the action, and the build
must refuse it.

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

action[name.literal]
The crate MUST make a name from a literal string at compile time. A literal
string that does not satisfy the rules for a name MUST fail the build.

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

## Position

A position tells where in a file a problem is. The first line of a file is
line 1, and the first column of a line is column 1, because that is what an
editor and a code host show. A column has no meaning without a line, so a
position always has a line.

action[position.line]
A position MUST give the line that the problem is on.

action[position.column]
A position MUST give the column that the problem is at. A position that was
made without a column MUST report that it has none.

## Location

A location tells where a problem is in a project. It always names a file, and
it can add a position in that file. The path of that file is relative to the
project root, so that a reader and a code host see the same path. A path that
starts at the root of the file system says nothing about the project.

action[location.path]
A location MUST give the path of the file that the problem is in.

action[location.relative]
The crate MUST refuse a path that is absolute. The refusal MUST report the
path.

action[location.position]
A location MUST give the position of the problem in that file. A location that
was made without a position MUST report that it has none.

## Finding

A finding is one problem that an action found in a project. Findings travel in
the outcome of an action run, and the machinery shows them to a reader or to a
machine. A finding says what the problem is and where it is. It says nothing
about how it looks, because the shape of the output belongs to the machinery.

action[finding.message]
A finding MUST give a message that describes the problem.

action[finding.location]
A finding MUST give the location of the problem.

## Outcome

An outcome is the result of one action run. It has one of four states: the
action passed, the action failed, the action does not apply, or the action
stopped. The machinery maps each state to output and to an exit code. A
scheduler runs actions in parallel, so an outcome travels between threads.

action[outcome.passed]
An outcome MUST have a state for an action that examined the project and found
no problem.

action[outcome.failed]
An outcome MUST have a state for an action that found problems. This state
MUST hold the findings.

action[outcome.skipped]
An outcome MUST have a state for an action that does not apply to the project.
This state MUST hold the reason why the action does not apply.

action[outcome.errored]
An outcome MUST have a state for an action that stopped before it got a
result. This state MUST hold the error that stopped the action.

action[outcome.send]
An outcome MUST be safe to move to a different thread.

action[outcome.sync]
An outcome MUST be safe to share with a different thread.

## Action

An action is the unit of maintenance work. The `Action` trait is the contract
between an action and the machinery that runs it. An action describes itself
as data, so that the machinery can present it without running it. A registry
holds many actions and a scheduler runs them in parallel, so an action
travels between threads.

action[action.name]
An action MUST give the name that identifies it.

action[action.args]
An action MUST define the type of the arguments that a run of the action
reads. That type MUST be safe to move to a different thread and safe to share
with a different thread.

action[action.send]
An action MUST be safe to move to a different thread.

action[action.sync]
An action MUST be safe to share with a different thread.

## Run

A run is one execution of an action against a project. A run waits most of
its time, for example on the subprocess of an external tool, and a scheduler
drives many runs at the same time. A run that holds its thread while it waits
would block every other run on that thread.

action[run.outcome]
A run MUST produce the outcome of the action from a context and from the
arguments of the action.

action[run.wait]
A run MUST be able to return control to the thread that drives it before the
outcome exists. The run MUST produce the outcome when the thread drives it
again.

action[run.send]
A run MUST be safe to move to a different thread.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
