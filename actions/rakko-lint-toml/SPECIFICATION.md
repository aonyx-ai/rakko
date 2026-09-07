# Rakko Lint TOML

`rakko-lint-toml` provides the action that validates the TOML files of a
project with [taplo]. The action wraps the taplo that mise pinned for the
project, so a run agrees with the editor and with a contributor that runs
taplo bare. Taplo discovers the files, reads its own configuration, and does
the validation. The action selects the operation and translates what taplo
reported into an outcome.

Validation is not formatting. Taplo asks whether it can read a file at all,
whether the file is TOML, and whether the content of the file matches the
schema that the project associated with it. How the file is laid out is the
question of another action, and a file that answers this one can still answer
that one badly.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

linttoml[name]
The action MUST identify itself as `lint-toml`.

## Applicability

The action applies to a project that holds TOML files, and taplo is what
decides that. Taplo reports a run that collected nothing on a log stream it
truncates on exit, so that answer arrives about four times in five and cannot
carry a requirement. A project without TOML files therefore passes, having
found nothing to report, rather than reporting that the action does not apply.

Little rests on the difference. A project marks its root with a TOML file, so
a project that a run can reach holds at least one.

## Arguments

The action reads no argument. Taplo repairs nothing that a validation finds: a
file that it cannot open, a file that is not TOML, and a value that its schema
refuses all need a hand. An action with no argument tells a user that, and a
`fix` that did nothing would tell them the opposite.

linttoml[args.none]
The action MUST declare no argument.

## Tool

The action runs the taplo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. A taplo that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

linttoml[tool.taplo]
A run that applies MUST resolve `taplo` through mise for the project of the
run, and MUST run the program that mise reports.

linttoml[tool.missing]
A run whose taplo mise does not report MUST stop, and the outcome MUST hold
the error.

## Check

Taplo examines the files that its configuration selects and reports what it
refused. Nothing about the project changes, whatever the run finds.

Taplo reports a refusal at the level that it could reach. A file that it read
and could not accept gets a line, a column, and a message, which covers a file
that is not TOML and a value that its schema refuses alike. A file that it
never opened gets the reason and nothing else, such as the message of the
operating system for a file that a run may not read. Both are problems of the
project, so both travel as findings.

A configuration file that taplo rejects stops the run. Taplo itself warns and
then runs with its defaults, and a run on the defaults quietly does what the
project asked it not to do, so the action treats the warning as the end of the
run.

A report that the action does not recognize stops the run as well. A run that
ended without success and named no file, and a run that passed without the
count of the files, both wrote a report that the action could not read, and an
answer built on such a report would hide every problem behind a green result.

linttoml[check.read]
A run MUST NOT change the project.

linttoml[check.passed+2]
A run whose taplo reports no problem MUST pass, and the outcome MUST say how
many files taplo checked, when taplo reported that count.

linttoml[check.diagnostic]
A file that taplo read and refused MUST produce a finding at the line and the
column that taplo reports, with the message of taplo.

linttoml[check.refused]
A file that taplo refused without naming a position in it MUST produce a
finding that names that file, with the path relative to the project root and
the reason of taplo.

linttoml[check.configuration]
A run whose taplo rejects a configuration file MUST stop, and the error MUST
hold what taplo reported about the file.

linttoml[check.unrecognized]
A taplo run that ends without success and reports no problem that the action
recognizes MUST stop the run, and the error MUST hold what taplo wrote.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[taplo]: https://taplo.tamasfe.dev
[tracey]: https://tracey.bearcove.eu/
