# Rakko Test Rust

`rakko-test-rust` provides the action that runs the tests of a project with
[nextest]. The action wraps the cargo that mise pinned for the project, so a
run agrees with the terminal of a contributor that runs nextest bare. Cargo
builds every target, and nextest reads its own configuration and runs the
tests. The action names the workspaces of the project and translates what
nextest and cargo reported into an outcome.

Nextest reports the tests as JSON, and cargo reports the diagnostics of the
build as JSON as well, so a failed test and a build that does not finish both
arrive as data. The action runs nextest and reads both reports through the
machinery that every action which runs nextest shares. The shape of a report
belongs to a version of the tools, and a report that the run cannot answer
from stops the action instead of passing quietly, so the drift shows as a red
pull request.

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

The action tests one workspace at a time and sums what the runs reported.
Nothing about the project changes, whatever a run finds.

A test that failed and a diagnostic of a build that did not finish are both
problems of the project, and both travel as findings, so a run that gets
either of them fails. A workspace without a test is not a failure: a project
can keep its tests in one workspace and its harness in another, so such a
workspace ran no test, and the count of the run says so.

A run that nextest leaves without an answer stops the action. Such a run
examined nothing that the action can report, and an answer built on it would
hide every failure behind a green result.

testrust[run.read]
A run MUST NOT change the project.

testrust[run.passed]
A run whose nextest reports no failure and whose cargo reports no diagnostic
at any root MUST pass, and the outcome MUST say how many tests the run ran
and in how many workspaces.

testrust[run.none]
A workspace without a test MUST count as a workspace that ran no test, and
MUST NOT fail the run.

testrust[run.failed+2]
A test that failed MUST fail the run, and the outcome MUST hold the finding
of the test.

testrust[run.build+2]
A diagnostic of the compiler MUST fail the run, and the outcome MUST hold the
finding of the diagnostic.

testrust[run.error]
A run of nextest that leaves the action without an answer MUST stop the run,
and the outcome MUST hold the error.

[nextest]: https://nexte.st
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
