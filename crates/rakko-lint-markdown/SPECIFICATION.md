# Rakko Lint Markdown

`rakko-lint-markdown` provides the action that lints the Markdown files of a
project with [markdownlint]. The action wraps the markdownlint that mise
pinned for the project, so a run agrees with the editor and with a contributor
that runs markdownlint bare. Markdownlint discovers the files, reads its own
configuration, and applies its rules. The action names the project, asks for
the report as data, and translates what markdownlint reported into an outcome.

Linting is not formatting. Markdownlint asks whether a document follows the
rules that the project turned on — one heading level per step, a language on
every fenced block, a line that stays within the width that the project
chose. How the document is laid out is the question of the action that wraps
prettier, and a file that answers this one can still answer that one badly.

Markdownlint reports as JSON, which is the shape that this action reads. The
shape belongs to a version of markdownlint, and the pin softens the risk: a
new shape arrives with a new version, a new version arrives with a pull
request, and a report that the action cannot read stops the run instead of
passing quietly, so the drift shows as a red pull request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

lintmarkdown[name]
The action MUST identify itself as `lint-markdown`.

## Applicability

The action applies to a project that holds Markdown files. The examination is
a cheap look of the action's own, and it runs before the tool resolves, so
that a broad bundle stays safe: a project without Markdown files and without a
markdownlint skips visibly instead of stopping over a tool that it has no
reason to install.

The look mirrors the discovery of markdownlint where mirroring is cheap. It
matches the `.md` and `.markdown` extensions, which are the two that
markdownlint collects below a directory, and it reads no entry whose name
starts with a dot, because markdownlint reads none either. The rule about the
dot covers the `.git` entry, which holds no file of the project, and it needs
no exception for the directory of installed packages, which markdownlint reads
like any other.

The look follows no symbolic link, so that a cycle of links cannot trap it,
and markdownlint follows one. A project whose Markdown files sit only behind a
link therefore skips. The look and markdownlint can disagree at the margins
for the ignore file of the project as well, which can exclude every file that
the look found, and a run that reaches markdownlint then reports what
markdownlint saw.

lintmarkdown[skip.missing]
A run in a project that holds no file with the `.md` or the `.markdown`
extension MUST report that the action does not apply, and MUST NOT resolve the
tool. The reason MUST name what the run looked for.

lintmarkdown[skip.hidden]
The examination MUST NOT read an entry whose name starts with a dot.

lintmarkdown[skip.links]
The examination MUST NOT follow a symbolic link.

lintmarkdown[skip.unexamined]
A run whose markdownlint examined no file MUST report that the action does not
apply, and the reason MUST say that markdownlint found nothing to examine.

## Arguments

The action reads no argument. A run only reports, although markdownlint can
repair a part of what it finds, because a repair that this action made would
arrive without a name: markdownlint applies its fixes and then reports what
remains, and it says nothing about the files that it rewrote. An outcome that
holds a repair for each file has to observe the repair, and observing it here
would take a second run of the tool before the first one. The action that
wraps prettier already rewrites the Markdown of a project, so nothing is lost
while this stays a question that a contributor answers by hand.

lintmarkdown[args.none]
The action MUST declare no argument.

## Tool

The action runs the markdownlint that mise installed for the project, at the
version that the project pinned, so a run reaches the same program as the
editor and the terminal of a contributor. A markdownlint that mise does not
report stops the action, because provisioning is the job of mise, and the
action installs nothing.

Markdownlint runs on Node, and mise installs the two as separate tools. A
project that pins markdownlint without Node therefore resolves a program that
cannot start, and the action reports the failure of the run.

lintmarkdown[tool.markdownlint]
A run that applies MUST resolve `markdownlint` through mise for the project of
the run, and MUST run the program that mise reports.

lintmarkdown[tool.missing]
A run whose markdownlint mise does not report MUST stop, and the outcome MUST
hold the error.

## Runs

Markdownlint discovers no files until a run names a place to look. The action
names the root of the project and nothing else, so a run covers what a
contributor covers when they start markdownlint bare in the root of their
checkout, and the ignore file of the project decides the rest.

The action asks for the report as data. Markdownlint writes its findings for a
reader by default, and the same run writes them as JSON on request, which
carries the rule, the position, and the message in fields instead of in a
line that a reader has to take apart. This selects the presentation of the
report and not the behavior of the tool: which rules apply to which file comes
from the configuration of the project alone.

lintmarkdown[run.project]
A run MUST name the root of the project to markdownlint, and no other place to
look.

lintmarkdown[run.structured]
A run MUST ask markdownlint for its report as data, and MUST set no other
option of markdownlint.

## Check

Markdownlint examines the files that it discovered and reports every rule that
a file broke. Nothing about the project changes, whatever the run finds.

Markdownlint reports one result per broken rule. Each result names the file,
the line, the rule, and what the rule expected, and a rule that can point at a
character in the line names a column as well. The action reports each result
as a finding at the place that markdownlint could name, and the message of the
finding is the sentence that markdownlint would have written for a reader, so
that a contributor reads the answer of the tool and not one that Rakko wrote
about it.

A report that the action cannot read stops the run, and so does a run that
ended without success and named no result. Markdownlint answers a file that it
cannot open by ending the run with a stack trace and no report at all, so a
run that lost its report examined an unknown part of the project, and an
answer built on it would hide every problem behind a green result.

The configuration of the project is the one thing that the action cannot
check. Markdownlint reads a configuration file that it cannot parse as a file
that is not there, without a word on either stream, and runs on its defaults.
Nothing distinguishes that run from one that had no configuration to begin
with, so this action states no requirement about it, where the actions that
wrap taplo and prettier state one.

lintmarkdown[check.read]
A run MUST NOT change the project.

lintmarkdown[check.passed]
A run whose markdownlint reports no result MUST pass.

lintmarkdown[check.violation]
A rule that markdownlint reports MUST produce a finding on the line that
markdownlint names, and the message MUST hold the rule and what markdownlint
said about it.

lintmarkdown[check.column]
A result that names a column MUST produce a finding at that column, and a
result that names none MUST produce a finding on its line alone.

lintmarkdown[check.unreadable]
A run whose report the action cannot read MUST stop, and the error MUST hold
what markdownlint wrote.

lintmarkdown[check.unrecognized]
A markdownlint run that ends without success and reports no result MUST stop
the run, and the error MUST hold what markdownlint wrote.

[markdownlint]: https://github.com/DavidAnson/markdownlint
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
