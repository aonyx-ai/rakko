# Rakko Cargo

`rakko-cargo` carries the machinery that the actions which wrap [cargo] share.
Cargo does several jobs of project maintenance — it formats Rust files with
rustfmt, it lints them with clippy, and it runs the tests with nextest — and
one action wraps each job. Every one of those actions asks the same questions,
so this crate answers them once: does the project hold Rust code, which cargo
runs here, which workspaces make up the project, which toolchain does a job
need, and what did cargo report about the build?

The crate reads what cargo reports as data. Cargo writes the diagnostics of a
build as JSON when a run asks for that format, and the shape of that JSON
belongs to a version of cargo, so one place that reads it is one place that a
new version can break. The pin softens the risk further: a new shape arrives
with a new version, a new version arrives with a pull request, and a report
that the crate does not recognize shows as a red pull request instead of
passing quietly.

The crate judges nothing. It reports what cargo said, and the action that
asked for the run decides what the answer means for its outcome.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Look

The look tells whether cargo has anything to do in a project. It is cheap, and
it runs before the tool resolves, so that a broad bundle stays safe: a project
without Rust code and without a cargo skips visibly instead of stopping over a
tool that it has no reason to install.

The look searches for a manifest, which is a file named `Cargo.toml`. It reads
hidden directories, because a project can keep a package in one. It does not
read the `.git` entry, which holds no file of the project. It does not read a
directory named `target`, because that is where cargo builds, and the
manifests that a build copies there belong to no package of the project. It
follows no symbolic link, so that a cycle of links cannot trap it.

cargo[look.manifest]
The crate MUST report whether the project holds a file named `Cargo.toml`,
below the root that the caller names.

cargo[look.git]
The look MUST NOT read the `.git` entry of the project.

cargo[look.target]
The look MUST NOT read a directory named `target`.

cargo[look.links]
The look MUST NOT follow a symbolic link.

cargo[look.unreadable]
A directory that the look cannot read MUST count as a directory that holds a
manifest. A look that cannot prove absence must not hide a real check behind a
skip.

## Tool

The cargo that runs is the cargo that mise installed for the project, at the
version that the project pinned, so a run reaches the same program as the
editor and the terminal of a contributor. Mise installs the Rust toolchain
through rustup, so the program that it reports is the proxy of rustup, and
that proxy selects a toolchain by the environment that mise sets and by an
argument that a run can add. The crate installs nothing: provisioning is the
job of mise, and a cargo that Rakko installed would run at a version that
nothing pinned.

cargo[tool.resolve]
The crate MUST resolve `cargo` through mise for the project whose root the
caller names, and a run MUST start the program that mise reports.

cargo[tool.missing]
The crate MUST report an error that names the tool when mise reports no cargo,
and MUST NOT install cargo.

## Roots

A project can hold more than one workspace. The harness of a project is a
package of its own, outside the workspace of the crates that it maintains, and
a job that stops at the workspace in the project root leaves the harness
unchecked. Cargo works on one workspace at a time, so the crate finds every
workspace root under the project, and an action runs its job at each of them.

A root is a manifest that cargo treats as the root of a workspace: the
manifest of a workspace, or the manifest of a package that belongs to no
workspace. Cargo decides, and the crate asks it. For a manifest, cargo names
the workspace that the manifest belongs to and every member of that
workspace, and the members need no question of their own. A manifest that
cargo cannot read stops the discovery, because a discovery that skipped it
would hide a broken manifest behind a green run.

A manifest of the project can belong to a workspace above the project, when
that workspace lists the manifest as a member. A job at that root would work
on files outside the project, so such a manifest stops the discovery as well.

cargo[root.discover]
The crate MUST report every workspace root below the project root, as cargo
names it, once each, in the order of their paths.

cargo[root.member]
A manifest that belongs to a workspace that the crate found MUST NOT count as
a root of its own.

cargo[root.contained]
A manifest that belongs to a workspace whose root the project root does not
contain MUST stop the discovery, and the error MUST name the manifest and the
workspace root.

cargo[root.walk]
The discovery MUST search for manifests with the rules of the look: it MUST
NOT read the `.git` entry, a directory named `target`, or a symbolic link.

cargo[root.directory]
A directory that the discovery cannot read MUST stop the discovery, and the
error MUST name the directory.

cargo[root.manifest]
A manifest that cargo cannot read MUST stop the discovery, and the error MUST
name the manifest and hold what cargo reported.

## Toolchain

Mise pins the Rust toolchain of a project, and it can pin more than one: the
default that builds and tests, and a second channel such as nightly for a job
that needs it. Mise installs a channel as a dated toolchain, and rustup knows
that toolchain by its date. The name of the channel reaches whatever rustup
calls by that name on the machine, which is not the pin, so the crate asks
mise which toolchain the channel resolved to, and a run names that toolchain.

A channel that the project does not pin, and a pinned toolchain that nothing
installed, both stop the caller. Provisioning is the job of mise, and the
crate installs nothing.

cargo[toolchain.resolve]
The crate MUST report the toolchain that mise installed for a channel that the
project pins, by the name that rustup knows it by. A pin that names the
channel and a pin that names a dated toolchain of the channel MUST both
answer.

cargo[toolchain.unpinned]
A channel that the project does not pin MUST produce an error that names the
channel.

cargo[toolchain.uninstalled]
A pinned toolchain that nothing installed MUST produce an error that names the
channel and the toolchain, and the crate MUST NOT install the toolchain.

cargo[toolchain.report]
A report of mise that the crate cannot read MUST produce an error that holds
what mise wrote.

## Runs

Cargo works on the workspace whose directory it runs in, so a run starts in
the directory of the root that the caller names. That directory comes from
the project and never from where the harness started, so a run behaves the
same from every directory of the tree, and a command line reads the same for
every root.

A run that needs a toolchain other than the default names it in the form that
the proxy of rustup reads, which is the name of the toolchain behind a plus
sign, in front of every other argument. The argument selects which cargo
answers, and it changes nothing about what that cargo does.

cargo[run.directory]
A run MUST start cargo in the directory of the root that the caller names.

cargo[run.toolchain]
A run that names a toolchain MUST give it to cargo as the first argument, in
the form that rustup reads.

## Diagnostics

Cargo reports the diagnostics of a build as one JSON document per line on its
standard output when a run asks for that format. A diagnostic carries a level,
a code when the compiler assigned one, a message, and the spans of the source
that it points at, one of which is primary. Cargo closes the stream with a
line that says whether the build finished with success.

The crate reads the lines that carry a diagnostic at the level of a warning or
an error, because those are the problems of a project. A note and a help line
explain a diagnostic above them, and a line that ends a failed build adds
nothing to the errors that it follows. Everything else is ignored, so a line
that a new version adds does not break the reading.

Cargo compiles a library once for the crate and once for its tests, and it
reports a diagnostic once per target that compiles the file. The crate keeps
one, because a reader wants to hear about a problem once.

The paths that cargo writes are relative to the root that cargo checked, and
not to the project, so a finding needs the root to name its file.

cargo[diagnostic.read]
The crate MUST read every compiler message at the level of a warning or an
error from the output of a run, with its level, its code when it has one, its
message, and its primary span when it has one.

cargo[diagnostic.ignore]
The crate MUST ignore a line that is not a compiler message, and a compiler
message at another level.

cargo[diagnostic.once]
A diagnostic that cargo reports more than once MUST count once.

cargo[diagnostic.finished]
The crate MUST report whether cargo said that the build finished with success,
and MUST report that cargo did not say when the line is absent.

cargo[diagnostic.finding]
A diagnostic MUST become a finding with the message of the compiler, followed
by the code when the diagnostic has one, at the range of its primary span,
with the path relative to the project root.

cargo[diagnostic.foreign]
A diagnostic without a span, and a diagnostic whose path the project does not
contain, MUST become a finding at the level of the project. The message MUST
name the path when there is one.

## Paths

Cargo writes some paths relative to the root that it works on, and some tools
that cargo runs write absolute paths. A finding names its file relative to the
project root, so that a reader, a machine, and a code host see the same path
for the same file, and the crate turns both forms into that name.

cargo[path.relative]
The crate MUST report a path that cargo wrote relative to the project root,
whether cargo wrote it relative to a root or absolute.

cargo[path.foreign]
A path that the project root does not contain MUST get no relative name.

[cargo]: https://doc.rust-lang.org/cargo/
[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
