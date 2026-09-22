# Rakko Lint TypeScript

`rakko-lint-typescript` provides the action that lints the TypeScript of a
project with [oxlint]. The action wraps the oxlint that mise pinned for the
project, so a run agrees with the editor and with a contributor that runs
oxlint bare. Oxlint discovers the files, reads its own configuration, and
applies its rules. The action names the project and translates what oxlint
reported into an outcome.

Linting is not formatting. Oxlint asks whether the code does something that
the project turned a rule against, such as a variable that nothing reads, a
`debugger` statement that reached a commit, or a type that hides a mistake
from the compiler. How the code is laid out is the question of the action that
wraps prettier, and a file that answers this one can still answer that one
badly.

Oxlint writes its report as one JSON object, and the action reads it rather
than the status of the process. The status alone is misleading in both
directions: oxlint ends with success when every rule that a file broke is one
that the project weighs as a warning, and it ends without success for a
project that holds no file to lint, which is no failure at all. The report
says what happened in both cases, so the report is what answers.

Oxlint gives each diagnostic a severity. A project decides that severity for
each rule, and the rules that oxlint enables by default carry the warning
severity. This action reports a warning and an error alike, because both are
rules that the project asked oxlint to look for, and a run with either fails.
The action reads the report to reach that answer, and it changes no option of
oxlint to get there.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

linttypescript[name]
The action MUST identify itself as `lint-typescript`.

## Skipping

The action applies to a project that holds TypeScript, and oxlint is what
decides that. The report of a run says how many files oxlint examined, so the
selection comes from the configuration of the project rather than from a guess
about it. A project whose configuration ignores every file it would otherwise
collect is answered correctly for the same reason.

linttypescript[skip.unexamined]
A run whose oxlint examined no file MUST report that the action does not
apply, and the reason MUST say that oxlint found nothing to examine.

## Arguments

The action reads no argument. A run only reports, because a rule that oxlint
repairs is still a rule that a contributor decides about: oxlint rewrites code
when it is asked to, and a fix that changes what a program does is not one
that a check applies on its own.

linttypescript[args.none]
The action MUST declare no argument.

## Tool

The action runs the oxlint that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. An oxlint that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

linttypescript[tool.oxlint]
A run MUST resolve `oxlint` through mise for the project of the run, and MUST
run the program that mise reports.

linttypescript[tool.missing]
A run whose oxlint mise does not report MUST stop, and the outcome MUST hold
the error.

## Runs

Oxlint discovers no files until a run names a place to look. The action names
the root of the project and nothing else, so a run covers what a contributor
covers when they start oxlint bare in the root of their checkout, and the
configuration of the project decides the rest.

The action asks for the report as data. Oxlint draws a block of source per
diagnostic by default, and the same run writes the diagnostics as one JSON
object on request, where each of them carries the rule, the severity, the
file, the position, and the message in fields instead of in a block that a
reader has to take apart. The object also carries how many files the run
examined, which is the count that a passing run reports and the answer that a
skip rests on. The format protects the run from its environment as well,
because the default format changes on a terminal and on a build server. This
selects the presentation of the report and not the behavior of the tool.

linttypescript[run.project]
A run MUST name the root of the project to oxlint, and no other place to look.

linttypescript[run.structured]
A run MUST ask oxlint for its report as data, and MUST set no other option of
oxlint.

## Configuration

The configuration of the project is the source of truth, and oxlint reads it
without help from the action. The action selects no rule, no category, and no
severity, so a project that turns a rule off is not asked about it, and a
project that never configured oxlint gets the rules that oxlint enables by
itself.

A configuration that oxlint refuses stops the run. Oxlint reads its
configuration before it collects a single file, and it writes no report at
all when it cannot accept that configuration, so the project asked for rules
that never applied.

linttypescript[check.configuration]
A run MUST apply the rules that the configuration of the project selects.

linttypescript[check.unreported]
A run whose oxlint wrote no report MUST stop, and the error MUST hold what
oxlint wrote.

## Check

Oxlint examines the files that it discovered and reports every rule that a
file broke. Nothing about the project changes, whatever the run finds.

Each diagnostic names the rule, how the project weighs it, the file, and what
oxlint said about the rule. The action reports each of them as a finding at
the position that oxlint marked, and the message of the finding reads like the
line that oxlint writes for a reader, so that a contributor reads the answer
of the tool and not one that Rakko wrote about it. Oxlint adds a sentence
about what to do for most of its rules, and the message carries it where
oxlint wrote one.

A diagnostic marks one or more places of a file. Oxlint writes one diagnostic
for a rule however many places it marks, and the action reports one finding
for the same reason, at the first place. A report that the action cannot read
stops the run, instead of passing with the diagnostics that the reading
happened to understand.

linttypescript[check.read]
A run MUST NOT change the project.

linttypescript[check.passed]
A run whose oxlint reports no diagnostic MUST pass, and the outcome MUST say
how many files oxlint examined.

linttypescript[check.diagnostic]
A diagnostic that oxlint reports MUST produce a finding at the line and the
column that oxlint marked, and the message MUST hold the severity, the rule,
and what oxlint said about it.

linttypescript[check.severity]
A diagnostic that oxlint reports as a warning MUST produce a finding, and a
run that reports one MUST fail.

linttypescript[check.unreadable]
A run whose report the action cannot read MUST stop, and the error MUST hold
what oxlint wrote.

[oxlint]: https://oxc.rs/docs/guide/usage/linter.html
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
