# Rakko Format YAML

`rakko-format-yaml` provides the action that formats the YAML files of
a project with [prettier]. The action wraps the prettier that mise pinned for
the project, so a run agrees with the editor and with a contributor that runs
prettier bare. Prettier reads its own configuration and does the formatting.
The action names the files that a run examines, selects the operation — a
report, or a rewrite — and translates what prettier reported into an outcome.

One action per language is a decision about feedback. Prettier formats
YAML, JSON, and YAML in one call, and a check that fails over all of them
says only that some file in the project needs attention. An action per
language names the language, so that a contributor reads which part of a pull
request to look at.

Prettier reports as text, and the action reads that text through the machinery
that every prettier action shares. The shape of the text belongs to a version
of prettier, and the pin softens the risk: a new shape arrives with a new
version, a new version arrives with a pull request, and a report that the
action does not recognize stops the run instead of passing quietly, so the
drift shows as a red pull request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

formatyaml[name]
The action MUST identify itself as `format-yaml`.

## Applicability

The action applies to a project that holds YAML files. The examination is
a cheap look of the action's own, and it runs before the tool resolves, so
that a broad bundle stays safe: a project without YAML files and without a
prettier skips visibly instead of stopping over a tool that it has no reason
to install. The look also keeps a run from starting a prettier that would
refuse a pattern which matches no file.

The look mirrors the files that the run examines where mirroring is cheap. It
matches the `.yaml` and `.yml` extensions with the case that prettier
matches, and it reads
hidden directories, because prettier reads them. It reads neither the `.git`
entry, which holds no file of the project, nor a `node_modules` entry, which
prettier excludes, and it follows no symbolic link, so that a cycle of links
cannot trap it.

The look and prettier can still disagree at the margins, because the ignore
files of a project can exclude every file that the look found. A run that
reaches prettier then reports what prettier saw, and a prettier that refuses
the pattern reports that the action found nothing to do after all.

formatyaml[skip.missing]
A run in a project that holds no file with the `.yaml` or the `.yml`
extension MUST report
that the action does not apply, and MUST NOT resolve the tool. The reason MUST
name what the run looked for.

formatyaml[skip.git]
The examination MUST NOT read the `.git` entry of the project.

formatyaml[skip.dependencies]
The examination MUST NOT read the `node_modules` entry of a directory.

formatyaml[skip.links]
The examination MUST NOT follow a symbolic link.

formatyaml[skip.unmatched]
A run whose prettier matched no file MUST report that the action does not
apply, and the reason MUST say that prettier found nothing to examine.

## Arguments

The action reads one argument. A run reports by default, and the `fix`
argument lets prettier rewrite what it can format. Reporting is the safe
default, because a run that a user started in order to look must not change
the tree that they hold.

formatyaml[args.fix]
The action MUST declare one argument: `fix`, holding a value that is true or
false, with documentation.

formatyaml[args.value]
A value for `fix` that is not true or false MUST fail the construction of the
arguments, and the failure MUST report the argument.

## Tool

The action runs the prettier that mise installed for the project, at the
version that the project pinned, so a run reaches the same program as the
editor and the terminal of a contributor. A prettier that mise does not report
stops the action, because provisioning is the job of mise, and the action
installs nothing.

Prettier runs on Node, and mise installs the two as separate tools. A project
that pins prettier without Node therefore resolves a program that cannot
start, and the action reports the failure of the run.

formatyaml[tool.prettier]
A run that applies MUST resolve `prettier` through mise for the project of the
run, and MUST run the program that mise reports.

formatyaml[tool.missing]
A run whose prettier mise does not report MUST stop, and the outcome MUST hold
the error.

## Report

A run without the fix argument asks prettier to report: prettier examines the
YAML files that its ignore files leave and names what it cannot leave
alone, and nothing rewrites the project.

Prettier reports three kinds of problems. A file that differs from what
prettier would write gets a path and nothing else. A file that prettier could
not open gets a path and the reason of the operating system. A file that
prettier read and could not parse gets a line, a column, and a message. All
three are problems of the project, so all three travel as findings, each at
the level that prettier could name.

A configuration that did not reach the run stops the run. Prettier ignores an
option that it does not know with a warning and then runs without it, and a
run without it quietly does what the project asked it not to do, so the action
treats the warning as the end of the run. A configuration file that prettier
cannot read at all stops the run the same way.

A report that the action does not recognize stops the run as well. A run that
ended without success and named no problem wrote a report that the action
could not read, and an answer built on such a report would hide every problem
behind a green result.

formatyaml[check.read]
A run without a true value for `fix` MUST NOT change the project.

formatyaml[check.passed]
A run whose prettier reports no problem MUST pass.

formatyaml[check.unformatted]
A file that prettier reports as one that a rewrite would change MUST produce a
finding that names that file.

formatyaml[check.invalid]
A file that prettier cannot parse MUST produce a finding at the line and the
column that prettier reports, with the message of prettier.

formatyaml[check.unreadable]
A file that prettier cannot read MUST produce a finding that names the file,
with the reason of prettier.

formatyaml[check.configuration]
A run whose prettier rejected a configuration file, or ignored an option of
one, MUST stop, and the error MUST hold what prettier reported.

formatyaml[check.unrecognized]
A prettier run that ends without success and reports no problem that the
action recognizes MUST stop the run, and the error MUST hold what prettier
wrote.

## Fix

A run with the fix argument asks prettier to rewrite. Prettier names every
file that it examined and marks the ones that it left alone, so one run both
repairs the project and says what it repaired, and no report has to run before
it.

A file that prettier cannot parse or cannot read remains, because a rewrite
repairs formatting and a broken file needs a hand.

formatyaml[fix.write]
A run with a true value for `fix` MUST let prettier rewrite the files that it
can format.

formatyaml[fix.changed]
A run that rewrote a file and left no problem MUST report the change, and the
outcome MUST hold one repair for each file that prettier rewrote.

formatyaml[fix.partial]
A run that left a problem MUST fail, and the outcome MUST hold the repairs
next to the problems that remain.

[prettier]: https://prettier.io
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
