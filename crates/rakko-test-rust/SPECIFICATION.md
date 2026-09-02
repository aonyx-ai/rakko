# Rakko Test Rust

`rakko-test-rust` provides the action that runs the tests of a project with
[nextest]. The action wraps the cargo that mise pinned for the project, so a
run agrees with the terminal of a contributor that runs nextest bare. Cargo
builds every target, and nextest reads its own configuration and runs the
tests. The action selects the operation and translates what nextest and
cargo reported into an outcome.

Nextest reports the tests as JSON when a run asks for that format, and cargo
reports the diagnostics of the build as JSON as well, so a failed test and a
build that does not finish both arrive as data. The structured report of
nextest is experimental, and nextest asks for consent through a variable in
the environment of the command. The shape of an experimental report can
change with a version, and the pin turns the change into a red pull request
instead of a quiet drift.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

testrust[name]
The action MUST identify itself as `test-rust`.

## Applicability

The action applies to a project that holds a manifest of cargo. The
examination is a cheap look that runs before the tool resolves, so that a
broad bundle stays safe: a project without Rust code and without a cargo skips
visibly instead of stopping over a tool that it has no reason to install.

The look reads hidden directories, because a project can keep a package in
one. It does not read the `.git` entry, which holds no file of the project,
and it does not read a directory named `target`, where cargo builds. It
follows no symbolic link, so that a cycle of links cannot trap it.

testrust[skip.missing]
A run in a project that holds no file named `Cargo.toml` MUST report that the
action does not apply, and MUST NOT resolve the tool. The reason MUST name
what the run looked for.

testrust[skip.git]
The examination MUST NOT read the `.git` entry of the project.

testrust[skip.target]
The examination MUST NOT read a directory named `target`.

testrust[skip.links]
The examination MUST NOT follow a symbolic link.

## Arguments

The action reads no argument. A test that fails needs a hand, and nothing
that a run could take as an argument changes that.

testrust[args.none]
The action MUST declare no argument.

## Tool

The action runs the cargo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the terminal of
a contributor. Nextest is a plugin of cargo, and cargo finds it on the path
of the environment that mise sets, at the version that the project pinned. A
cargo that mise does not report stops the action, because provisioning is the
job of mise, and the action installs nothing.

testrust[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

testrust[tool.missing]
A run whose cargo mise does not report MUST stop, and the outcome MUST hold
the error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and nextest runs the tests of one workspace at a
time. A run therefore tests every workspace root of the project, and a
workspace that the run cannot discover stops it, because a run that skipped a
workspace would hide every failure of that workspace behind a green result.

testrust[roots.all]
A run MUST test every workspace root of the project.

testrust[roots.error]
A run whose workspace roots cannot be discovered MUST stop, and the outcome
MUST hold the error.

## Run

Nextest builds every target of every package with every feature enabled, so
that a test behind a feature runs as well, and it runs the tests the way the
configuration of the project says: whether a failure stops the run early, how
many tests run at once, and which tests are retried all come from that
configuration and not from the action. Nothing about the project changes,
whatever the run finds.

The action asks nextest for its structured report and cargo for its report as
JSON, because it reads both as data. This selects the presentation of the
reports and not the behavior of the tools. The structured report of nextest
is experimental, and nextest asks for consent through a variable in the
environment of the command; the action gives that consent for the command
that it starts and for nothing else.

A test that failed becomes a finding that names the test and carries the
message of the panic, at the line and the column where the test panicked,
when the report names them. A build that does not finish arrives as the
diagnostics of the compiler, and each becomes a finding at the range that the
compiler named. A workspace without a test is not a failure: a project can
keep its tests in one workspace and its harness in another, so such a
workspace ran no test, and the count of the run says so.

A report that the action does not recognize stops the run. A run that ended
without success and reported no failure, no diagnostic, and no absence of
tests wrote a report that the action could not read, and an answer built on
such a report would hide every failure behind a green result.

testrust[run.read]
A run MUST NOT change the project.

testrust[run.operation]
A run MUST ask nextest to run every target with every feature, MUST ask
nextest for its structured report and cargo for its report as JSON, and MUST
NOT change any other option of nextest or cargo.

testrust[run.consent]
A run MUST give consent to the experimental report of nextest in the
environment of the command that it starts, and in no other place.

testrust[run.passed]
A run whose nextest reports no failure and whose cargo reports no diagnostic
at any root MUST pass, and the outcome MUST say how many tests the run ran
and in how many workspaces.

testrust[run.none]
A workspace without a test MUST count as a workspace that ran no test, and
MUST NOT fail the run.

testrust[run.failed]
A test that failed MUST produce a finding that names the test and carries the
message of the panic, and a run with such a finding MUST fail.

testrust[run.position]
The finding of a failed test MUST be at the line and the column where the
test panicked, with the path relative to the project root, when the report
names them.

testrust[run.build]
A diagnostic of the compiler MUST produce a finding at the range that the
compiler reports, with the message and the code of the diagnostic, and a run
with such a finding MUST fail.

testrust[run.unrecognized]
A nextest run that ends without success and reports no failure, no
diagnostic, and no absence of tests MUST stop the run, and the error MUST hold
what nextest wrote.

testrust[run.unreadable]
A stream that holds a record of nextest or of cargo which the action cannot
read MUST stop the run, and the error MUST name the root and hold the record.

[nextest]: https://nexte.st
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
