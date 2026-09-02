# Rakko Prettier

`rakko-prettier` carries the machinery that the actions which wrap [prettier]
share. Prettier formats many languages, and one action wraps each group of
files that a project keeps apart, so that a check names the language that
needs attention instead of the tool that examined it. Every one of those
actions asks the same three questions, so this crate answers them once: does
the project hold files of the group, which prettier runs here, and what did
that prettier report?

Prettier discovers no files of its own. A run names the files that it
examines, and the caller therefore names the extensions of its group, and the
crate writes the pattern. A caller that names no extension gets every file
that prettier understands, which is what a project gets today from a bare
call of prettier.

The crate reads the report of prettier as text, because prettier offers
nothing structured for it. The shape of that text belongs to a version of
prettier, and one place that reads it is one place that a new version can
break. The pin softens the risk further: a new shape arrives with a new
version, a new version arrives with a pull request, and a report that the
crate does not recognize stops the caller instead of passing quietly, so the
drift shows as a red pull request.

The crate judges nothing. It reports what prettier said, and the action that
asked for the run decides what the answer means for its outcome.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Selection

Prettier examines the files that a run names and no others, so the selection
is the one part of the behavior that the caller supplies. An action names the
extensions of its group — the markdown action names `md`, and an action for
JSON names `json` and `json5` — and the crate turns them into the pattern
that prettier reads.

A caller that names no extension selects every file with an extension.
Prettier then skips the files whose language it does not know, so the run
covers what prettier understands and stops at nothing else. The skip is also
what keeps a broad selection usable at all: prettier refuses a file that it
cannot assign to a language, and a project holds many such files.

The selection names files and nothing else. It does not exclude a directory,
because prettier reads the ignore files of the project, and it changes no
option of prettier, because the behavior of the tool comes from the
configuration of the project alone.

prettier[select.extensions]
A run MUST examine the files whose extension the caller named, below the root
of the project, and no file with another extension.

prettier[select.any]
A run whose caller names no extension MUST examine every file that has an
extension.

prettier[select.unknown]
A run MUST let prettier skip a file whose language prettier does not know.

## Look

The look tells whether prettier has anything to do in a project. It is cheap,
and it runs before the tool resolves, so that a broad bundle stays safe: a
project without files of the group and without a prettier skips visibly
instead of stopping over a tool that it has no reason to install. The look
also keeps a run from starting a prettier that would refuse a pattern which
matches no file.

The look mirrors the selection where mirroring is cheap. It matches the
extensions of the selection with the case that prettier matches, and it reads
hidden directories, because prettier reads them. It does not read the `.git`
entry, which holds no file of the project, and it does not read the
`node_modules` entry, which prettier excludes. It follows no symbolic link,
so that a cycle of links cannot trap it.

The look and prettier can still disagree at the margins, because the ignore
files of a project can exclude every file that the look found. A caller that
reaches prettier therefore reports what prettier saw.

prettier[look.files]
The crate MUST report whether the project holds a file that the selection
matches, below the root that the caller names.

prettier[look.git]
The look MUST NOT read the `.git` entry of the project.

prettier[look.dependencies]
The look MUST NOT read the `node_modules` entry of a directory.

prettier[look.links]
The look MUST NOT follow a symbolic link.

prettier[look.unreadable]
A directory that the look cannot read MUST count as a directory that holds
files of the selection. A look that cannot prove absence must not hide a real
check behind a skip.

## Tool

The prettier that runs is the prettier that mise installed for the project, at
the version that the project pinned, so a run reaches the same program as the
editor and the terminal of a contributor. The crate installs nothing:
provisioning is the job of mise, and a prettier that Rakko installed would run
at a version that nothing pinned.

prettier[tool.resolve]
The crate MUST resolve `prettier` through mise for the project whose root the
caller names, and a run MUST start the program that mise reports.

prettier[tool.missing]
The crate MUST report an error that names the tool when mise reports no
prettier, and MUST NOT install prettier.

## Runs

A caller names the operation that it wants, and the crate writes the command
line. One place holds the command line of every operation, so an action states
a job and never a flag, and a change to how prettier is called reaches every
action at once.

The two operations differ in one thing: whether prettier writes. A report
names the files that a rewrite would change and leaves the project alone, so a
run that a user started in order to look changes nothing. A rewrite formats
what it can and names every file that it examined, and it marks the files that
it left unchanged, so the caller learns what the run repaired from the run
itself.

prettier[run.operation]
Each operation MUST run the flags of prettier that do that job, and no other
option of prettier.

## Report

Prettier splits its report over both streams. The files that a run names
travel on the standard output stream, one per line, and everything that
prettier could not do travels on the standard error stream. The crate reads
the lines that carry an answer and ignores the rest, so a line that a new
version adds does not break the reading, and what the reading cannot find is
absent from the answer.

A configuration file that prettier rejects is part of the answer, and so is an
option of one that prettier ignored. Prettier ignores an unknown option with a
warning and then runs without it, and a run without it quietly does what the
project asked it not to do, so the warning travels to the caller, which
decides that the run has ended.

Prettier names a problem as precisely as its operation allows. A file that a
report would rewrite gets a path and nothing else. A file that prettier could
not read gets a path and a reason, such as the message of the operating system
for a file that prettier could not open. A file that prettier read and could
not parse gets a line, a column, and a message. Each problem travels at the
level that prettier could name.

prettier[report.unformatted]
A file that a report names as one that a rewrite would change MUST become a
problem that names the file and no position in it.

prettier[report.rewritten]
The crate MUST report which files a rewrite changed, and MUST NOT report a
file that the rewrite left unchanged.

prettier[report.diagnostic]
A file that prettier could not parse MUST become a problem at the line and the
column that prettier names, with the message of prettier.

prettier[report.unreadable]
A file that prettier could not read MUST become a problem that names the file
and holds the reason of prettier.

prettier[report.configuration]
The crate MUST report what prettier said when it rejected a configuration file
of the project, and when it ignored an option of one.

prettier[report.pattern]
The crate MUST report whether prettier refused the pattern of the run because
no file matched it.

prettier[report.status]
The crate MUST report whether prettier ended with success.

## Paths

Prettier starts in the project root and reports the path of a file relative to
it, which is the name that a reader, a machine, and a code host all recognize.
A path that arrives absolute anyway loses the root, so that every problem
names its file the same way.

prettier[path.relative]
The crate MUST report the path of a problem relative to the root of the
project.

prettier[path.foreign]
A path that the project root does not contain MUST get no relative name.

[prettier]: https://prettier.io
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
