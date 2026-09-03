# Rakko Taplo

`rakko-taplo` carries the machinery that the actions which wrap [taplo] share.
Taplo does several jobs of project maintenance — it formats TOML files, and it
validates them — and one action wraps each job. Every one of those actions
asks the same three questions, so this crate answers them once: does the
project hold TOML files, which taplo runs here, and what did that taplo
report?

The crate reads the report of taplo as text, because taplo offers nothing
structured for it. The shape of that text belongs to a version of taplo, and
one place that reads it is one place that a new version can break. The pin
softens the risk further: a new shape arrives with a new version, a new
version arrives with a pull request, and a report that the crate does not
recognize stops the caller instead of passing quietly, so the drift shows as a
red pull request.

The crate judges nothing. It reports what taplo said, and the action that
asked for the run decides what the answer means for its outcome.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Look

The look tells whether taplo has anything to do in a project. It is cheap, and
it runs before the tool resolves, so that a broad bundle stays safe: a project
without TOML files and without a taplo skips visibly instead of stopping over
a tool that it has no reason to install.

The look mirrors the discovery of taplo where mirroring is cheap. It matches
the `.toml` extension with the case that taplo matches, and it reads hidden
directories, because taplo reads them. It does not read the `.git` entry,
which holds no file of the project, and it follows no symbolic link, so that a
cycle of links cannot trap it.

The look and taplo can still disagree at the margins, because a configuration
can exclude every file that the look found. A caller that reaches taplo
therefore reports what taplo saw.

taplo[look.toml]
The crate MUST report whether the project holds a file with the `.toml`
extension, below the root that the caller names.

taplo[look.git]
The look MUST NOT read the `.git` entry of the project.

taplo[look.links]
The look MUST NOT follow a symbolic link.

taplo[look.unreadable]
A directory that the look cannot read MUST count as a directory that holds
TOML files. A look that cannot prove absence must not hide a real check behind
a skip.

## Tool

The taplo that runs is the taplo that mise installed for the project, at the
version that the project pinned, so a run reaches the same program as the
editor and the terminal of a contributor. The crate installs nothing:
provisioning is the job of mise, and a taplo that Rakko installed would run at
a version that nothing pinned.

taplo[tool.resolve]
The crate MUST resolve `taplo` through mise for the project whose root the
caller names, and a run MUST start the program that mise reports.

taplo[tool.missing]
The crate MUST report an error that names the tool when mise reports no taplo,
and MUST NOT install taplo.

## Runs

A caller names the operation that it wants, and the crate writes the command
line. One place holds the command line of every operation, so an action states
a job and never a flag, and a change to how taplo is called reaches every
action at once.

The crate asks taplo for a report without color codes, because it reads the
report as data and a color code inside a path would corrupt the reading. This
selects the presentation of the report and not the behavior of the tool: what
taplo does to a project comes from the configuration of that project alone.

Taplo can lose the end of its report when it exits, and the loss reaches
whatever taplo wrote last. The answer of a run survives it: the exit status
carries whether taplo found anything, a formatting run names the files that it
would rewrite on its standard output stream, and a file that taplo read and
could not accept gets a diagnostic that the loss does not reach. A run that
ends without success therefore names at least one problem, and a report of
such a run that names none arrived incomplete.

The count of the files is the one part of a report that no other stream
carries, and a run that has nothing left to do after it counts loses the count
most often. A caller reports a pass without the count, because a run that
ended with success found nothing whatever its report lost.

The crate starts a run again, a few times, for a report that holds no answer,
and again, fewer times, for one that lost only the count. Repeating is safe for
every operation, because a report reads the project again, and a rewrite
formats files that a previous attempt already formatted.

taplo[run.operation]
Each operation MUST run the subcommand and the flags of taplo that do that
job, and no other option of taplo.

taplo[run.plain]
Every run MUST ask taplo for a report without color codes.

taplo[run.complete+2]
A report of a run that ended without success MUST count as complete only when
it names at least one problem. A run whose report never arrives complete MUST
report an error that holds what taplo wrote. A run that ended with success MUST
count as complete, with or without the count of its files.

## Report

Taplo writes its report to two streams, and the crate reads the lines of each
that carry an answer. Everything else is ignored, so a log line that a new
version adds does not break the reading, and what the reading cannot find is
absent from the answer.

A formatting run that reports instead of rewriting prints the difference that
it would write for each file that is not formatted, on its standard output
stream. The crate reads the header of each difference, which names the file,
and ignores the text below it. The stream that carries the differences does not
lose lines, so the answer of such a run arrives whole.

A configuration file that taplo rejects is part of the answer. Taplo warns
about a configuration that it cannot read and then runs with its defaults, and
a run on the defaults quietly does what the project asked it not to do, so the
warning travels to the caller, which decides that the run has ended.

Taplo names a problem as precisely as its operation allows. A file that a
formatting run would rewrite gets a path and nothing else. A file that a
validating run refused gets a path and a reason, such as the message of the
operating system for a file that taplo could not open. A file that taplo read
and could not accept gets a line, a column, and a message. Each problem
travels at the level that taplo could name.

A validating run also sums the diagnostics of a file up in a line of its own.
The summary says less than the diagnostics above it, so it survives only for a
file that got no diagnostic at all, where it is the whole answer.

taplo[report.configuration]
The crate MUST report what taplo said about a configuration file that taplo
rejected.

taplo[report.checked+2]
The crate MUST report how many files a run examined, which is the count of the
files that taplo matched without the files that its configuration excluded,
when taplo reported that count.

taplo[report.unformatted+2]
A file whose difference a formatting run prints MUST become a problem that
names the file and no position in it.

taplo[report.invalid]
A file that taplo refused MUST become a problem that names the file and holds
the reason of taplo.

taplo[report.diagnostic]
A diagnostic of taplo MUST become a problem at the line and the column that
the diagnostic names, with the message of taplo.

taplo[report.summarized]
A file that carries a diagnostic MUST NOT also carry the problem of the line
that sums its diagnostics up.

## Paths

Taplo starts in the project root and reports absolute paths. A finding names
its file relative to the root, so that a reader, a machine, and a code host
see the same path for the same file, and the crate makes the paths of a
problem relative.

taplo[path.relative]
The crate MUST report the path of a problem relative to the root of the
project.

taplo[path.foreign]
A path that the project root does not contain MUST get no relative name.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[taplo]: https://taplo.tamasfe.dev
[tracey]: https://tracey.bearcove.eu/
