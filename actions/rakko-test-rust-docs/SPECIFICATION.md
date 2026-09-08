# Rakko Test Rust Docs

`rakko-test-rust-docs` provides the action that runs the examples in the
documentation of a project. An example in a doc comment is a test: the
compiler builds it against the library that documents it, and the test
harness runs it. It is also the part of the documentation that rots most
quietly, because nothing else in a project names the symbols that it uses.

The action exists beside the action that runs the tests, because [nextest]
does not run the examples. Nextest omits them without a word, so a project
that runs its tests with nextest alone has every example unchecked. The
action wraps the cargo that mise pinned for the project and asks it for the
documentation tests of every workspace, so a run agrees with the terminal of
a contributor that runs `cargo test --doc` bare.

Cargo reports the diagnostics of the build as JSON, and the test harness
writes its own report as text on the same stream. The report of the harness
is the older of the two formats, and a structured one is available only on
an unstable option, so the action reads the text. It reads the summary of
each workspace for the count and the block of a failed example for what the
example wrote. A stream that the action cannot answer from stops the run
instead of passing quietly, so the drift shows as a red pull request.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

testrustdocs[name]
The action MUST identify itself as `test-rust-docs`.

## Skipping

The action applies to a project that holds a cargo workspace with a library
in it, and cargo is what decides both. A run discovers the workspace roots of
the project before it does anything with them, and a project that holds no
manifest gives an empty discovery.

A workspace can also hold no library. The harness of a project is a binary,
and a library that builds a C library links no example, so cargo refuses a
run that asks for the documentation examples of such a workspace. Each root
of the discovery states whether cargo can test its examples, and a project
whose roots all say no has nothing to test.

testrustdocs[skip.undiscovered]
A run whose cargo discovers no workspace MUST report that the action does not
apply, and the reason MUST say that cargo found none.

testrustdocs[skip.libraries]
A run whose workspaces hold no library MUST report that the action does not
apply, and the reason MUST say that no workspace holds one.

## Arguments

The action reads no argument. An example that fails needs a hand, and nothing
that a run could take as an argument changes that.

testrustdocs[args.none]
The action MUST declare no argument.

## Tool

The action runs the cargo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the terminal of
a contributor, and the examples answer for the compiler that the project
ships with. A cargo that mise does not report stops the action, because
provisioning is the job of mise, and the action installs nothing.

testrustdocs[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

testrustdocs[tool.missing]
A run whose cargo mise does not report MUST stop, and the outcome MUST hold
the error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and cargo works on one workspace at a time. A run
therefore tests every workspace root that holds a library, and it leaves the
other roots alone, because cargo refuses a run at a root without one.

A workspace that the run cannot discover stops it, because a run that skipped
one would hide every failing example of that workspace behind a green result.

testrustdocs[roots.all]
A run MUST test every workspace root of the project whose documentation cargo
tests, and MUST NOT run the tool at a root whose documentation it does not.

testrustdocs[roots.error]
A run whose workspace roots cannot be discovered MUST stop, and the outcome
MUST hold the error.

## Run

The action tests one workspace at a time and sums what the runs reported.
Nothing about the project changes, whatever a run finds.

Cargo tests the examples of every package of the workspace, with every
feature, so that an example behind a feature runs as well. It stops at the
first package whose examples fail unless a run asks it to continue, and a run
that stopped there would report the failures of one package and leave the
packages behind it unexamined, so the action asks for all of them.

An example that failed and a diagnostic of a build that did not finish are
both problems of the project, and both travel as findings, so a run that gets
either of them fails. A library without an example is not a failure: a
project documents what it has, and such a workspace ran no example, and the
count of the run says so.

A run that leaves the action without an answer stops it. Such a run examined
nothing that the action can report, and an answer built on it would hide
every failure behind a green result.

testrustdocs[run.operation]
A run MUST ask cargo to test the documentation of every package of the
workspace with every feature, MUST ask cargo to test every package even after
one of them failed, MUST ask cargo for its report as JSON, and MUST NOT
change an option of cargo that this document does not name.

testrustdocs[run.read]
A run MUST NOT change the project.

testrustdocs[run.passed]
A run whose examples all passed and whose cargo reports no diagnostic at any
root MUST pass, and the outcome MUST say how many examples the run ran and in
how many workspaces.

testrustdocs[run.none]
A workspace without an example MUST count as a workspace that ran no example,
and MUST NOT fail the run.

testrustdocs[run.failed]
An example that failed MUST fail the run, and the outcome MUST hold the
finding of the example.

testrustdocs[run.build]
A diagnostic of the compiler MUST fail the run, and the outcome MUST hold the
finding of the diagnostic.

testrustdocs[run.error]
A run that leaves the action without an answer MUST stop the run, and the
outcome MUST hold the error.

## Report

Cargo tests one package at a time, and the test harness writes a summary per
package, which counts the examples that passed, that failed, and that the
harness left out. It writes a block
for each example that failed, which holds what the example wrote: the
diagnostics of the compiler for an example that does not compile, and the
panic for an example that ran and failed. Cargo writes its own records as
JSON on the same stream, and the action reads the lines of both.

An example that the harness left out ran as little as an example that a
filter removed, so the count of a run is the examples that passed and the
examples that failed.

A summary whose counts the action cannot read stops the run, and so does a
run that ends without success and reports nothing that the action can name.
Both wrote a report that the action cannot answer from, and an answer built
on it would hide failures behind a green result.

testrustdocs[report.ran]
A run MUST report how many examples ran, which is every example that passed
and every example that failed, over every workspace of the run.

testrustdocs[report.failures]
A run MUST hold every example that the harness reported as failed, with the
name of the example and what the example wrote.

testrustdocs[report.unreadable]
A summary whose counts the action cannot read MUST stop the run, and the
error MUST name the root and hold the summary.

testrustdocs[report.unrecognized]
A run that ends without success and reports no failed example and no
diagnostic MUST stop the run, and the error MUST name the root and hold what
cargo wrote.

## Findings

The action names a problem of the project as precisely as the report allows.
The harness names an example by the file that documents it and the line that
the example starts on, and that name is where the finding goes. The place
where an example panicked is not: the compiler builds the examples of a
workspace into one program, and the panic then names a file of that program
and not a file of the project.

A finding names its file relative to the root of the project, which is the
name that a reader, a machine, and a code host all recognize. An example
whose name the action cannot read, and an example in a file outside the
project, get a finding at the level of the project instead.

testrustdocs[finding.failed]
An example that failed MUST produce a finding that names the example and
carries what the example wrote, which is the message of the panic for an
example that panicked and the first line of the output otherwise.

testrustdocs[finding.position]
The finding of a failed example MUST be at the line that the name of the
example carries, with the path relative to the project root.

testrustdocs[finding.build]
A diagnostic of the compiler MUST produce a finding at the range that the
compiler reports, with the message and the code of the diagnostic.

[nextest]: https://nexte.st
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
