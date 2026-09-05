# Rakko Build Internal Docs

`rakko-build-internal-docs` provides the action that builds the internal
documentation of a project with [rustdoc]. The internal documentation
describes the code to whoever maintains it, so it covers the private items as
well as the public ones. The documentation that a project writes for the
people who use it is a different task, with a tool of its own.

The build is also the only examination that this documentation gets. Rustdoc
resolves the links between items while it renders them, and a link that names
nothing is a warning that no other tool reports. The action wraps the cargo
that mise pinned for the project, so a run agrees with a contributor that runs
`cargo doc` bare.

Cargo reports the diagnostics of a build as JSON when a run asks for that
format, and the action reads that JSON through the shared cargo machinery.
Every diagnostic that rustdoc raises is a problem of the project, whatever its
level, so a run with a warning fails. The recipe that this action replaces
named that goal and never reached it. Nothing denied the rustdoc lints there,
so a broken link left the recipe green. The action reads the report instead of
changing what rustdoc does.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.
The name also separates this documentation from the documentation that a
project publishes for its users, which a task of its own builds.

buildinternaldocs[name]
The action MUST identify itself as `build-internal-docs`.

## Applicability

The action applies to a project that holds a manifest of cargo. The
examination is a cheap look that runs before the tool resolves, so that a
broad bundle stays safe: a project without Rust code and without a cargo skips
visibly instead of stopping over a tool that it has no reason to install.

The look reads hidden directories, because a project can keep a package in
one. It does not read the `.git` entry, which holds no file of the project,
and it does not read a directory named `target`, where cargo builds. It
follows no symbolic link, so that a cycle of links cannot trap it.

buildinternaldocs[skip.missing]
A run in a project that holds no file named `Cargo.toml` MUST report that the
action does not apply, and MUST NOT resolve the tool. The reason MUST name
what the run looked for.

buildinternaldocs[skip.git]
The examination MUST NOT read the `.git` entry of the project.

buildinternaldocs[skip.target]
The examination MUST NOT read a directory named `target`.

buildinternaldocs[skip.links]
The examination MUST NOT follow a symbolic link.

## Arguments

The action reads no argument. A run builds the documentation and reports what
rustdoc said about it. Rustdoc repairs nothing that it finds, because every
broken link is a decision: the link names another item, the item that it names
arrives, or the text stops being a link.

buildinternaldocs[args.none]
The action MUST declare no argument.

## Tool

The action runs the cargo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. A cargo that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

buildinternaldocs[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

buildinternaldocs[tool.missing]
A run whose cargo mise does not report MUST stop, and the outcome MUST hold
the error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and cargo documents one workspace at a time. A run
therefore documents every workspace root of the project, and a workspace that
the run cannot discover stops it, because a run that skipped a workspace would
hide every problem of that workspace behind a green result.

buildinternaldocs[roots.all]
A run MUST document every workspace root of the project.

buildinternaldocs[roots.error]
A run whose workspace roots cannot be discovered MUST stop, and the outcome
MUST hold the error.

## Build

Rustdoc documents every package of the workspace, with every feature and with
the private items, so that the documentation of a member that nothing depends
on, the documentation behind a feature that is off by default, and the
documentation of an item that only a maintainer reads are all built and
examined. It documents no dependency, because the documentation of a
dependency belongs to the project that publishes it.

The documentation goes where cargo builds, and the sources of the project stay
as they are, whatever the run finds. The action asks cargo for its report as
JSON, because it reads the report as data. This selects the presentation of
the report and not the behavior of the tool: which lints apply, and at which
level, comes from the configuration of the project alone.

Every diagnostic becomes a finding at the range that the compiler named, with
the message of the compiler and the code that names the lint, so that a reader
can look the lint up. A warning and an error are both problems of the
documentation, so a run with either fails.

A report that the action does not recognize stops the run. A run that ended
without success and named no diagnostic, and a run that ended with success
without saying that the build finished, both wrote a report that the action
could not read, and an answer built on such a report would hide every problem
behind a green result.

buildinternaldocs[build.sources]
A run MUST NOT change a source of the project.

buildinternaldocs[build.operation]
A run MUST ask rustdoc to document every package of the workspace, with every
feature, with the private items, and without the dependencies, and MUST ask
cargo for its report as JSON. It MUST NOT change any other option of rustdoc.

buildinternaldocs[build.passed]
A run whose rustdoc reports no diagnostic at any root MUST pass, and the
outcome MUST say how many workspaces the run documented.

buildinternaldocs[build.diagnostic]
A diagnostic of rustdoc MUST produce a finding at the range that the compiler
reports, with the message of the compiler and the code of the diagnostic, and
with the path relative to the project root.

buildinternaldocs[build.failed]
A run whose rustdoc reports a diagnostic at any root MUST fail, and the
outcome MUST hold one finding per diagnostic.

buildinternaldocs[build.unrecognized]
A cargo run that ends without success and reports no diagnostic, or that ends
with success and does not report that the build finished, MUST stop the run,
and the error MUST hold what cargo wrote.

buildinternaldocs[build.unreadable]
A report that holds a record of cargo which the action cannot read MUST stop
the run, and the error MUST name the root and hold the record.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[rustdoc]: https://doc.rust-lang.org/rustdoc/
[tracey]: https://tracey.bearcove.eu/
