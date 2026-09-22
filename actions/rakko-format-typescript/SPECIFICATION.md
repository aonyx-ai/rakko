# Rakko Format TypeScript

`rakko-format-typescript` provides the action that formats the TypeScript of a
project with [oxfmt]. The action wraps the oxfmt that mise pinned for the
project, so a run agrees with the editor and with a contributor that runs
oxfmt bare. Oxfmt reads its own configuration and does the formatting. The
action names the files that a run examines, selects the operation — a report,
or a rewrite — and translates what oxfmt reported into an outcome.

Oxfmt formats more languages than this action asks it about. It writes
Markdown, JSON, YAML, CSS, HTML, and others, and the actions that wrap
prettier already own those languages in a project that mounts them. Two
formatters that rewrite the same file disagree about it quietly, on every
commit, so the action names the extensions of the language that it is about
and oxfmt never sees the rest. One action per language is also a decision
about feedback: a check that fails names the language to look at.

The name says TypeScript and the files include JavaScript. A TypeScript
project carries configuration and scripts that are written in JavaScript, the
tool reads both with one parser, and the action that lints this language
covers both for the same reason. A project written in JavaScript alone is
served by this action as well, whatever its name suggests.

Oxfmt reports a run as text, and the action reads that text, because oxfmt
offers nothing structured. The shape of the text belongs to a version of
oxfmt, and the pin softens the risk: a new shape arrives with a new version, a
new version arrives with a pull request, and a report that the action does not
recognize stops the run instead of passing quietly, so the drift shows as a
red pull request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

formattypescript[name]
The action MUST identify itself as `format-typescript`.

## Files

A run names the files that carry an extension of the language, and oxfmt
collects them. Oxfmt skips the dependencies of a Node project and reads the
ignore files of the project on its own, so the name of the extensions is the
whole of what the action decides about the selection.

formattypescript[files.extensions]
A run MUST name the files that carry a TypeScript or a JavaScript extension,
and no other place to look.

## Skipping

The action applies to a project that holds files of the language, and oxfmt is
what decides that. Oxfmt treats a pattern that matches no file as an error, so
a project without such files reports itself, and the action answers with a
skip rather than with a problem. A project whose ignore files exclude every
one of them reports the same way, which is the right answer: oxfmt had nothing
to examine either way.

formattypescript[skip.unmatched]
A run whose oxfmt matched no file MUST report that the action does not apply,
and the reason MUST say that oxfmt found nothing to examine.

## Arguments

The action reads one argument. A run reports by default, and the `fix`
argument lets oxfmt rewrite what it can format. Reporting is the safe default,
because a run that a user started in order to look must not change the tree
that they hold. It is also the safe default against the tool, which rewrites
the project when nobody asks it for an operation at all.

formattypescript[args.fix]
The action MUST declare one argument: `fix`, holding a value that is true or
false, with documentation.

formattypescript[args.value]
A value for `fix` that is not true or false MUST fail the construction of the
arguments, and the failure MUST report the argument.

## Tool

The action runs the oxfmt that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. An oxfmt that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

Oxfmt runs on Node, and mise installs the two as separate tools. A project
that pins oxfmt without Node therefore resolves a program that cannot start,
and the action reports the failure of the run.

formattypescript[tool.oxfmt]
A run MUST resolve `oxfmt` through mise for the project of the run, and MUST
run the program that mise reports.

formattypescript[tool.missing]
A run whose oxfmt mise does not report MUST stop, and the outcome MUST hold
the error.

## Report

A run without the fix argument asks oxfmt to list the files that a rewrite
would change. Oxfmt writes one path per file and nothing else, and it rewrites
nothing. This selects the presentation of the report and not the behavior of
the tool: what oxfmt does to a file comes from the configuration of the
project alone.

Oxfmt reports problems at two levels. A file that differs from what oxfmt
would write gets a path and nothing else. A file that oxfmt could not turn
into a formatted file gets a sentence, and a position in that file where oxfmt
reached one: a file that it could not parse names the character that broke it,
while a file that it could not read or could not save names no place, because
oxfmt never got far enough to have one. All of them are problems of the
project, so all of them travel as findings, each at the level that oxfmt could
name. Oxfmt suggests what to do about some of its failures, and the message of
a finding carries the suggestion where oxfmt wrote one, so that a contributor
reads the answer of the tool and not one that Rakko wrote about it.

Oxfmt draws its report for a reader when it believes that the stream carries
one, and it uses plain marks otherwise. Neither shape says anything that the
other does not, so the reading recognizes both and answers the same way. The
choice is not the action's to make, because oxfmt offers no option for it and
makes the decorated shape the one that a build server gets.

A configuration that oxfmt refuses stops the run. Oxfmt reads its
configuration before it collects a single file, and it formats nothing at all
when it cannot accept that configuration, so the project asked for rules that
never applied.

A report that the action does not recognize stops the run as well. A run that
ended without success and named no problem wrote a report that the action
could not read, and an answer built on such a report would hide every problem
behind a green result.

formattypescript[check.read]
A run without a true value for `fix` MUST NOT change the project.

formattypescript[check.operation]
A run without a true value for `fix` MUST ask oxfmt to list the files that a
rewrite would change, and MUST set no other option of oxfmt.

formattypescript[check.passed]
A run whose oxfmt reports no problem MUST pass.

formattypescript[check.unformatted]
A file that oxfmt reports as one that a rewrite would change MUST produce a
finding that names that file.

formattypescript[check.invalid]
A file that oxfmt could not format at a place that it named MUST produce a
finding at the line and the column of that place, with the message of oxfmt.

formattypescript[check.unplaced]
A failure that oxfmt named no place for MUST produce a finding about the
project, with the message of oxfmt.

formattypescript[check.decorated]
A run whose oxfmt drew its report for a reader MUST produce the findings that
a run without the decoration produces.

formattypescript[check.configuration]
A run whose oxfmt refused a configuration file MUST stop, and the error MUST
hold what oxfmt reported.

formattypescript[check.unrecognized]
An oxfmt run that ends without success and reports no problem that the action
recognizes MUST stop the run, and the error MUST hold what oxfmt wrote.

## Fix

A run with the fix argument asks oxfmt to rewrite. Oxfmt names no file that it
rewrote, so the rewrite alone says only that it happened, and the action asks
for the list of files a second time once the rewrite is done. What the first
list holds and the second one does not is what the run repaired, and what the
second list still holds is what remains.

A file that oxfmt cannot parse remains, and so does a file that it cannot
write. A rewrite repairs formatting, and a file that the tool cannot read or a
tree that refuses a write needs a hand.

formattypescript[fix.write]
A run with a true value for `fix` MUST let oxfmt rewrite the files that it can
format.

formattypescript[fix.changed]
A run that repaired every problem that it found MUST report the change, and
the outcome MUST hold one repair for each file that oxfmt rewrote.

formattypescript[fix.partial]
A run that repaired part of what it found MUST fail, and the outcome MUST hold
the repairs next to the problems that remain.

[oxfmt]: https://oxc.rs/docs/guide/usage/formatter.html
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
