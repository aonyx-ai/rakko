# Rakko CLI

`rakko-cli` is the command-line projection of Rakko. It turns the actions that
a harness mounts into commands: the command tree, the help text of each
command, and the flags that every command shares. A harness depends on this
crate, and an action never does, so the command-line framework stays out of the
crate that every action depends on.

The crate builds its command tree when the harness runs, and not when the
harness compiles. [Clawless] collects the commands of a binary with the
[inventory] crate at link time, and that collection does not reach a command
that another crate exported. A harness mounts such commands, so this crate
builds a command tree of its own and hands it to the chassis. The parser behind
that tree is an implementation detail, and it appears in no signature that a
harness or an action can see.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Builder

A harness builds its command line with a builder. The harness creates the
builder, names what the project mounts, and then runs it. The builder is the
whole surface that a harness touches, so a project adopts Rakko in a few lines
of naming.

cli[builder.create]
The crate MUST provide a function that creates a builder.

cli[builder.run]
A builder MUST run the command line that it describes, and MUST report a run
that the command line cannot read.

## Command Line

One projection builds the command line of every project in the fleet, so every
project gets the same shape. A run names the action that it wants, and flags
that every action shares control what the run shows. An action names none of
this, because uniform output is what the projection exists for.

cli[command.action]
The command line MUST refuse a run that names no action.

cli[command.help]
The command line MUST show its help for a run that gives no argument.

cli[command.output]
The command line MUST carry the flags that control the output of a run.

## Mount

A harness mounts the actions that a project uses. It passes lists, and a list
is an ordinary value that comes from a bundle, from another list, or from the
code of the harness. Reading the harness therefore reports what the project
runs, at the versions that Cargo resolved.

The command tree is flat. A name identifies an action, and nothing groups
actions today, so the command of an action sits directly under the command
line.

Two lists can carry one action, and a bundle that holds another bundle makes
such an overlap ordinary. A name must mean one action, and a harness that
mounts one name twice has a defect that only a change of its own code corrects.
The mount is therefore where the conflict stops.

cli[mount.list]
The builder MUST accept a list of erased actions, and MUST give each action of
that list a command.

cli[mount.flat]
The command tree MUST be flat. The command of an action MUST hold no command.

cli[mount.collision]
Two mounted actions with one name MUST stop the harness where it mounts them.
The failure MUST report the name.

## Run

A run drives one action. The command that the user named resolves to the action
that the harness mounted under that name, and the action gets the context that
it needs to examine the project.

The project root of that context is the directory that the user ran the command
from. A user that runs the command from a subdirectory of a project therefore
gets that subdirectory as the root, and finding the root of a project is a task
of its own.

A run shows nothing. The outcome of an action has no path to the output of the
command line yet, so a run reports neither what an action found nor that it
found nothing.

cli[run.action]
A run MUST drive the action that its command names.

[clawless]: https://github.com/aonyx-ai/clawless
[inventory]: https://crates.io/crates/inventory
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
