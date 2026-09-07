# Rakko Check Latest Deps

`rakko-check-latest-deps` provides the action that checks a project against
the newest versions of its dependencies. A manifest states a floor for each
dependency, and a floor is a promise: every later version of that dependency
works as well. Nobody can read that promise off the manifest, so the action
resolves the newest versions that the manifests allow and runs the tests of
the project against them.

The action wraps the cargo that mise pinned for the project. Cargo resolves
the dependencies, and [nextest] runs the tests, so a run agrees with a
contributor who does the same two things by hand. The action selects the
operations and translates what the two runs reported into an outcome.

A run rewrites the lockfile of every workspace, and the checkout of a
contributor is no place for that, so the whole run happens in a disposable
copy of the project. The contributor keeps the tree they are working in,
whatever the run finds and however it ends.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

checklatestdeps[name]
The action MUST identify itself as `check-latest-deps`.

## Applicability

The action applies to a project that holds a manifest of cargo. The
examination is a cheap look that runs before the tool resolves, so that a
broad bundle stays safe: a project without Rust code and without a cargo skips
visibly instead of stopping over a tool that it has no reason to install.

The look reads hidden directories, because a project can keep a package in
one. It does not read the `.git` entry, which holds no file of the project,
and it does not read a directory named `target`, where cargo builds. It
follows no symbolic link, so that a cycle of links cannot trap it.

checklatestdeps[skip.missing]
A run in a project that holds no file named `Cargo.toml` MUST report that the
action does not apply, and MUST NOT resolve the tool. The reason MUST name
what the run looked for.

checklatestdeps[skip.git]
The examination MUST NOT read the `.git` entry of the project.

checklatestdeps[skip.target]
The examination MUST NOT read a directory named `target`.

checklatestdeps[skip.links]
The examination MUST NOT follow a symbolic link.

## Arguments

The action reads no argument. The recipe that this action replaces took one,
which let a contributor run the check with changes in their tree, because the
check would otherwise have rewritten their lockfile. The copy makes that
argument meaningless: a tree with changes in it is what the run copies, and
the check never writes where the contributor works.

checklatestdeps[args.none]
The action MUST declare no argument.

## Tool

The action runs the cargo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the terminal of
a contributor. Nextest is a plugin of cargo, and cargo finds it on the path of
the environment that mise sets, at the version that the project pinned. A
cargo that mise does not report stops the action, because provisioning is the
job of mise, and the action installs nothing.

The tool resolves at the project and not in the copy. Mise reads no
configuration that nobody trusted, and the copy is a directory that nobody has
seen before, so the copy could answer for no pin of the project. The program
that the project resolves to is the program that the run starts in the copy.

checklatestdeps[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

checklatestdeps[tool.missing]
A run whose cargo mise does not report MUST stop, and the outcome MUST hold
the error.

## Copy

A run resolves the dependencies of every workspace, which rewrites a lockfile,
and it builds the project. Neither belongs in the checkout of a contributor. A
check that leaves the lockfile of a project rewritten costs the contributor
their afternoon, and it is a check that they will stop running.

The run therefore does its work in a disposable copy of the project. The copy
holds the commit that the project is on, with the changed files of the
checkout over it, so a contributor who raises a floor in a manifest and runs
the check before they commit reads the answer for the new floor. A tree with
changes in it is no reason to stop.

The copy is a working tree of the repository of the project, so a project that
is no repository has no copy, and such a run stops instead of writing where the
contributor works.

The copy holds the tree of the project at its own root, so a path of the copy
and the same path of the project read alike. A finding therefore names the
file that a contributor opens, and nothing of the copy reaches the report.

checklatestdeps[copy.disposable]
A run MUST resolve and test in a copy of the project, and MUST leave the
working tree of the project as it was.

checklatestdeps[copy.paths]
A finding MUST name its file with the path that the file has in the project.

checklatestdeps[copy.unavailable]
A run in a project that is not a git repository, and a run whose copy cannot
be created, MUST stop, and the outcome MUST hold the error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and cargo resolves and tests one workspace at a time.
A run therefore covers every workspace root of the project. The harness of a
project usually depends on the crates of the project by path, so the run there
confirms that the harness builds with the newest versions of everything behind
those paths.

A workspace that the run cannot discover stops it, because a run that passed
over a workspace would hide every failure of that workspace behind a green
result. So does a workspace root that the copy does not hold, which is a
project that moved while the run was on.

checklatestdeps[roots.all]
A run MUST cover every workspace root of the project.

checklatestdeps[roots.error]
A run whose workspace roots cannot be discovered, and a run whose workspace
root the copy does not hold, MUST stop, and the outcome MUST hold the error.

## Update

The run asks cargo to resolve the dependencies of every workspace to the
newest versions that the manifests allow. That resolution is the question of
the action: a manifest that names `1.2` promises that every later `1.x` works,
and only a run against the newest of them can confirm the promise.

The resolution runs on the default toolchain of the project, because a
contributor gets the resolution that their cargo produces.

An update that ends without success is an answer about the project and not a
failure of the run. A dependency that no version can satisfy, and a manifest
that cargo refuses, are both problems that somebody has to solve, so the run
reports what cargo wrote as a finding at the manifest of that root. Such a run
runs no test: the resolution that a test would answer for never came about,
and a test that ran anyway would answer for a lockfile that nobody wrote.

checklatestdeps[update.operation]
A run MUST ask cargo to update the dependencies of a workspace root to the
newest versions that its manifests allow, on the default toolchain of the
project, and MUST NOT change any other option of cargo.

checklatestdeps[update.failed]
An update that ends without success MUST fail the run with a finding at the
manifest of its root that holds what cargo wrote, and that run MUST NOT test.

## Tests

The tests answer the question that the update raised. The run tests every
workspace with nextest, which builds every target of every package with every
feature and runs the tests the way the configuration of the project says.

The build gets the versions that the update resolved and no others. A build
that resolved for itself would answer for a set of versions that nobody read,
so cargo refuses such a build, and the run reports that it has no answer.

A test that failed and a diagnostic of a build that did not finish are both
answers about the newest versions, and both travel as findings, so a run that
gets either of them fails. A workspace without a test is not a failure: a
project can keep its tests in one workspace and its harness in another, so
such a workspace ran no test, and the count of the run says so.

A run that nextest leaves without an answer stops the action. Such a run
examined nothing that the action can report, and an answer built on it would
hide every failure behind a green result.

checklatestdeps[tests.locked]
A run MUST test a workspace root with the versions that the update resolved
for it.

checklatestdeps[tests.passed]
A run whose updates all succeeded and whose nextest reports no failure and
whose cargo reports no diagnostic at any root MUST pass, and the outcome MUST
say how many workspaces the run updated and how many tests it ran.

checklatestdeps[tests.none]
A workspace without a test MUST count as a workspace that ran no test, and
MUST NOT fail the run.

checklatestdeps[tests.failed]
A test that failed MUST fail the run, and the outcome MUST hold the finding of
the test. A diagnostic of the compiler MUST fail the run, and the outcome MUST
hold the finding of the diagnostic.

checklatestdeps[tests.error]
A run of nextest that leaves the action without an answer MUST stop the run,
and the outcome MUST hold the error.

[nextest]: https://nexte.st
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
