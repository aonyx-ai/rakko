# Rakko Nextest

`rakko-nextest` carries the machinery that the actions which run the tests of
a project with [nextest] share. Nextest is a plugin of cargo, and every action
that runs it asks the same questions: which command does a run write, what did
nextest and cargo report, and can the caller answer from what they wrote? This
crate answers all three, so that an action names the workspace that it wants
tested and reads the answer as data.

A run tests one workspace root, because cargo works on one workspace at a
time. The caller resolves cargo for the project and discovers the roots, and
it runs the crate once per root. Cargo builds every target of every package
with every feature, and nextest reads its own configuration and runs the
tests, so a run agrees with the terminal of a contributor that runs nextest
bare.

Nextest reports the tests as JSON when a run asks for that format, and cargo
reports the diagnostics of the build as JSON as well, so a failed test and a
build that does not finish both arrive as data. The structured report of
nextest is experimental, and nextest asks for consent through a variable in
the environment of the command. The shape of an experimental report can change
with a version, and one place that reads it is one place that a new version
can break. The pin softens the risk further: a new shape arrives with a new
version, a new version arrives with a pull request, and a report that the
crate does not recognize stops the caller instead of passing quietly.

The crate judges nothing. It reports what nextest and cargo said, and the
action that asked for the run decides what the answer means for its outcome.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Run

One place holds the command line of a run, so an action states the workspace
that it wants tested and never a flag. Nextest builds every target of every
package with every feature, so that a test behind a feature runs as well, and
it runs the tests the way the configuration of the project says: whether a
failure stops the run early, how many tests run at once, and which tests are
retried all come from that configuration and not from the crate.

The crate asks nextest for its structured report and cargo for its report as
JSON, because it reads both as data. This selects the presentation of the
reports and not the behavior of the tools. Nextest hands the consent to the
experimental report through a variable in the environment of the command, and
the crate sets that variable for the command that it starts and for nothing
else, so a process that the caller starts later is unaffected.

The lockfile of the workspace is the one part of the run that the caller
decides. A run that answers for the project as it stands lets cargo resolve
the dependencies of the build, because that resolution is part of the build
that a contributor gets. A caller that resolved the dependencies before the
run answers for those versions instead, and a build that quietly chose
another one would answer a question that nobody asked, so cargo refuses such
a build. The refusal reaches the caller as a run that reported nothing it can
answer from.

nextest[run.operation+2]
A run MUST ask nextest to run every target with every feature, MUST ask
nextest for its structured report and cargo for its report as JSON, and MUST
NOT change an option of nextest or cargo that this document does not name.

nextest[run.consent]
A run MUST give consent to the experimental report of nextest in the
environment of the command that it starts, and in no other place.

nextest[run.lockfile]
A run MUST ask cargo to build the versions that the lockfile of the workspace
holds when the caller asks for that, and MUST leave the resolution of cargo
alone when the caller does not.

## Report

Nextest and cargo write their JSON to the same stream, one document per line,
and the crate reads the lines of both. It keeps every test that failed with
what the test wrote, and it sums the tests of every binary that finished. It
ignores every other line, so a line that a new version adds does not break the
reading.

A workspace without a test is not a failure of a project. A project can keep
its tests in one workspace and its harness in another, so such a workspace ran
no test, and nextest says so with an exit status of its own.

A report that the crate cannot read stops the caller, and so does a report
that the crate does not recognize. A run that ended without success and
reported no failure, no diagnostic, and no absence of tests wrote a report
that the crate could not read, and an answer built on such a report would hide
every failure behind a green result.

nextest[report.ran]
A run MUST report how many tests ran, which is every test that passed and
every test that failed, over every binary of the workspace.

nextest[report.failures]
A run MUST hold every test that nextest reported as failed, with the name of
the test and what the test wrote.

nextest[report.none]
A workspace where nextest found no test MUST count as an answer, with no test
that ran and no finding.

nextest[report.unrecognized]
A run that ends without success and reports no failure, no diagnostic, and no
absence of tests MUST stop with an error, and the error MUST name the root and
hold what nextest wrote.

nextest[report.unreadable]
A stream that holds a record of nextest or of cargo which the crate cannot
read MUST stop with an error, and the error MUST name the root and hold the
record.

## Findings

The crate names a problem of the project as precisely as the report allows. A
test that failed carries the output of the test, and the test harness of Rust
writes the message of the panic and its location there. A build that does not
finish arrives as the diagnostics of the compiler, which name a range of their
own.

A finding names its file relative to the root of the project, which is the
name that a reader, a machine, and a code host all recognize. A test that
panicked in a file outside the project, and a test whose output names no
panic, get a finding at the level of the project instead.

nextest[finding.failed]
A test that failed MUST produce a finding that names the test and carries the
message of the panic.

nextest[finding.position]
The finding of a failed test MUST be at the line and the column where the test
panicked, with the path relative to the project root, when the report names
them.

nextest[finding.build]
A diagnostic of the compiler MUST produce a finding at the range that the
compiler reports, with the message and the code of the diagnostic.

[nextest]: https://nexte.st
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
