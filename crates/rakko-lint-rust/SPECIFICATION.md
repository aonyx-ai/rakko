# Rakko Lint Rust

`rakko-lint-rust` provides the action that lints the Rust code of a project
with [clippy]. The action wraps the cargo that mise pinned for the project, so
a run agrees with the editor and with a contributor that runs clippy bare.
Cargo reads the manifests, selects the lints that the project configured, and
does the linting. The action selects the operation and translates what cargo
reported into an outcome.

Cargo reports the diagnostics of a build as JSON when a run asks for that
format, and the action reads that JSON through the shared cargo machinery.
Every diagnostic that clippy raises is a problem of the project, whatever its
level: the lints that a project allows never appear, and the lints that it
warns about or denies both become findings, so a run with a warning fails.
That is what the recipe achieved by denying every warning, and the action
achieves it by reading the report instead of changing what clippy does.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

The name of the action names the maintenance task and not the tool, so that
the task can change its tool without renaming the command of every project.

lintrust[name]
The action MUST identify itself as `lint-rust`.

## Applicability

The action applies to a project that holds a manifest of cargo. The
examination is a cheap look that runs before the tool resolves, so that a
broad bundle stays safe: a project without Rust code and without a cargo skips
visibly instead of stopping over a tool that it has no reason to install.

The look reads hidden directories, because a project can keep a package in
one. It does not read the `.git` entry, which holds no file of the project,
and it does not read a directory named `target`, where cargo builds. It
follows no symbolic link, so that a cycle of links cannot trap it.

lintrust[skip.missing]
A run in a project that holds no file named `Cargo.toml` MUST report that the
action does not apply, and MUST NOT resolve the tool. The reason MUST name
what the run looked for.

lintrust[skip.git]
The examination MUST NOT read the `.git` entry of the project.

lintrust[skip.target]
The examination MUST NOT read a directory named `target`.

lintrust[skip.links]
The examination MUST NOT follow a symbolic link.

## Arguments

The action reads no argument. Clippy can repair some of what it finds, but a
repair that the compiler applies is a change to the code that a contributor
wants to read before it lands, and a lint that clippy cannot repair needs a
hand either way. An action with no argument tells a user that.

lintrust[args.none]
The action MUST declare no argument.

## Tool

The action runs the cargo that mise installed for the project, at the version
that the project pinned, so a run reaches the same program as the editor and
the terminal of a contributor. A cargo that mise does not report stops the
action, because provisioning is the job of mise, and the action installs
nothing.

lintrust[tool.cargo]
A run that applies MUST resolve `cargo` through mise for the project of the
run, and MUST run the program that mise reports.

lintrust[tool.missing]
A run whose cargo mise does not report MUST stop, and the outcome MUST hold
the error.

## Roots

A project can hold more than one workspace, because the harness of a project
is a package of its own, and cargo lints one workspace at a time. A run
therefore lints every workspace root of the project, and a workspace that the
run cannot discover stops it, because a run that skipped a workspace would
hide every problem of that workspace behind a green result.

lintrust[roots.all]
A run MUST lint every workspace root of the project.

lintrust[roots.error]
A run whose workspace roots cannot be discovered MUST stop, and the outcome
MUST hold the error.

## Check

Clippy examines every target of every package with every feature enabled, so
that a lint in a test or behind a feature is found as well, and it reports
the diagnostics of the compiler. Nothing about the project changes, whatever
the run finds. The action asks cargo for its report as JSON, because it reads
the report as data. This selects the presentation of the report and not the
behavior of the tool: which lints apply, and at which level, comes from the
configuration of the project alone.

Every diagnostic becomes a finding at the range that the compiler named,
with the message of the compiler and the code that names the lint, so that a
reader can look the lint up. A warning and an error are both problems of the
project, so a run with either fails.

A report that the action does not recognize stops the run. A run that ended
without success and named no diagnostic, and a run that ended with success
without saying that the build finished, both wrote a report that the action
could not read, and an answer built on such a report would hide every
problem behind a green result.

lintrust[check.read]
A run MUST NOT change the project.

lintrust[check.operation]
A run MUST ask clippy to examine every target with every feature, and MUST ask
cargo for its report as JSON. It MUST NOT change any other option of clippy.

lintrust[check.passed]
A run whose clippy reports no diagnostic at any root MUST pass, and the
outcome MUST say how many workspaces the run checked.

lintrust[check.diagnostic]
A diagnostic of clippy MUST produce a finding at the range that the compiler
reports, with the message of the compiler and the code of the diagnostic, and
with the path relative to the project root.

lintrust[check.failed]
A run whose clippy reports a diagnostic at any root MUST fail, and the outcome
MUST hold one finding per diagnostic.

lintrust[check.unrecognized]
A cargo run that ends without success and reports no diagnostic, or that ends
with success and does not report that the build finished, MUST stop the run,
and the error MUST hold what cargo wrote.

[clippy]: https://doc.rust-lang.org/clippy/
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
