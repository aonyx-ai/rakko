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

## Span

A span tells which range of a file a problem covers. A tool that reports a
range gives the position where the range starts and the position where it
ends, and the range can cross lines. The crate does not check that the end is
at or after the start, because a range that a tool reported backwards is a
defect in the action that reported it, and no reader of a span can repair it.

action[span.start]
A span MUST give the position where the range starts.

action[span.end]
A span MUST give the position where the range ends.

## Location

A location tells how precisely an action can place a problem in a project.
The tools that actions wrap do not agree on that precision. A formatter knows
a file and no line in it, a linter knows a line and a column, and a check of
the dependencies of a project knows no path at all. A location therefore
names the level that it speaks at, and each level carries what that level
knows and no more, so that a reader knows from the level alone what it can
show. Every path is relative to the project root, so that a reader and a code
host see the same path. A path that starts at the root of the file system
says nothing about the project.

action[location.project]
A location MUST have a level for a problem that belongs to the project and to
no path in it.

action[location.directory]
A location MUST have a level for a problem that belongs to a directory. This
level MUST give the path of that directory.

action[location.file]
A location MUST have a level for a problem that belongs to a file. This level
MUST give the path of that file.

action[location.position+2]
A location MUST have a level for a problem at one position in a file. This
level MUST give the path of that file and the position of the problem in it.

action[location.span]
A location MUST have a level for a problem that covers a range of a file.
This level MUST give the path of that file and the range that the problem
covers.

action[location.relative]
The crate MUST refuse a path that is absolute. The refusal MUST report the
path.

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

An outcome is the result of one action run. It has one of five states: the
action passed, the action changed the project, the action failed, the action
does not apply, or the action stopped. The machinery maps each state to output
and to an exit code. A scheduler runs actions in parallel, so an outcome
travels between threads.

An action that repairs a problem instead of reporting it ends in one of two of
those states. It changed the project when it repaired every problem that it
found, and it failed when a problem remains. Both states hold the repairs. A
run that rewrote a file changed the working tree of whoever started it, and a
result that says nothing about that change leaves them to find it in a diff. A
repair is a problem that the project no longer has, so it takes the same shape
as a finding.

action[outcome.passed]
An outcome MUST have a state for an action that examined the project and found
no problem.

action[outcome.changed]
An outcome MUST have a state for an action that found problems and repaired
all of them. This state MUST hold the repairs.

action[outcome.failed]
An outcome MUST have a state for an action that found problems. This state
MUST hold the findings.

action[outcome.repairs]
The state for an action that found problems MUST hold the repairs that the
action made, together with the problems that remain.

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

## Arguments

An action reads arguments when it runs, and each action defines their type. The
machinery parses what a user asked for without knowing that type, so the type
describes itself as data and builds itself from the values that the machinery
collected. Both halves belong to the type, because only the crate that defines
a type can generate code from its fields. The description says what an action
reads, not how a user writes it, because the shape of a command line is the
same for every action and belongs to the projection. The description holds
what every argument has, and it grows when an action needs to say more.

An argument has a name, a shape, and documentation, and it has all three. The
shape says what kind of value the argument holds, and the documentation is the
sentence that a user reads. The set of shapes is closed: every projection
renders every shape, so a shape that a projection cannot render stops the
build of that projection and not a run. A new shape is a breaking release of
this crate, and that is what the guarantee costs.

The name of an argument follows the rules that the name of an action follows.
A projection turns the name into the token that a user types, so a name that a
command line cannot carry is a defect in the action, and the build must refuse
it.

The description carries no default, and it does not say that an argument is
required. The values of a run record what the user said, and the argument set
decides what an absent value means, so a scheduler, a test, and a command line
read the rules of the action and not the rules of a parser.

action[args.schema]
An argument set MUST describe the arguments that it holds, in the order that
the action declares them. The description MUST be available without a value of
the argument set.

action[args.argument]
The description of one argument MUST give the name that identifies the
argument.

action[args.name]
The crate MUST refuse the name of an argument that does not satisfy the rules
for the name of an action. The refusal MUST report the rule that the name does
not satisfy.

action[args.literal]
The crate MUST make the name of an argument from a literal string at compile
time. A literal string that does not satisfy those rules MUST fail the build.

action[args.shape]
The description of one argument MUST give the shape of the value that the
argument holds. The shapes MUST be a value that is true or false, a whole
number, the path of a file or of a directory, and a line of text.

action[args.documentation]
The description of one argument MUST give the documentation of the argument.

action[args.values]
An argument set MUST build itself from the values that a run holds. The run
MUST give the value of an argument by the name of that argument, and MUST
report that it has none when it holds no value for a name.

action[args.unreadable]
Building an argument set MUST fail when a value that the argument set requires
is absent, and when a value does not convert to the type that it belongs to.
The failure MUST report the argument.

action[args.empty]
The crate MUST provide the description and the construction for an action that
reads no arguments. That description MUST hold no argument.

action[args.send]
An argument set MUST be safe to move to a different thread.

action[args.sync]
An argument set MUST be safe to share with a different thread.

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

## Erased Action

The `Action` trait carries the type of the arguments that an action reads, so
two actions have two types. A registry holds many actions at once and cannot
name a type for each of them, so it holds a view of an action that hides its
type. The view answers what the machinery needs without naming the action:
the name, the description of the arguments, and a run that takes the values of
a command. The conversion from those values into the arguments happens behind
the view, where the type is still known.

action[erased.name]
An erased action MUST give the name that identifies the action.

action[erased.arguments]
An erased action MUST give the description of the arguments that the action
reads.

action[erased.run]
An erased action MUST run the action from a context and from the values of a
run, and MUST produce the outcome of that run.

action[erased.unreadable]
An erased action that cannot build the arguments of the action from the values
MUST produce the outcome for an action that stopped, and that outcome MUST hold
the failure.

action[erased.total]
Every action MUST have an erased view, and the crate MUST provide that view
for all of them.

action[erased.object]
An erased action MUST be usable as a trait object, so that one collection holds
actions that have different types.

action[erased.send]
An erased action MUST be safe to move to a different thread.

action[erased.sync]
An erased action MUST be safe to share with a different thread.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
