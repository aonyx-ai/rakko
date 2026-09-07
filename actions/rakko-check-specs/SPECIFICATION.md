# Rakko Check Specs

`rakko-check-specs` provides the action that checks the specifications of a
project with [tracey]. A specification states what a crate does as a list of
requirements, each with an identifier, and the code that implements or
verifies a requirement names that identifier in a comment. Tracey holds the
two together. The action wraps the tracey that mise pinned for the project, so
a run agrees with the editor and with a contributor that runs tracey bare.

The action asks tracey three questions, because a specification can fail a
project in three ways. Does every reference point at a requirement that
exists, at the version that the requirement carries now? Does a change to the
text of a requirement carry the version bump that such a change needs? And how
much of the specification does the code answer for?

The first two questions gate a run, and the third does not. A reference that
names nothing, and a requirement whose text moved under the code that
implements it, are both broken links, and a broken link is worth stopping for.
Coverage is information: a specification may land before the code that answers
it, and a run that failed for that would punish the order in which the work
arrives.

Tracey answers the first and the third question as JSON, and the action reads
those reports. The shape of a report belongs to a version of tracey, and the
pin softens the risk: a new shape arrives with a new version, a new version
arrives with a pull request, and a report that the action cannot read stops
the run instead of passing quietly, so the drift shows as a red pull request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

checkspecs[name]
The action MUST identify itself as `check-specs`.

## Skipping

The action applies to a project that configures tracey, and tracey is what
decides that. A run asks tracey for the coverage of the project, and a project
that configures nothing gets a report with no implementation in it and a
normal exit. The action reads that as a project with no requirements to check.

The question costs nothing extra. A run that passes reports the same coverage
as its summary, so the answer is read once and used twice.

checkspecs[skip.unconfigured]
A run whose tracey reports no requirement MUST report that the action does not
apply, and the reason MUST say that tracey tracks none.

## Arguments

The action reads no argument. A run only reports. Tracey can bump the version
of a requirement whose text changed, and that is a decision about what a
requirement now means, not a repair that an action makes on a contributor's
behalf.

checkspecs[args.none]
The action MUST declare no argument.

## Tool

The action runs the tracey that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. A tracey that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

checkspecs[tool.tracey]
A run that applies MUST resolve `tracey` through mise for the project of the
run, and MUST run the program that mise reports.

checkspecs[tool.missing]
A run whose tracey mise does not report MUST stop, and the outcome MUST hold
the error.

## Daemon

Tracey answers a query from a daemon that it keeps per workspace, and that
daemon serves what it scanned. It notices a change to a file seconds after the
change, so a query that follows an edit closely enough answers for the tree as
it was before the edit. A check that reports the problems of a tree that no
longer exists is worse than no check, because it passes a tree that it never
read.

A run therefore stops the daemon before it asks anything. The next query
starts a daemon that scans the workspace first, so the answer is about the
tree that the run is checking. Whatever else uses tracey, such as the editor
of a contributor, starts a daemon of its own again on its next call.

checkspecs[daemon.stop]
A run MUST stop the daemon of the project before it asks tracey a question.

checkspecs[daemon.absent]
A run MUST NOT fail because the project had no daemon to stop.

## References

Tracey validates the specifications of the project against the code, and
reports what it refused: a reference that names no requirement, a reference
that names a requirement at a version older than the one the requirement
carries, an identifier that two requirements share, an identifier that its
naming rules refuse, and a file that it could not parse.

The action asks tracey to treat a warning as fatal. Tracey sorts what it finds
into warnings and errors, and both describe a link between a specification and
the code that no longer holds. A project that lets the warnings accumulate
learns nothing from the ones that remain.

checkspecs[check.read]
A run MUST NOT change the project.

checkspecs[check.structured]
A run MUST ask tracey for its report as data.

checkspecs[check.warnings]
A run MUST ask tracey to treat a warning as fatal.

checkspecs[check.diagnostic]
Each problem that tracey reports MUST produce a finding at the line and the
column that tracey names, in the file that tracey names, with the message of
tracey.

checkspecs[check.unreadable]
A run whose report the action cannot read MUST stop, and the error MUST hold
what tracey wrote.

## Versions

The text of a requirement is a promise, and the code that implements it
answers that promise. A change to the text therefore needs a new version of
the requirement, so that every reference to the old version shows up as stale
and somebody reads the code again. Tracey compares the staged content of a
specification with the committed one and reports a change that carries no
bump.

That comparison needs a commit to compare against. In the checkout of a
contributor the comparison is the natural one: the index holds what they
staged, and HEAD holds what they last committed. On a pull request it is not.
A code host checks out a merge commit of the branch and its base, stages
nothing, and a comparison of that index against that HEAD finds no change at
all, so every pull request would pass this gate without being read.

The comparison that a pull request needs is the content of the pull request
against the base branch, which is the first parent of the merge commit. The
action builds that comparison in a disposable copy of the repository: a
worktree at the first parent, whose index holds the tree of the merge commit.
Tracey then compares the pull request with its base, and the checkout of the
run keeps the commit and the index that it had.

The copy is what keeps this safe. Moving the HEAD of the checkout would
produce the same comparison in one command, and an action is mounted by
projects that this one never sees, where an environment that looks like a
pull request and a checkout that is not one would cost a contributor a commit.
An action that never writes to the repository of its caller cannot make that
mistake.

checkspecs[version.staged]
A run MUST report a staged change to the text of a requirement that carries no
new version of that requirement, and such a change MUST fail the run.

checkspecs[version.base]
A run on a checkout of a merge commit, in an environment that names the base
branch of a pull request, MUST compare the tree of that merge commit with its
first parent.

checkspecs[version.checkout]
A run MUST NOT change the commit, the index, or the working tree of the
repository of the project.

## Coverage

Tracey counts the requirements of each specification, and how many of them the
code implements and the tests verify. The counts are information and not a
gate: a specification may land before its implementation, and the gaps belong
in the report of a run and in the dashboard of tracey.

checkspecs[coverage.summary]
A run that passes MUST report how many requirements the project holds, and how
many of them are implemented and verified.

checkspecs[coverage.open]
A requirement that no code implements, and a requirement that no test
verifies, MUST NOT fail a run.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
