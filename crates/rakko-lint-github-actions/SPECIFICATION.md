# Rakko Lint GitHub Actions

`rakko-lint-github-actions` provides the action that audits the GitHub Actions
workflows of a project with [zizmor]. Zizmor is a static analysis tool for
GitHub Actions, and it looks for the patterns that turn a workflow into a way
into the repository, such as a template that expands attacker-controlled text
into a shell, a job that carries more permissions than it needs, and an action
that no digest pins. The action wraps the zizmor that mise pinned for the
project, so a run agrees with the editor and with a contributor that runs
zizmor bare. Zizmor collects the files, reads its own configuration, and
applies its audits. The action names the project and translates what zizmor
reported into an outcome.

Auditing is not linting the YAML. Whether a workflow file is a well-formed
YAML document, and whether it obeys the layout rules of the project, are the
questions of the actions that wrap yamllint and prettier. A file that answers
those two well can still hand a pull request the write token of the
repository.

Zizmor reports its findings as JSON, and this action reads that report. The
shape of the report belongs to a version of zizmor, and the pin softens the
risk: a new shape arrives with a new version, a new version arrives with a
pull request, and a report that the action cannot read stops the run instead
of passing quietly, so the drift shows as a red pull request.

Zizmor gives each finding a severity, and it reports a finding of every
severity in one run. This action reports them all, because each of them is a
pattern that zizmor was asked to look for, and a project that wants an audit
to stay quiet turns that audit off in its configuration.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

lintgithubactions[name]
The action MUST identify itself as `lint-github-actions`.

## Applicability

The action applies to a project that holds GitHub Actions workflows. The
examination is a cheap look of the action's own, and it runs before the tool
resolves, so that a broad bundle stays safe. A project without workflows and
without a zizmor skips visibly instead of stopping over a tool that it has no
reason to install.

The look reads one directory: `.github/workflows` below the root of the
project. GitHub reads a workflow in that directory and nowhere else, so a
project with a workflow has one there. The look matches the `.yaml` and the
`.yml` extension, which are the two that GitHub reads.

The look follows no symbolic link, so that a link cannot lead it out of the
project. A directory that the look cannot read counts as holding workflows,
because a look that cannot prove absence must not hide a real check behind a
skip.

Zizmor collects more than the workflows of a project. It also collects an
action definition, a Dependabot configuration, and the configuration and the
hooks of pre-commit. The look therefore answers narrower than zizmor, and a
project whose only auditable file is one of those skips. Such a project has no
GitHub Actions workflows, which is what this action is named for, and a
project that adds its first workflow gets the rest of the audit with it.

lintgithubactions[skip.missing]
A run in a project whose `.github/workflows` directory holds no file with the
`.yaml` or the `.yml` extension MUST report that the action does not apply,
and MUST NOT resolve the tool. The reason MUST name what the run looked for.

lintgithubactions[skip.links]
The examination MUST NOT follow a symbolic link.

lintgithubactions[skip.uncollected]
A run whose zizmor collects no input MUST report that the action does not
apply, and the reason MUST say that zizmor found nothing to audit.

## Arguments

The action reads no argument. A run only reports. Zizmor can repair some of
what it finds, and its own documentation calls that experimental, so a run of
this action leaves every repair to a contributor.

lintgithubactions[args.none]
The action MUST declare no argument.

## Tool

The action runs the zizmor that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. A zizmor that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

lintgithubactions[tool.zizmor]
A run that applies MUST resolve `zizmor` through mise for the project of the
run, and MUST run the program that mise reports.

lintgithubactions[tool.missing]
A run whose zizmor mise does not report MUST stop, and the outcome MUST hold
the error.

## Runs

Zizmor collects no file until a run names a place to look. The action names
the root of the project and nothing else, so a run covers what a contributor
covers when they start zizmor bare in the root of their checkout, and the
configuration of the project decides the rest.

A run asks for the report in the JSON format. Zizmor writes its findings for a
reader by default, in a block per finding that carries the source of the
workflow, and the same run writes the findings as data on request. Each of
them then carries the audit, the severity, the place, and the annotation in
fields instead of in a block that a reader has to take apart. The format also
protects the run from the environment, because the default format changes on a
terminal and on a build server.

A run asks for the pedantic persona. A persona decides how much a run reports:
the regular persona reports the findings that zizmor is confident about, and
the pedantic persona adds the code smells that a reviewer of a workflow wants
to see. Zizmor takes a persona on its command line alone, and its
configuration file has no key for one, so a run that names no persona gets the
regular one and the project has no way to ask for more. The action therefore
names the persona that this fleet audits with, and a contributor who wants the
same answer from a bare zizmor passes `--pedantic` to it.

A run asks zizmor to stop at a file that it collected and cannot read. Zizmor
warns about such a file by default, drops it, and audits the rest, so a
workflow with a syntax error or with a key that GitHub does not define leaves
the audit through a warning that no outcome carries. A project whose only
workflow is such a file passes over nothing at all. An audit that is worth
running is worth running over every file that it collected, so the run asks
for the stop and the outcome names the file.

lintgithubactions[run.project]
A run MUST name the root of the project to zizmor, and no other place to look.

lintgithubactions[run.structured]
A run MUST ask zizmor for its report in the JSON format.

lintgithubactions[run.persona]
A run MUST ask zizmor for the findings of the pedantic persona.

lintgithubactions[run.strict]
A run MUST ask zizmor to stop at a file that it collected and cannot read.

## Configuration

The configuration of the project is the source of truth, and zizmor reads it
without help from the action. A configuration that zizmor rejects stops the
run, because zizmor audits nothing at all in that case, and a report that
never arrived says nothing about the project.

lintgithubactions[check.configuration]
A run whose zizmor rejects the configuration of the project MUST stop, and the
error MUST hold what zizmor wrote about it.

## Check

Zizmor audits the files that it collected and reports every pattern that it
recognized. Nothing about the project changes, whatever the run finds.

A finding of zizmor names one or more locations. The first of them is where
the finding is, and the others are what a reader needs to read it: the step
that holds the expression, the job that the permissions belong to, the block
that a rule speaks about. Zizmor draws them together in one block of source,
which an outcome of Rakko has no place for, so each location becomes a finding
of its own. Nothing that zizmor said is then lost, and the audit and the
severity in each message say which of them belong together.

Each finding covers the range that zizmor named, from the start of the range
to its end, so a reader and a code host see the same part of the file that
zizmor underlines. The message holds the severity, the audit, what the audit
is about, and what zizmor wrote about this location, so that a contributor
reads the answer of the tool and not one that Rakko wrote about it. A finding
that zizmor calls informational becomes a finding like one that it calls high,
and a run with either fails.

A report that the action cannot read stops the run, and so does a run that
zizmor could not finish. Zizmor ends a run when it cannot accept the
configuration of the project, and, because the run asks it to, when it cannot
read a file that it collected. It audits nothing in the first case and less
than the project in the second, and an outcome built on such a report would
hide a part of the project behind the problems of another part.

lintgithubactions[check.read]
A run MUST NOT change the project.

lintgithubactions[check.passed]
A run whose zizmor reports no finding MUST pass.

lintgithubactions[check.finding]
A location that a finding of zizmor names MUST produce a finding of the action
that covers the range of the location, and the message MUST hold the severity,
the audit, the description of the audit, and what zizmor wrote about the
location.

lintgithubactions[check.severity]
A finding of any severity that zizmor reports MUST produce a finding, and a
run that reports one MUST fail.

lintgithubactions[check.unreadable]
A run whose report the action cannot read MUST stop, and the error MUST hold
what zizmor wrote.

lintgithubactions[check.incomplete]
A run that zizmor could not finish MUST stop, and the error MUST hold what
zizmor wrote.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
[zizmor]: https://docs.zizmor.sh
