# Rakko Pre-Commit

`rakko-pre-commit` provides the `pre-commit` command, which runs the actions
that guard a commit. A harness mounts the command beside its actions and gives
it two lists: the actions that write to the tree, and the actions that only
read it. The harness names the actions, and the command drives them.

A commit has to pass a list of actions, and no action describes that list. The
list repairs the tree first and then examines what it repaired. Whoever reads
the result wants what each action said, and an action answers with one
outcome. A written command can drive many actions and write one report for
each of them, so this crate provides a written command and not an action.

The shape of the run is the same in every project, and only the lists change.
Which activities guard a commit is a decision of the project, so the crate
names no action. A project that needs a different shape writes a command of
its own.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key word
MUST has the meaning that [RFC 2119] defines.

## Name

The hook that Git runs before a commit calls the command by its name. The name
therefore names the activity and not the actions in it.

precommit[name]
The command MUST identify itself as `pre-commit`.

## Arguments

The command reads one argument. A run reports by default, and the `fix`
argument lets the actions that write repair what they find. Reporting is the
safe default, because a run that a user started to look must not change the
tree that they hold. The hook that guards a commit gives `fix`.

precommit[args.fix]
The command MUST declare one argument: `fix`, holding a value that is true or
false, with documentation.

precommit[args.value]
A value for `fix` that is not true or false MUST fail the construction of the
arguments, and the failure MUST report the argument.

precommit[args.absent]
A run that gives `fix` no value MUST give no argument to the actions that
write.

## Lists

The harness gives the command the actions that write to the tree and the
actions that only read it. Each list holds its actions in the order in which
they run. An action that writes is not only a formatter. A generator whose
output derives from formatted files writes as well. Its place in the first
list comes after the formatters that it reads.

The actions that write run first, so that what the other actions read is what
the commit will contain. Two actions that write can change the same file, so
no two actions run at the same time. The actions that read run one after
another as well, because no action declares what it reads and writes.

Only the actions that write repair a problem, so only these actions receive
`fix`. An action that only reads declares no such argument.

precommit[lists.order]
A run MUST drive every action that writes, in the order of its list, before
it drives the first action that reads. It MUST drive the actions that read in
the order of their list.

precommit[lists.sequential]
A run MUST wait for the end of an action before it starts the next action.

precommit[lists.fix]
A run that gives `fix` the value true MUST give every action that writes the
`fix` argument with the value true. A run MUST give no argument to the actions
that read.

## Report

Whoever reads a run wants what each action said. The command writes the report
that a run of one action writes, so its output has the shape of the output of
each action alone, in text and in JSON. The run writes a report when its
action ends, so a reader learns what one action found while the next action
runs.

A report that nothing reads is lost. The reader then learns nothing about the
actions after it either, so the run stops at that report.

precommit[report.each]
A run MUST write one report for each action that it drives, in the order in
which it drives them.

precommit[report.unreported]
A run whose report does not reach the reader MUST fail, and the failure MUST
name the action of that report.

## Result

An action answers with one of five outcomes, and a command answers with
success or an error. The run therefore reduces each outcome to clean or to a
problem, and the reports keep what each action said.

An action that passed is clean. An action that does not apply is clean,
because a skip is an answer. An action that repaired everything that it found
is clean as well, because the run asked for the repair. The files that it
changed stay in the working tree, and the hook that started the run compares
them with the files of the commit.

An action that found problems is a problem. An action that stopped is a
problem as well, although it says nothing about the project. A commit that
nothing examined is what the hook exists to prevent.

A problem does not end the run. A run that stops at the first problem hides
the problems after it, and whoever started it learns them one run at a time.
The run fails at its end, and the failure counts the actions, because the
reports already said what each action found.

precommit[result.clean]
A run MUST count an action that passed, changed the project, or skipped as
clean.

precommit[result.problem]
A run MUST count an action that failed or stopped as a problem.

precommit[result.continue]
An action that a run counts as a problem MUST NOT end the run. The run MUST
drive every action after it.

precommit[result.failed]
A run that counts an action as a problem MUST fail after it drove every
action. The failure MUST state how many actions were problems and how many
actions the run drove.

precommit[result.succeeded]
A run that counts every action as clean MUST succeed.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
