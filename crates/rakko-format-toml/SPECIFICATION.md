# Rakko Format TOML

`rakko-format-toml` provides the action that formats the TOML files of a
project with [taplo]. The action wraps the taplo that mise pinned for the
project, so a run agrees with the editor and with a contributor that runs
taplo bare. Taplo discovers the files, reads its own configuration, and does
the formatting. The action selects the operation — a check, or a fix — and
translates what taplo reported into an outcome.

Taplo reports a format run as text on its standard error stream, and the
action parses that text, because taplo offers nothing structured for it. The
shape of the text belongs to a version of taplo, and the pin softens the
risk: a new shape arrives with a new version, a new version arrives with a
pull request, and a report that the action does not recognize stops the run
instead of passing quietly, so the drift shows as a red pull request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

formattoml[name]
The action MUST identify itself as `format-toml`.

## Applicability

The action applies to a project that holds TOML files. The examination is a
cheap look of the action's own, and it runs before the tool resolves, so that
a broad bundle stays safe: a project without TOML files and without a taplo
skips visibly instead of stopping over a tool that it has no reason to
install.

The look mirrors the discovery of taplo where mirroring is cheap. It matches
the `.toml` extension with the case that taplo matches, and it reads hidden
directories, because taplo reads them. It does not read the `.git` entry,
which holds no file of the project, and it follows no symbolic link, so that
a cycle of links cannot trap it. A directory that the look cannot read counts
as applicable, because a look that cannot prove absence must not hide a real
check behind a skip. The look and taplo can still disagree at the margins — a
configuration can exclude every file that the look found — and a run that
reaches taplo then reports what taplo saw.

formattoml[skip.missing]
A run in a project that holds no file with the `.toml` extension MUST report
that the action does not apply, and MUST NOT resolve the tool. The reason
MUST name what the run looked for.

formattoml[skip.git]
The examination MUST NOT read the `.git` entry of the project.

formattoml[skip.links]
The examination MUST NOT follow a symbolic link.

## Arguments

The action reads one argument. A run reports by default, and the `fix`
argument lets taplo rewrite what it can format. Reporting is the safe
default, because a run that a user started in order to look must not change
the tree that they hold.

formattoml[args.fix]
The action MUST declare one argument: `fix`, holding a value that is true or
false, with documentation.

formattoml[args.value]
A value for `fix` that is not true or false MUST fail the construction of the
arguments, and the failure MUST report the argument.

## Tool

The action runs the taplo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. A taplo that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

formattoml[tool.taplo]
A run that applies MUST resolve `taplo` through mise for the project of the
run, and MUST run the program that mise reports.

formattoml[tool.missing]
A run whose taplo mise does not report MUST stop, and the outcome MUST hold
the error.

## Check

Every run starts with a check: taplo examines the files that its
configuration selects and reports the problems, and nothing rewrites the
project. The action asks taplo for plain output, because it reads the report
as data, and a color code inside a path would corrupt the parse. This selects
the presentation of the report and not the behavior of the tool: what taplo
does to the project comes from the configuration of the project alone.

Taplo reports two kinds of problems. A file that is not formatted gets a path
and nothing else, and a file that taplo cannot parse gets a line, a column,
and a message. Both are problems of the project, so both travel as findings,
each at the level that taplo could name.

A configuration file that taplo rejects stops the run. Taplo itself warns and
then runs with its defaults, and a run on the defaults quietly does what the
project asked it not to do, so the action treats the warning as the end of
the run.

A report that the action does not recognize stops the run as well. A run that
ended without success and named no problem, and a run that passed without the
count of the files, both wrote a report that the action could not read, and
an answer built on such a report would hide every problem behind a green
result.

Taplo can lose the tail of its report when it exits. A run that ended without
success closes its report with the summary of the failure, and a run that
passed closes it with the count of the files, so a report without its closing
line is incomplete, and problems can be missing from it. The action starts
such a run again, a few times, before it treats the report as one that it
does not recognize. Repeating is safe, because a check only reads, and a
rewrite formats files that a previous attempt already formatted.

formattoml[check.read]
A run without a true value for `fix` MUST NOT change the project.

formattoml[check.passed]
A run whose taplo reports no problem MUST pass, and the outcome MUST say how
many files taplo checked.

formattoml[check.unformatted]
A file that taplo reports as not formatted MUST produce a finding that names
that file, with the path relative to the project root.

formattoml[check.invalid]
A file that taplo cannot parse MUST produce a finding at the line and the
column that taplo reports, with the message of taplo.

formattoml[check.configuration]
A run whose taplo rejects a configuration file MUST stop, and the error MUST
hold what taplo reported about the file.

formattoml[check.unrecognized]
A taplo run that ends without success and reports no problem that the action
recognizes MUST stop the run, and the error MUST hold what taplo wrote.

## Fix

A run with the fix argument repairs what it can. The check runs first either
way, because taplo does not report which files a rewrite touched, and the
check is where the action learns them. When the check finds nothing, the run
passes and no rewrite starts. When it finds problems, taplo rewrites the
files that it can format, and the files that it cannot parse remain: a
rewrite repairs formatting, and a syntax error needs a hand.

formattoml[fix.write]
A run with a true value for `fix` MUST let taplo rewrite the files that are
not formatted.

formattoml[fix.changed]
A run that repaired every problem that the check found MUST report the
change, and the outcome MUST hold one repair for each file that taplo
rewrote.

formattoml[fix.partial]
A run that repaired part of what the check found MUST fail, and the outcome
MUST hold the repairs next to the problems that remain.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[taplo]: https://taplo.tamasfe.dev
[tracey]: https://tracey.bearcove.eu/
