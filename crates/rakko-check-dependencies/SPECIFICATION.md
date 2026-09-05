# Rakko Check Dependencies

`rakko-check-dependencies` provides the action that examines the dependencies
of a project with [cargo-deny]. A project depends on code that it did not
write, and cargo-deny answers three questions about that code: whether every
crate carries a license that the project accepts, whether the crates come from
a registry that the project trusts, and whether the graph holds a shape that
the project banned, such as two versions of one crate or a version requirement
that accepts any future release. The action wraps the cargo-deny that mise
pinned for the project, so a run agrees with a contributor that runs cargo-deny
bare.

Cargo-deny is its own program, and not a subcommand that cargo carries, so the
action starts it directly. Cargo answers a different question for the same run:
which workspaces make up the project. Cargo-deny works on one workspace at a
time, so the action asks cargo for the workspace roots and checks each of them.

Cargo-deny weighs each of its checks with the level that the project gave it:
`deny` for a shape that must not appear, `warn` for one that a maintainer wants
to read about, and `allow` for one that the project does not care about. The
level is the configuration, and this action reads it as the answer that the
project already gave. An error fails a run, a warning does not, and a passing
run says how many warnings it read, so the middle level keeps the meaning that
the project gave it.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

checkdependencies[name]
The action MUST identify itself as `check-dependencies`.

## Applicability

The action applies to a project that holds a manifest of cargo. The
examination is a cheap look of the action's own, and it runs before the tools
resolve, so that a broad bundle stays safe: a project without Rust code and
without a cargo skips visibly instead of stopping over a tool that it has no
reason to install.

The look reads hidden directories, because a project can keep a package in
one. It does not read the `.git` entry, which holds no file of the project,
and it does not read a directory named `target`, where cargo builds. It
follows no symbolic link, so that a cycle of links cannot trap it.

checkdependencies[skip.missing]
A run in a project that holds no file named `Cargo.toml` MUST report that the
action does not apply, and MUST NOT resolve a tool. The reason MUST name what
the run looked for.

checkdependencies[skip.git]
The examination MUST NOT read the `.git` entry of the project.

checkdependencies[skip.target]
The examination MUST NOT read a directory named `target`.

checkdependencies[skip.links]
The examination MUST NOT follow a symbolic link.

## Arguments

The action reads no argument. A run only reports. Cargo-deny repairs nothing
that it finds, because every answer to a finding is a decision: a dependency
goes, a version pin moves, or the project writes down that it accepts what
cargo-deny reported.

checkdependencies[args.none]
The action MUST declare no argument.

## Tools

A run needs two programs, and mise installed both for the project, at the
versions that the project pinned. Cargo-deny does the checking, and cargo
describes the workspaces that it checks. A program that mise does not report
stops the action, because provisioning is the job of mise, and the action
installs nothing.

checkdependencies[tool.deny]
A run that applies MUST resolve `cargo-deny` through mise for the project of
the run, and MUST run the program that mise reports.

checkdependencies[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

checkdependencies[tool.missing]
A run whose tool mise does not report MUST stop, and the outcome MUST hold the
error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and cargo-deny checks one workspace at a time. A run
therefore checks every workspace root of the project, and a workspace that the
run cannot discover stops it, because a run that skipped a workspace would
hide every dependency of that workspace behind a green result.

A run names every member of the workspace to cargo-deny. Cargo-deny takes the
manifest that it starts at as the only root of the graph, and a workspace
whose root manifest is a package of its own would then contribute that package
and nothing else, so a member that no other member depends on would leave the
check. The option selects how much of the workspace the run covers, in the way
that a run of clippy over every target does, and it changes nothing about what
cargo-deny does with what it collected.

checkdependencies[roots.all]
A run MUST check every workspace root of the project.

checkdependencies[roots.members]
A run MUST name every member of a workspace to cargo-deny.

checkdependencies[roots.error]
A run whose workspace roots cannot be discovered MUST stop, and the outcome
MUST hold the error.

## Runs

A run asks cargo-deny for three of its four checks: the bans, the licenses,
and the sources. The fourth check reads the advisory database of RustSec,
which cargo-deny fetches over the network and keeps outside the project. That
check answers a different question, it needs a resource that the other three
do not, and it reports a project that stood still as broken on the day that an
advisory lands. It belongs to an action of its own, and this action names the
three checks that read the project alone.

A run starts cargo-deny in the directory of the workspace root that it checks.
Cargo-deny then reads the manifest of that workspace, and it looks for its
configuration from that directory upwards, so a workspace that carries a
`deny.toml` of its own is checked with that file, and one that carries none is
checked with the file of the project above it.

A run asks for the report in the JSON format. Cargo-deny draws a block per
finding for a reader by default, with the source of the manifest and the
inclusion graph of the crate in it, and the same run writes the findings as
data on request. Each of them then carries the check, the level, the message,
and the crates in fields instead of in a block that a reader has to take
apart. The format also protects the run from the environment, because the
default format changes on a terminal and on a build server.

checkdependencies[run.checks]
A run MUST ask cargo-deny for the bans, the licenses, and the sources check,
and MUST NOT ask for the advisories check.

checkdependencies[run.directory]
A run MUST start cargo-deny in the directory of the workspace root that it
checks.

checkdependencies[run.structured]
A run MUST ask cargo-deny for its report in the JSON format.

## Configuration

The configuration of the project is the source of truth, and cargo-deny reads
it without help from the action. A configuration that cargo-deny rejects stops
the run, because cargo-deny checks nothing at all in that case, and a report
that never arrived says nothing about the project.

A project that wrote no configuration is checked with the defaults of
cargo-deny, and those defaults accept no license, so such a project fails and
says why. That is the answer that a bare cargo-deny gives, and it is a true
statement about a project that has not said which licenses it accepts. The
action states nothing in place of the project.

checkdependencies[check.configuration]
A run whose cargo-deny rejects the configuration of the project MUST stop, and
the error MUST hold what cargo-deny wrote about it.

## Check

Cargo-deny reads the manifests and the lock file of a workspace and reports
what it recognized. Nothing about the source of the project changes, whatever
the run finds.

Cargo-deny weighs every report with a level, and the level comes from the
configuration of the project. An error is a shape that the project said must
not appear, and a run that reports one fails. A warning is a shape that the
project asked to read about and not to fail over, and a run that reports one
passes. A project that turned a check off gets neither, and a project that
wants a warning to fail raises it to an error in its configuration. The action
therefore reads the weight that the project gave and adds none of its own.

A warning that no outcome carried would be a warning that nobody reads, so a
passing run says how many of them the report held, next to the number of
workspaces that the run checked. A reader who wants the warnings themselves
runs cargo-deny bare, which is the tool that wrote them.

A finding names the workspace that the error came from, because a report of
cargo-deny names no file that the project holds. The place that cargo-deny
underlines is a line of a lock file, or of a manifest that lies in the
registry cache of the machine and not in the project at all, so a finding that
claimed a path would name a file that a reader cannot open. The workspace is
what the run knows, and a finding names it and nothing more.

The message of a finding holds the check that reported it, what cargo-deny
wrote about it, and the crates that cargo-deny named for it. The crates matter
because the message alone often does not name them: cargo-deny writes the
crate of a rejected license into the path of the block that it draws, and the
JSON report keeps it in the graph of the finding instead.

A report that the action cannot read stops the run, and so does a run that
cargo-deny could not finish. Cargo-deny ends every run that reached its checks
with a summary of them, so a report without that summary belongs to a run that
stopped before it had checked the workspace, and an outcome built on such a
report would describe a workspace that nothing examined.

checkdependencies[check.read]
A run MUST NOT change the source of the project.

checkdependencies[check.passed]
A run whose cargo-deny reports no error at any root MUST pass, and the outcome
MUST say how many workspaces the run checked and how many warnings it read.

checkdependencies[check.finding]
An error that cargo-deny reports MUST produce a finding, and the message MUST
hold the check that reported it, what cargo-deny wrote, and the crates that
cargo-deny named for it.

checkdependencies[check.location]
A finding MUST name the workspace that the error came from: the directory of
the workspace root relative to the project root, or the project itself when
the workspace root is the project root.

checkdependencies[check.failed]
A run whose cargo-deny reports an error at any root MUST fail, and the outcome
MUST hold one finding per error.

checkdependencies[check.warning]
A warning that cargo-deny reports MUST NOT produce a finding, and MUST NOT
fail a run.

checkdependencies[check.incomplete]
A run whose cargo-deny wrote no summary of its checks MUST stop, and the error
MUST name the workspace root and hold what cargo-deny wrote.

checkdependencies[check.unreadable]
A run whose report holds a record that the action cannot read MUST stop, and
the error MUST name the workspace root and hold the record.

[cargo-deny]: https://embarkstudios.github.io/cargo-deny/
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
