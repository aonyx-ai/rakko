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
builds a command tree of its own and hands it to Clawless. The parser behind
that tree is an implementation detail, and it appears in no signature that a
harness or an action can see.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Arguments

An action declares the arguments that it reads, and the projection turns each
of them into a flag. One projection builds the flags of every action, so a
user who learns the shape of one command knows the shape of all of them.

Every argument becomes a long flag that carries the name of the argument. The
projection gives no short flag, because one letter is a name that only one
argument in the whole fleet can hold, and no action has a claim on it.

The command line gives the action what the user wrote, as text. It tests that
a number is a number, so that a user who wrote something else reads the usage
message of the command line instead of an error from the action. Every other
rule that a value must satisfy belongs to the action, which reports it.

A flag that the user left out gives the action no value. The action decides
what an absent value means, so a run from a command line and a run from a
test read the same rules.

cli[argument.flag]
The command of an action MUST carry one long flag for each argument of that
action, and the name of the flag MUST be the name of the argument.

cli[argument.help]
The flag of an argument MUST show the documentation of the argument.

cli[argument.switch]
The flag of an argument that holds a value that is true or false MUST take no
value.

cli[argument.value]
The flag of an argument that holds a whole number, a path, or a text MUST take
a value, and MUST name that value after the argument.

cli[argument.integer]
The command line MUST refuse a value that is not a whole number for an
argument that holds a whole number.

cli[argument.values]
A run MUST give its action the value of every flag that the user gave. The
value of a flag that holds a value that is true or false MUST say that it is
true, and every other value MUST be the text that the user wrote.

cli[argument.absent]
A run MUST give its action no value for a flag that the user left out.

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

## Exit Code

The exit code is what a CI job reads without parsing anything, so it is the
contract between Rakko and every workflow that runs it. It answers one
question, and the answer has three meanings: the project is in order, the
project has a problem, or the run could not answer.

An action that does not apply exits clean. A skip is an answer, and a bundle
that mounts an action for a stack that a project does not use must not turn
that project red.

A run that repaired the project exits clean as well. The caller asked the run
to repair what it finds, and the run delivered exactly that, so a step that
runs a fix in CI must not go red over its success. The code loses nothing: the
repairs travel in the report, and the change sits in the working tree, which
is what a hook that guards a commit reads anyway. A run that repaired some of
what it found still has a problem, and it takes the code for a problem.

A run that could not answer takes the code that the parser already gives to a
command line that it cannot read. A run that never reached an action and a run
whose action stopped are the same event for whoever reads the result, so one
code covers both.

cli[exit.clean]
A run whose action passed, and a run whose action does not apply, MUST exit
with zero.

cli[exit.changed+2]
A run whose action repaired the project MUST exit with zero.

cli[exit.findings]
A run whose action found problems MUST exit with a nonzero code of its own.

cli[exit.unanswered]
A run whose action stopped MUST exit with a nonzero code that differs from the
code for findings.

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

The command line carries flags of its own, and a user reaches every one of
them in the command of an action. An action that declares an argument with the
name of such a flag takes a name that means something else in every other
command of the fleet, and the command line cannot carry both. That conflict is
a defect of the action, and the mount stops it as well.

cli[mount.list]
The builder MUST accept a list of erased actions, and MUST give each action of
that list a command.

cli[mount.flat]
The command tree MUST be flat. The command of an action MUST hold no command.

cli[mount.collision]
Two mounted actions with one name MUST stop the harness where it mounts them.
The failure MUST report the name.

cli[mount.reserved]
A mounted action whose argument carries the name of a flag of the command line
MUST stop the harness where it mounts the action. The failure MUST report the
action and the argument.

## Project Root

Every action reads the project root from the context that a run gives it. The
layout of a project derives the configuration directory and the cache
directory from that root, and a finding reports its location relative to it,
so a run that names the wrong root reads the wrong files and reports paths
that no other tool in the project agrees with.

A project marks its root with `.config/rakko.toml`. A run tests whether that
entry exists and never reads it, so the rule stays right when the content of
the file changes and when the file has no content at all.

A user whose checkout is laid out in a way that Rakko does not expect names
the root of the run instead. A run that receives a root does not search for
one, because the user knows the answer that the search would look for.

A run that finds no root stops. A wrong root reads the wrong files and reports
paths that mean nothing, and it does all of that quietly, so a message that
names the missing file is worth more than an answer that nobody can trust.

cli[root.marker]
A run MUST take the project root from the first directory, at or above the
directory that the run starts in, that holds `.config/rakko.toml`.

cli[root.named]
A run whose user names the project root MUST take that root, and MUST NOT
search for one.

cli[root.unmarked]
A run that finds no project root MUST stop, and MUST report the file that
marks a root.

## Report

A run shows what its action found. The projection renders the outcome, and an
action writes to no stream, so every project of the fleet reports in one shape
and no action carries output code.

A finding renders on one line, and that line starts with the location that the
finding names. One line is what every granularity of a finding can produce,
and it is what a reader greps. A block of source under that line, with the
offending span marked, needs data that a finding does not carry.

A repair renders as a finding does, because it is the problem that the run
took away. A run that repaired some of the problems that it found shows its
repairs first. The problems that remain follow them, so that the lines a
reader has to act on sit next to the summary.

A pass shows what the run examined when the action said so, the way a skip
shows its reason. A pass that examined less than the reader expects points to
a misconfiguration, and the summary is the only place where that shows.

The result of a run is not an informational message, so the flags that reduce
the output do not suppress it. A run reports the same result at every
verbosity, and those flags reach the progress that a longer run reports later.

The JSON that a run emits carries no compatibility promise yet. It says so in
the payload, because the granularity of a finding is still open, and a schema
that promises the shape of today's finding would break when that shape
changes.

cli[report.findings]
A run whose action found problems MUST show every finding with its location.

cli[report.repairs]
A run whose action repaired the project MUST show every repair with its
location. A run whose action repaired some of the problems that it found MUST
show the repairs and the problems that remain.

cli[report.passed]
A run whose action passed with a summary MUST show that summary.

cli[report.skipped]
A run whose action does not apply MUST show the reason.

cli[report.errored]
A run whose action stopped MUST show the error.

cli[report.json]
A run MUST render its outcome as JSON when the user asks for JSON, and that
JSON MUST state that its schema is unstable.

## Run

A run drives one action. The command that the user named resolves to the action
that the harness mounted under that name, and the action gets the context that
it needs to examine the project.

Where the project root of that context comes from is the Project Root section
of this document.

What a run shows, and the code that it gives back to the process that started
it, are the Report and the Exit Code sections of this document.

cli[run.action]
A run MUST drive the action that its command names.

[clawless]: https://github.com/aonyx-ai/clawless
[inventory]: https://crates.io/crates/inventory
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
