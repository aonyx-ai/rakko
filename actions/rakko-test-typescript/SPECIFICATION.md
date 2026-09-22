# Rakko Test TypeScript

`rakko-test-typescript` provides the action that runs the tests of a Node
project with the [test runner] that Node carries. The action wraps the node
that mise pinned for the project, so a run agrees with a contributor that runs
the tests bare. Node finds the test files, runs each of them in a process of
its own, and reports what every test did. The action names the project and
translates that report into an outcome.

The runner is the one built into Node, and the project adds nothing for it. A
test file imports `node:test`, and a project that wants its tests run needs no
dependency, no configuration file, and no script. That is what makes this
action worth having for a project that would otherwise run no tests at all.

Node decides which files hold tests. It carries a set of patterns for that,
and it walks the project from the root with them, so the action states no
pattern of its own. A project therefore gets the same set of tests from the
action as from a bare run, and a project that changes its mind about where its
tests live changes nothing here.

Node reports the run as [TAP], which names every test, says whether it passed,
and carries the place and the reason of each failure. The report is the answer
of a run, and the status of the process decides only whether a run without a
failure is trusted. The shape of a report belongs to a version of Node, and a
report that the run cannot answer from stops the action instead of passing
quietly, so the drift shows as a red pull request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

The name says TypeScript, because that is the language of the projects that
mount this action, and because it puts the action next to the three that lint,
format, and type check the same code. Node runs the JavaScript of a project as
well, and it strips the types of a TypeScript file on the way in, so a project
that holds both gets both tested.

testtypescript[name]
The action MUST identify itself as `test-typescript`.

## Arguments

The action reads no argument. A test that fails needs a hand, and nothing that
a run could take as an argument changes that.

testtypescript[args.none]
The action MUST declare no argument.

## Tool

The action runs the node that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the terminal of
a contributor. The version decides what the runner does: the test runner of
Node gained the discovery of its files, the reading of TypeScript, and the
shape of its report over several releases, and a project that pins an older
Node gets an older answer. A node that mise does not report stops the action,
because provisioning is the job of mise, and the action installs nothing.

testtypescript[tool.node]
A run MUST resolve `node` through mise for the project of the run, and MUST
run the program that mise reports.

testtypescript[tool.missing]
A run whose node mise does not report MUST stop, and the outcome MUST hold the
error.

## Runs

A run starts the test runner of Node in the root of the project, and names no
file and no pattern to it. Node then walks the project with the patterns that
it carries, so the tests of a run are the tests of a bare run, and a project
that holds its packages in directories below the root gets all of them from
the one run. What Node leaves out of that walk, such as the directory that
holds the installed packages, belongs to Node and not to the action.

The report is what the action reads, and it asks for it as TAP. Node draws the
run for a reader otherwise, with a summary that a person reads and a machine
does not, and the drawing changes with the width of a terminal. TAP is the
same everywhere. This selects the presentation of the report and not the
behavior of the runner.

Nothing of the project changes, whatever a run finds. A test that writes a
file writes it, because the project asked for that, and the action writes
nothing of its own.

testtypescript[run.project]
A run MUST start node in the root of the project, and MUST name no file and no
pattern, so that the discovery of node selects the tests.

testtypescript[run.tap]
A run MUST ask node for the report of the run as TAP, and MUST set no other
option of the runner.

testtypescript[run.read]
A run MUST NOT change the project.

## Skipping

The action applies to a project that holds a test, and Node is what decides
that. A project whose walk finds no test file, and a project whose test files
hold no test, both give a report that counts no test at all, and such a run
examined nothing. A run that reported no test is therefore an action that does
not apply, and not a project whose tests all passed.

testtypescript[skip.untested]
A run whose node reports that it ran no test MUST report that the action does
not apply, and the reason MUST say that node found no test.

## Results

A test that failed is a problem of the project, and it travels as a finding.
The finding names the test, so that a reader knows which one to look at
without reading the whole report, and it carries what Node said about the
failure, so that a reader reads the answer of the runner and not one that
Rakko wrote about it.

Node reports the place of a failure as the line and the column where the test
was declared, which is where a reader goes to repair it. A failure that Node
reports no place for becomes a finding about the project, because there is no
path to report it at.

A test that holds tests below it fails when one of those fails, and Node
reports both. The test below is the one that a reader repairs, so only it
becomes a finding, and the one above is left out. A run that reported both
would name the same failure twice and hide the count of real ones.

A test that the project marked as skipped, or as still to do, is not a test
that the project stands behind, and Node counts neither of them as a failure
however they end. A run of such a test therefore changes nothing about the
outcome. This matters because Node reports a test that is still to do as
failed while it counts it as not failed, and a reading that went by the report
alone would fail a run that Node passes.

A run that reported no failure passes, and it says how many tests it ran, so
that a reader can see that a run which found nothing wrong also ran something.

A run that ended without success and reported no failure stops the action.
Node reports what it found, so such a run stopped for a reason of its own, and
the action states that instead of passing a project whose tests never ran to
the end. A report that the action cannot read stops the run for the same
reason.

testtypescript[result.passed]
A run whose node reports no failure MUST pass, and the outcome MUST say how
many tests the run ran.

testtypescript[result.failed]
A test that failed MUST fail the run, and the outcome MUST hold a finding that
names the test.

testtypescript[result.message]
The finding of a test that failed MUST hold what node said about the failure.

testtypescript[result.position]
The finding of a test that failed MUST sit at the line and the column that
node reported for the test.

testtypescript[result.project]
The finding of a test that node reported no place for MUST be about the
project.

testtypescript[result.subtests]
A test that failed only because a test below it failed MUST NOT produce a
finding of its own.

testtypescript[result.excused]
A test that the project marked as skipped, or as still to do, MUST NOT fail
the run, whatever the test did.

testtypescript[report.unreported]
A run whose node ended without success and reported no failure MUST stop, and
the error MUST hold what node wrote.

testtypescript[report.unreadable]
A run whose report the action cannot read MUST stop, and the error MUST hold
what node wrote.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tap]: https://testanything.org
[test runner]: https://nodejs.org/api/test.html
[tracey]: https://tracey.bearcove.eu/
