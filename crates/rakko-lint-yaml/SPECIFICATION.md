# Rakko Lint YAML

`rakko-lint-yaml` provides the action that lints the YAML files of a project
with [yamllint]. The action wraps the yamllint that mise pinned for the
project, so a run agrees with the editor and with a contributor that runs
yamllint bare. Yamllint discovers the files, reads its own configuration, and
applies its rules. The action names the project, asks which files yamllint
examines, and translates what yamllint reported into an outcome.

Linting is not formatting. Yamllint asks whether a document follows the rules
that the project turned on, such as a key that appears once, a line that stays
within the width that the project chose, and a value that no reader mistakes
for a boolean. How the document is laid out is the question of the action that
wraps prettier, and a file that answers this one can still answer that one
badly.

Yamllint reports one problem per line of text, and this action reads those
lines. The shape of a line belongs to a version of yamllint, and the pin
softens the risk: a new shape arrives with a new version, a new version
arrives with a pull request, and a line that the action cannot read stops the
run instead of passing quietly, so the drift shows as a red pull request.

Yamllint gives each problem a level. A project decides that level for each
rule, and the rules that yamllint enables by default carry both. This action
reports a warning and an error alike, because both are problems that the
project asked yamllint to look for. The action reads the report to reach that
answer, and it changes no option of yamllint to get there.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

lintyaml[name]
The action MUST identify itself as `lint-yaml`.

## Applicability

The action applies to a project that holds YAML files. The examination is a
cheap look of the action's own, and it runs before the tool resolves, so that
a broad bundle stays safe. A project without YAML files and without a yamllint
skips visibly instead of stopping over a tool that it has no reason to
install.

The look mirrors the discovery of yamllint where mirroring is cheap. It
matches the `.yaml` and `.yml` extensions and the name `.yamllint`, which are
the three that yamllint collects below a directory by default. It reads an
entry whose name starts with a dot, because yamllint reads one as well. A
project whose only YAML files sit in a directory such as `.github` therefore
applies. That rule costs a walk through the version control directory of the
project, which yamllint walks as well.

The look follows no symbolic link, so that a cycle of links cannot trap it. A
project whose YAML files sit only behind a link therefore skips. The look and
yamllint can also disagree about the configuration of the project, which can
name other file patterns and can exclude every file that the look found. The
action therefore asks yamllint which files it examines before it lints them.

lintyaml[skip.missing]
A run in a project that holds no file with the `.yaml` or the `.yml`
extension, and no file named `.yamllint`, MUST report that the action does not
apply, and MUST NOT resolve the tool. The reason MUST name what the run looked
for.

lintyaml[skip.hidden]
The examination MUST read an entry whose name starts with a dot.

lintyaml[skip.links]
The examination MUST NOT follow a symbolic link.

lintyaml[skip.unexamined]
A run whose yamllint examines no file MUST report that the action does not
apply, and the reason MUST say that yamllint found nothing to examine.

## Arguments

The action reads no argument. A run only reports, because yamllint repairs
nothing that it finds. The action that wraps prettier rewrites the YAML of a
project, and a rule of yamllint that no formatter answers stays a question
that a contributor answers by hand.

lintyaml[args.none]
The action MUST declare no argument.

## Tool

The action runs the yamllint that mise installed for the project, at the
version that the project pinned, so a run reaches the same program as the
editor and the terminal of a contributor. A yamllint that mise does not report
stops the action, because provisioning is the job of mise, and the action
installs nothing.

lintyaml[tool.yamllint]
A run that applies MUST resolve `yamllint` through mise for the project of the
run, and MUST run the program that mise reports.

lintyaml[tool.missing]
A run whose yamllint mise does not report MUST stop, and the outcome MUST hold
the error.

## Runs

Yamllint discovers no files until a run names a place to look. The action
names the root of the project and nothing else, so a run covers what a
contributor covers when they start yamllint bare in the root of their
checkout, and the configuration of the project decides the rest.

A run of the action starts yamllint twice, for two answers that one run of
yamllint cannot give. The first run asks which files yamllint examines. The
second run lints them.

The listing answers two questions that the report leaves open. A yamllint that
examined no file reports the same empty text as a yamllint that examined the
whole project and found nothing, so without the listing a project that
excludes all of its YAML would pass over nothing at all. The listing also
gives the count that a passing run reports, so that a reader can question a
pass that examined less than they expect.

The action asks for the report in the parsable format. Yamllint writes its
findings for a reader by default, and the same run writes one line per problem
on request, which carries the file, the position, the level, and the message
in fields instead of in a block that a reader has to take apart. The format
also protects the run from the environment, because the default format changes
on a terminal and on a build server. This selects the presentation of the
report and not the behavior of the tool: which rules apply to which file comes
from the configuration of the project alone.

lintyaml[run.project]
A run MUST name the root of the project to yamllint, and no other place to
look.

lintyaml[run.listing]
A run MUST ask yamllint which files it examines before it asks yamllint to
lint them.

lintyaml[run.structured]
A run MUST ask yamllint for its report in the parsable format, and MUST set no
other option of yamllint.

## Configuration

The configuration of the project is the source of truth, and yamllint reads it
without help from the action. A configuration that yamllint rejects stops the
run, because yamllint refuses to lint anything at all in that case, and a
report that never arrived says nothing about the project.

lintyaml[check.configuration]
A run whose yamllint rejects the configuration of the project MUST stop, and
the error MUST hold what yamllint wrote about it.

## Check

Yamllint examines the files that it discovered and reports every rule that a
file broke. Nothing about the project changes, whatever the run finds.

Yamllint reports one problem per broken rule. Each problem names the file, the
line, the column, the level, and what the rule expected, and the action
reports each problem as a finding at that position. The message of the finding
is the sentence that yamllint wrote for a reader, so that a contributor reads
the answer of the tool and not one that Rakko wrote about it. A problem that
yamllint calls a warning becomes a finding like one that it calls an error,
and a run with either fails.

A line that the action cannot read stops the run, and so does a run that
yamllint could not finish. Yamllint answers a file that it cannot open by
ending the run, and the files that it had not reached stay unread, so an
answer built on that report would hide a part of the project behind the
problems of another part.

lintyaml[check.read]
A run MUST NOT change the project.

lintyaml[check.passed]
A run whose yamllint reports no problem MUST pass, and the outcome MUST say
how many files yamllint examined.

lintyaml[check.problem]
A problem that yamllint reports MUST produce a finding at the line and the
column that yamllint names, and the message MUST hold the level and what
yamllint said about the rule.

lintyaml[check.level]
A problem that yamllint reports as a warning MUST produce a finding, and a run
that reports one MUST fail.

lintyaml[check.unreadable]
A run whose report holds a line that the action cannot read MUST stop, and the
error MUST hold what yamllint wrote.

lintyaml[check.incomplete]
A run that yamllint could not finish MUST stop, and the error MUST hold what
yamllint wrote.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
[yamllint]: https://github.com/adrienverge/yamllint
