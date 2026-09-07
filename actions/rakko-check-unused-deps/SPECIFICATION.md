# Rakko Check Unused Dependencies

`rakko-check-unused-deps` provides the action that finds the dependencies
which a project declares and never uses. A dependency that nothing reaches
still costs the project: it is built, it is audited, it is updated, and it
widens the surface that a reader of the manifest has to understand. Nothing
in the manifest says whether a dependency is used, so the answer comes from a
build.

The action wraps [cargo-udeps], which reads the record that the compiler
writes about the crates each target actually loaded and holds it against the
dependencies of the manifest. The action selects the operation and translates
what cargo-udeps reported into an outcome.

Cargo-udeps asks the compiler for that record with an unstable option, so the
build runs on the nightly toolchain that the project pins.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

checkunuseddeps[name]
The action MUST identify itself as `check-unused-deps`.

## Applicability

The action applies to a project that holds a manifest of cargo. The
examination is a cheap look that runs before the tool resolves, so that a
broad bundle stays safe: a project without Rust code and without a cargo
skips visibly instead of stopping over a tool that it has no reason to
install.

The look reads hidden directories, because a project can keep a package in
one. It does not read the `.git` entry, which holds no file of the project,
and it does not read a directory named `target`, where cargo builds. It
follows no symbolic link, so that a cycle of links cannot trap it.

checkunuseddeps[skip.missing]
A run in a project that holds no file named `Cargo.toml` MUST report that the
action does not apply, and MUST NOT resolve the tool. The reason MUST name
what the run looked for.

checkunuseddeps[skip.git]
The examination MUST NOT read the `.git` entry of the project.

checkunuseddeps[skip.target]
The examination MUST NOT read a directory named `target`.

checkunuseddeps[skip.links]
The examination MUST NOT follow a symbolic link.

## Arguments

The action reads no argument. A run only reports. Cargo-udeps repairs nothing
that it finds, because every answer to an unused dependency is a decision:
the dependency goes, the code that was meant to use it arrives, or the
project writes down that it accepts the report.

checkunuseddeps[args.none]
The action MUST declare no argument.

## Tool

The action runs the cargo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the terminal of
a contributor. Cargo-udeps is a plugin of cargo, and cargo finds it on the
path of the environment that mise sets, at the version that the project
pinned. A cargo that mise does not report stops the action, because
provisioning is the job of mise, and the action installs nothing.

Cargo-udeps reads which crates a target loaded from a record that only an
unstable option of the compiler writes, so the build needs the nightly
channel. A stable toolchain refuses the option and reports nothing about the
dependencies at all. The action therefore runs cargo on the nightly toolchain
that the project pins, and a project that pins none stops the action, for the
same reason that a missing cargo does.

checkunuseddeps[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

checkunuseddeps[tool.missing]
A run whose cargo mise does not report MUST stop, and the outcome MUST hold
the error.

checkunuseddeps[tool.toolchain]
A run MUST run cargo on the toolchain that mise installed for the `nightly`
channel of the project.

checkunuseddeps[tool.unpinned]
A run in a project whose `nightly` channel mise does not report as pinned and
installed MUST stop, and the outcome MUST hold the error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and cargo works on one workspace at a time. Each
workspace declares its own dependencies, so a run examines every workspace of
the project.

A workspace that the run cannot discover stops the run, because a run that
passed over such a workspace would hide every unused dependency of that
workspace behind a green result.

checkunuseddeps[roots.every]
A run MUST examine every workspace root of the project.

checkunuseddeps[roots.error]
A run whose workspace roots cannot be discovered MUST stop, and the outcome
MUST hold the error.

## Check

A dependency is used when a target of the project loads it, so the run has to
build every target that the workspace can build. It examines every target of
every package with every feature, because a dependency that only a test
reaches, and a dependency that only a feature reaches, are used as much as
one that the library reaches. It examines every package of the workspace, and
not only the package that the manifest of the root describes, so that a
member which nothing else depends on answers as well. Nothing about the
project changes, whatever the run finds.

The action asks cargo-udeps for its report as JSON, and cargo for its own
report as JSON, because it reads both as data. This selects the presentation
of the reports and not the behavior of the tools: which dependencies a run
passes over comes from the configuration of the project alone.

Cargo-udeps names a dependency that no target loaded, together with the
manifest that declares it and the kind of dependency that the manifest
declares it as. Each of those becomes a finding at the manifest, because the
report names no line, and the message names the dependency and its kind, so
that a reader finds the entry to remove.

A build that does not finish leaves cargo-udeps without an answer, and it
writes no report. The compiler said why, so every diagnostic of that build
becomes a finding instead. A build that finished carries the answer of
cargo-udeps, and the diagnostics of that build belong to the action that
lints the code, so they stay out of the outcome here.

A report that the action does not recognize stops the run. A run that ended
without success and named no unused dependency and no diagnostic, and a run
that ended with success and wrote no report of cargo-udeps, both leave the
action without an answer, and a result built on such a run would hide every
unused dependency behind a green result.

checkunuseddeps[check.read]
A run MUST NOT change the project.

checkunuseddeps[check.operation]
A run MUST ask the compiler to examine every target of every package of the
workspace with every feature, and MUST ask cargo-udeps and cargo for their
reports as JSON. It MUST NOT change any other option of either tool.

checkunuseddeps[check.passed]
A run whose cargo-udeps reports no unused dependency at any root MUST pass,
and the outcome MUST say how many workspaces the run examined.

checkunuseddeps[check.finding]
An unused dependency MUST produce a finding at the manifest that declares it,
with the path relative to the project root, and the message MUST name the
dependency and the kind of dependency that the manifest declares it as.

checkunuseddeps[check.foreign]
An unused dependency whose manifest lies outside the project MUST produce a
finding that belongs to the project, and the message MUST name the manifest.

checkunuseddeps[check.failed]
A run whose cargo-udeps reports an unused dependency at any root MUST fail,
and the outcome MUST hold one finding per unused dependency.

checkunuseddeps[check.diagnostic]
A run whose build did not finish MUST hold one finding per diagnostic of the
compiler, at the range that the compiler reports, with the message of the
compiler and the code of the diagnostic, and with the path relative to the
project root. A run whose build finished MUST NOT report a diagnostic.

checkunuseddeps[check.unrecognized]
A run that ends without success and reports no unused dependency and no
diagnostic, or that ends with success without a report of cargo-udeps, MUST
stop the run, and the error MUST hold what the tools wrote.

checkunuseddeps[check.unreadable]
A report that holds a record which the action cannot read MUST stop the run,
and the error MUST name the root and hold the record.

[cargo-udeps]: https://github.com/est31/cargo-udeps
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
