# Rakko Check TypeScript

`rakko-check-typescript` provides the action that type checks the TypeScript of
a project with [tsc]. The action wraps the tsc that mise pinned for the
project, so a run agrees with the editor and with a contributor that runs tsc
bare. Tsc reads the `tsconfig.json` of the project, collects the files that the
configuration selects, and reports every rule of the language that the code
breaks. The action names the project and translates what tsc reported into an
outcome.

Type checking is not linting. The compiler asks whether the program holds
together: whether a value fits the type that it is given, whether a call
matches the signature that it names, and whether a module that an import names
exists. A linter asks whether the code does something that the project turned a
rule against, and a file that answers this one can still answer that one badly.
The distinction matters more here than the name suggests, because a design that
rests on the compiler to reject a wrong composition has no guarantee at all
while no check runs the compiler.

A run emits nothing. The compiler is the tool that builds a project as well,
and a check that leaves output behind would write files that whoever started it
did not ask for. The action therefore asks tsc for the diagnosis and not for
the build.

The status of the process is not the answer on its own, and it is not ignored
either. Tsc ends without success for a project that holds a type error, for a
project whose configuration it refuses, and for a project that it found no
configuration for, and it writes a diagnostic for each of those, so the
diagnostics say which of them happened. A run that ends without success and
writes no diagnostic at all stopped for a reason that the action cannot name,
and that run stops the action instead of passing.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

checktypescript[name]
The action MUST identify itself as `check-typescript`.

## Arguments

The action reads no argument. A run only reports, because the compiler repairs
nothing: a type error states that the program disagrees with itself, and which
of the two sides is wrong is a question that only a contributor answers.

checktypescript[args.none]
The action MUST declare no argument.

## Tool

The action runs the tsc that mise installed for the project, at the version
that the project pinned, so a run reaches the same compiler as the editor and
the terminal of a contributor. The version matters more here than for most
tools, because a release of TypeScript adds rules of the language, and a
program that one version accepts is a program that the next one can refuse. A
tsc that mise does not report stops the action, because provisioning is the job
of mise, and the action installs nothing.

checktypescript[tool.tsc]
A run MUST resolve `tsc` through mise for the project of the run, and MUST run
the program that mise reports.

checktypescript[tool.missing]
A run whose tsc mise does not report MUST stop, and the outcome MUST hold the
error.

## Runs

Tsc compiles one project, and a project is a `tsconfig.json` together with
everything that it selects. The action names the root of the project, so a run
covers what a contributor covers when they start tsc bare in the root of their
checkout. A project that holds several configurations states the relation
between them in the one at its root, the way it states everything else about
itself, and the action asks about no other place.

A run writes nothing. The compiler emits the JavaScript of the project unless
it is told not to, and a check that emitted would leave files behind that
nobody asked for, in the directory that the configuration of the project names
for them.

The action asks for the diagnostics without the drawing. Tsc draws the source
of a diagnostic underneath it with the place marked in color when it writes to
a terminal, and it writes one line per diagnostic otherwise, so the report of a
run would depend on where the run happened. The plain form is the same
everywhere. This selects the presentation of the report and not the behavior of
the compiler.

checktypescript[run.project]
A run MUST name the root of the project to tsc, and no other place to look.

checktypescript[run.noemit]
A run MUST ask tsc to emit nothing.

checktypescript[run.plain]
A run MUST ask tsc for its diagnostics without the drawing, and MUST set no
other option of tsc.

## Configuration

The `tsconfig.json` of the project is the source of truth, and tsc reads it
without help from the action. The action selects no compiler option, so a
project that turns a check of the compiler off is not asked about it, and a
project decides for itself how strict its TypeScript is.

A configuration that tsc refuses is reported and not worked around. Tsc writes
a diagnostic about the configuration file, at the place in it that it could not
accept, and such a diagnostic reaches a reader the same way that a type error
does. A run that fell back to the defaults of the compiler instead would check
the project against rules that the project never asked for.

checktypescript[check.configuration]
A run MUST apply the compiler options that the configuration of the project
states.

## Skipping

The action applies to a project that tsc has something to compile in, and tsc
is what decides that. Two answers of tsc say that it has nothing: one for a
project that holds no `tsconfig.json` at its root, and one for a project whose
configuration selects no file. The first is a project that is not a TypeScript
project, and the second is a project that says its TypeScript is somewhere that
holds none.

checktypescript[skip.unconfigured]
A run whose tsc reports that it found no configuration file MUST report that
the action does not apply, and the reason MUST be what tsc wrote.

checktypescript[skip.uninhabited]
A run whose tsc reports that the configuration selected no input MUST report
that the action does not apply, and the reason MUST be what tsc wrote.

## Check

Tsc reads the files that the configuration selected and reports every rule of
the language that they break. Nothing of the source of the project changes,
whatever the run finds. A project that asked the compiler to remember its work
between runs gets the file that holds that memory written, because the
configuration of the project asked for it, and the action changes no file that
it examined.

Each diagnostic names the place, the kind, the number that TypeScript gives the
rule, and what the compiler said about it. The action reports each of them as a
finding at the position that tsc marked, and the message of the finding reads
like the line that tsc writes for a reader, so that a contributor reads the
answer of the compiler and not one that Rakko wrote about it.

A diagnostic explains itself in further lines where one sentence does not carry
the answer, and the compiler indents those lines underneath the first one. They
belong to the diagnostic above them, and they carry the part of the answer that
names which type disagreed with which, so the message of the finding holds them
as well.

A diagnostic names no file when it is about the run rather than about the code.
Such a diagnostic becomes a finding about the project, because there is no path
to report it at.

A diagnostic can also name a file outside the project root. The configuration
of a project can include a file from a parent directory, and tsc then writes a
path that starts with `..`. A finding names its path relative to the root, and
such a file has no path there. The diagnostic therefore becomes a finding about
the project, and the message names the file and the position, so that the
reader does not lose the place.

A run that ended without success and reported no diagnostic stops the action.
Tsc reports what it found, so such a run stopped for a reason of its own, and
the action states that instead of passing a project that the compiler never
checked. A report that the action cannot read stops the run for the same
reason.

checktypescript[check.read]
A run MUST NOT change a file that it examined.

checktypescript[check.passed]
A run whose tsc reports no diagnostic and ends with success MUST pass.

checktypescript[check.diagnostic+2]
A diagnostic that names a file below the project root MUST produce a finding
at the line and the column that tsc marked, and the message MUST hold the
kind, the number, and what tsc said about it.

checktypescript[check.elaboration]
A diagnostic that tsc explains in further lines MUST produce one finding, and
the message MUST hold what those lines say.

checktypescript[check.project]
A diagnostic that names no file MUST produce a finding about the project.

checktypescript[check.foreign]
A diagnostic that names a file outside the project root MUST produce a finding
about the project, and the message MUST hold the path, the line, and the
column that tsc wrote.

checktypescript[check.unreported]
A run whose tsc ended without success and reported no diagnostic MUST stop, and
the error MUST hold what tsc wrote.

checktypescript[check.unreadable]
A run whose report the action cannot read MUST stop, and the error MUST hold
what tsc wrote.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
[tsc]: https://www.typescriptlang.org/docs/handbook/compiler-options.html
