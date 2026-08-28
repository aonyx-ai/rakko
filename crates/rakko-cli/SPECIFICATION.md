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

[clawless]: https://github.com/aonyx-ai/clawless
[inventory]: https://crates.io/crates/inventory
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
