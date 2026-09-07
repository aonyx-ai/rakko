# Rakko Check MSRV

`rakko-check-msrv` provides the action that checks the code of a project
against the oldest Rust toolchain that it promises to compile on. A package
writes that promise as the `rust-version` of its manifest, and whoever depends
on the package reads it as a fact. The compiler is the only thing that can
confirm the fact, so the action runs the compiler on the toolchain that the
promise names.

The action wraps the cargo that mise pinned for the project. Cargo reads the
manifests, and rustup selects the toolchain, so a run agrees with a
contributor who checks the same thing by hand. The action selects the
operation and translates what cargo reported into an outcome.

The check is self-gating. A project that promises nothing has nothing to
confirm, so a run in such a project skips visibly instead of inventing a
toolchain to check against.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

checkmsrv[name]
The action MUST identify itself as `check-msrv`.

## Applicability

The action applies to a project that holds a manifest of cargo and declares a
Rust version in it. The first examination is a cheap look that runs before the
tool resolves, so that a broad bundle stays safe: a project without Rust code
and without a cargo skips visibly instead of stopping over a tool that it has
no reason to install.

The look reads hidden directories, because a project can keep a package in
one. It does not read the `.git` entry, which holds no file of the project,
and it does not read a directory named `target`, where cargo builds. It
follows no symbolic link, so that a cycle of links cannot trap it.

The second examination reads the declaration, and it needs cargo, because
cargo resolves what a package inherits from its workspace. A project whose
packages declare no Rust version promises nothing, and a run there has nothing
to confirm.

checkmsrv[skip.missing]
A run in a project that holds no file named `Cargo.toml` MUST report that the
action does not apply, and MUST NOT resolve the tool. The reason MUST name
what the run looked for.

checkmsrv[skip.git]
The examination MUST NOT read the `.git` entry of the project.

checkmsrv[skip.target]
The examination MUST NOT read a directory named `target`.

checkmsrv[skip.links]
The examination MUST NOT follow a symbolic link.

checkmsrv[skip.undeclared]
A run in a project whose workspaces declare no `rust-version` MUST report that
the action does not apply. The reason MUST name what the run looked for.

## Arguments

The action reads no argument. A run confirms a promise that the project
already wrote, and nothing about that promise is a choice that a caller
makes: the toolchain comes from the manifest, and the compiler decides the
answer.

checkmsrv[args.none]
The action MUST declare no argument.

## Tool

The action runs the cargo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. A cargo that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

The toolchain of the run is the one that the declaration names. Mise pins the
toolchains of a project, so the declared version is a pin like every other
tool, and the action asks mise for it. A project that declares a version and
pins no toolchain for it stops the action, for the same reason that a missing
cargo does. The two places state the same version, and a disagreement between
them stops the run with the version that the run looked for.

checkmsrv[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

checkmsrv[tool.missing]
A run whose cargo mise does not report MUST stop, and the outcome MUST hold
the error.

checkmsrv[tool.toolchain]
A run MUST check a workspace on the toolchain that mise installed for the
`rust-version` that the workspace declares.

checkmsrv[tool.unpinned]
A run in a project whose declared toolchain mise does not report as pinned and
installed MUST stop, and the outcome MUST hold the error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and cargo checks one workspace at a time. Each
workspace makes its own promise, so a run reads the declaration of every
workspace and checks each workspace on its own toolchain.

A workspace that declares nothing is passed over, and the run reports on the
workspaces that it checked. A workspace that the run cannot discover, and a
declaration that the run cannot read, both stop the run, because a run that
passed over such a workspace would hide every problem of that workspace behind
a green result.

checkmsrv[roots.declared]
A run MUST check every workspace root of the project that declares a
`rust-version`, and MUST pass over a root that declares none.

checkmsrv[roots.error]
A run whose workspace roots cannot be discovered, and a run whose declaration
of a workspace cannot be read, MUST stop, and the outcome MUST hold the error.

## Check

The compiler examines every target of every package with every feature, so
that code in a test and code behind a feature answer for the promise as well.
It only reads the code, and it produces no binary, because the question is
whether the toolchain accepts the code. Nothing about the project changes,
whatever the run finds. The action asks cargo for its report as JSON, because
it reads the report as data. This selects the presentation of the report and
not the behavior of the tool: which lints apply, and at which level, comes
from the configuration of the project alone.

Every diagnostic becomes a finding at the range that the compiler named, with
the message of the compiler and the code that names the diagnostic, so that a
reader can look it up. A warning and an error are both answers of the older
compiler about the promise, so a run with either fails. A deprecation that
only the older compiler reports is the clearest example: the code compiles,
and the promise still costs the project something that a reader wants to know
about.

A report that the action does not recognize stops the run. A run that ended
without success and named no diagnostic, and a run that ended with success
without saying that the build finished, both wrote a report that the action
could not read, and an answer built on such a report would hide every problem
behind a green result.

checkmsrv[check.read]
A run MUST NOT change the project.

checkmsrv[check.operation]
A run MUST ask the compiler to examine every target with every feature without
producing a binary, and MUST ask cargo for its report as JSON. It MUST NOT
change any other option of cargo.

checkmsrv[check.passed]
A run whose compiler reports no diagnostic at any root MUST pass, and the
outcome MUST say how many workspaces the run checked.

checkmsrv[check.diagnostic]
A diagnostic of the compiler MUST produce a finding at the range that the
compiler reports, with the message of the compiler and the code of the
diagnostic, and with the path relative to the project root.

checkmsrv[check.failed]
A run whose compiler reports a diagnostic at any root MUST fail, and the
outcome MUST hold one finding per diagnostic.

checkmsrv[check.unrecognized]
A cargo run that ends without success and reports no diagnostic, or that ends
with success and does not report that the build finished, MUST stop the run,
and the error MUST hold what cargo wrote.

checkmsrv[check.unreadable]
A report that holds a record of cargo which the action cannot read MUST stop
the run, and the error MUST name the root and hold the record.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
