# Rakko Tool

`rakko-tool` finds and runs the external tools that actions wrap. An action
that wraps a tool depends on this crate next to the contract crate, so the
machinery of a subprocess is written once and every action states only what it
wants to run.

The crate answers two questions, and it keeps both answers in one value. Where
is the tool? Mise installed it at the version that the project pinned, so the
crate asks mise instead of searching a path of its own. How does a run start
it? At the location that mise gave, in the root of the project, and with the
arguments of the action as the action wrote them.

The crate installs nothing. A tool that mise did not install stops the action
with an error, because provisioning is the job of mise, and a tool that Rakko
installed would run at a version that nothing pinned.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Resolution

Resolution turns the name of a tool into the location of a program. The name
is the one that a project writes in its `mise.toml` and that a contributor
types in a terminal. The location is the file that mise installed for it.

Mise answers, and nothing else does. The path of the process that started the
harness holds whatever the shell of a contributor set up, and that is a
different program on every machine. A run that read the path would check a
project against the version of whoever started it, and the result would change
with the shell instead of with the pin.

Mise reads the configuration of a project, so its answer belongs to a
directory. Resolution therefore asks about the project whose root the caller
names, and a run from a subdirectory reaches the same program as a run from
the root.

tool[resolve]
The crate MUST report the location of the program that mise installed for the
name of a tool.

tool[resolve.root]
Resolution MUST ask mise about the project whose root the caller names, and
MUST NOT read the directory that the process runs in.

tool[resolve.missing]
Resolution MUST report an error that names the tool when mise reports no
location for it. The crate MUST NOT install the tool.

## Runs

A resolved tool describes the command that runs it. An action adds the
arguments of the operation that it wants, starts the command, and reads what
the tool reported. The description starts nothing, so an action can write the
command to a log or name it in an error before a run exists.

An exit status that is not a success is the answer of the tool and not a
failure of the run. A formatter that finds a file to format ends without
success, and that is what the action asked it. The status therefore travels in
the result of the run, and the crate keeps it out of its errors.

tool[run.program]
A run MUST start the program that resolution reported.

tool[run.root]
A run MUST start the tool in the root of the project.

tool[run.vector]
A run MUST give the tool every argument as the action wrote it, and MUST NOT
let a shell read the command. Nothing splits an argument at a space, removes a
quotation mark, or expands a pattern.

tool[run.capture]
A run MUST report the exit status of the tool, what the tool wrote to its
standard output, and what the tool wrote to its standard error. An exit status
that is not a success MUST NOT be an error of the run.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
