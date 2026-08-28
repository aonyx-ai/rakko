# ADR-009: Harness Entry Point

## Status

Accepted

## Context

Every project that adopts Rakko ships a harness: a small binary crate that
mounts the actions the project runs. Something has to start it, and where
that something lives decides more than it appears to.

The [vision][vision] answers with `cargo ra`, a cargo alias, and the
[glossary][glossary] carries `cargo ra` as a defined term. Nothing
implements it yet. The mechanism and the name are both still free, and they
stop being free on the day the first project adopts them: the entry point
goes into the README of every repository in the fleet, into the workflow
files, and into the fingers of everyone who uses Rakko. Renovate cannot
change it later, because it is neither a version nor a dependency.

A cargo alias carries a constraint that is easy to miss. An alias prepends
arguments and nothing more, so a relative `--manifest-path` resolves against
the working directory of the caller rather than against the configuration
file that declares it. The command then works in the root of the project and
fails in every subdirectory. Only `--package` selects a target independently
of where the user stands, and `--package` reaches only the members of the
workspace that cargo finds by walking up. A cargo alias therefore requires
the harness to be a member of the workspace of the project.

That requirement is the problem. Aonyx ships libraries, and a library
promises its consumers a minimum supported Rust version. A harness in the
workspace shares one `Cargo.lock` with the crates around it, and one lockfile
means one resolution: a dependency that the harness and the library both use
resolves to a version that satisfies both. The maintenance tool then
constrains what the library can promise. Rakko requires 1.88.0 today, against
1.85.0 in the projects it would maintain, and that gap widens rather than
closes, because a maintenance tool adopts new language features on a schedule
that a published library deliberately does not. A tool that raises the floor
of the code it maintains has the dependency backwards.

Windows decides the rest. Aonyx has projects that need testing on Windows and
cannot be tested there today, so the entry point must work natively on
Windows from the first day rather than eventually. [ADR-003][adr-003] already
chose mise partly because it runs on Windows where Nix does not, but the
guarantees of mise are narrower there than elsewhere. Environment variables
from `mise.toml` do not reach a native Windows shell unless the command runs
through `mise exec` or `mise run`. A directory that mise adds to `PATH`, and
any short script inside it, therefore work on macOS and Linux and nowhere
else. A script with a shebang line fails for a second reason, because Windows
derives executability from a file extension and not from a first line.

## Decision

The mise environment is the entry point of a harness, and the harness is a
standalone package.

1. **The harness is not a member of the workspace of the project.** It is a
   Cargo package of its own, with its own `Cargo.lock`, in a directory of the
   project. Its dependencies resolve separately, so nothing the harness needs
   reaches the crates that the project publishes. A library keeps the minimum
   supported Rust version that it chose, and Rakko cannot move it.

2. **A mise task runs the harness.** `mise run rakko` is the canonical form,
   in the documentation, in the workflow files, and in every instruction that
   a project gives a contributor. A task resolves the manifest of the harness
   from the root of the project instead of from the working directory, so the
   command behaves the same everywhere in the tree. That is the property a
   cargo alias loses the moment the harness leaves the workspace.

3. **The canonical form runs on every platform that mise supports.** This is
   a requirement on the mechanism and not an aspiration. It rules out a
   shebang script, which Windows does not execute, and it rules out an entry
   point that depends on mise injecting a directory into `PATH`, which a
   native Windows shell does not receive. What is written down is what a
   contributor on Windows types.

4. **A short form is a convenience of a platform, not part of the contract.**
   Where the environment can supply one, it does, and nothing depends on it.
   The documentation, the workflow files, and the error messages always write
   the canonical form, so a reader is never shown a name that their own
   platform does not have.

The decision stops at the mechanism and the layer that owns it. The directory
that holds the harness, the definition of the task, whether the build
directory is shared across projects, and whether a short form exists on
Windows are all open.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### A Cargo Alias

`cargo ra`, or the clearer `cargo rakko`, needs no mise at all. It works in
cmd, in PowerShell, and in Git Bash without any environment to enter, which
is a better Windows story than the one this decision accepts, and it puts the
entry point in the tool that already builds the harness.

It requires the harness to be a workspace member, for the reason the context
gives, and workspace membership is what couples the lockfile and the minimum
supported Rust version. That coupling is the cost this decision exists to
avoid, and no arrangement of the alias removes it. The name `ra` fails on its
own terms as well: it abbreviates rust-analyzer everywhere else in the Rust
ecosystem, and two letters give a stranger nothing to look up when they meet
the command in a failing job.

### The Harness in the Workspace, Excluded From the Default Members

A workspace can name its `default-members`, and a harness left out of that
list is not built by a bare `cargo build`, `cargo test`, or `cargo clippy`.
The harness keeps the shared target directory, `cargo rakko` keeps working
through `--package`, and CI stops paying to compile a maintenance tool on
every run of the product.

This addresses the build cost and leaves the coupling untouched. Membership
is what produces one lockfile, one lockfile is what unifies the dependency
versions, and the unified versions are what reach the minimum supported Rust
version of a published library. A flag that changes what cargo builds by
default changes nothing about what cargo resolves.

### A Short Script on the PATH of the Environment

Mise can add a directory to `PATH`, and a two-line script in that directory
gives a one-word command that needs no `mise run` prefix. This repository
already uses the mechanism to expose tracey.

Mise does not supply its environment to a native Windows shell, so the script
is unreachable on the platform this decision has to support. Shipping a
second script with a Windows extension repairs the reachability and leaves
two files to keep in agreement, and the entry point of the fleet is the wrong
place for a mechanism that is different on each platform. The mechanism
survives as an optional short form, where point four of the decision puts it.

### An Installed Launcher Binary

Mise could install a small `cargo-rakko` binary through its cargo backend.
Cargo would then find it as a subcommand, `cargo rakko` would work from any
directory, and the harness could stay outside the workspace. The launcher
would be compiled rather than interpreted, so no shell would be involved on
any platform.

Cargo discovers a subcommand as `cargo-rakko` with the executable suffix of
the platform, which is `cargo-rakko.exe` on Windows, and the shims that mise
writes there are `.cmd` files by default. The subcommand would therefore be
invisible to cargo on the platform it exists to serve, unless every developer
configured a different shim mode. It also adds an artifact that versions on
its own schedule, in front of a harness whose whole purpose is to be the one
place a project states what it runs.

## Consequences

- A project keeps the minimum supported Rust version that it chose. This is
  the property the decision buys, and it is the one that cannot be recovered
  later without moving the harness anyway, because the coupling is silent
  until a shared dependency raises a floor.
- Mise owns the documented entry point, but not the ability to run a harness.
  The harness stays an ordinary Cargo package, so `cargo run` inside its
  directory works with no environment at all, and `--manifest-path` reaches it
  from the root. What a project adopts here is a convention, and anyone who
  wants a different launcher can write one against the same package. The one
  invocation that membership would have added, and that this decision gives
  up, is `--package` from the root of the project.
- The dependency on mise that does bind is older and larger than this
  decision. An action resolves the tool it wraps through mise, so a harness
  run outside the environment finds actions that cannot find their tools.
  [ADR-003][adr-003] already made that trade, and the entry point neither
  deepens it nor escapes it.
- The harness compiles in its own build directory, so its artifacts are not
  shared with the project and the first run after a change costs a build.
  Cargo can direct those intermediates out of the repository, which turns the
  duplicate tree into a per-user cost instead of a per-project one.
- The harness is invisible to rust-analyzer until a project lists it as a
  linked project. Editor support for the one file that states what a project
  runs is now a line of configuration that a workspace member would not have
  needed.
- A project carries a second `Cargo.lock`, and Renovate has to be pointed at
  it. Two lockfiles is the mechanism that produces the isolation, so this cost
  is the decision working rather than a defect in it.
- A contributor on Windows types the canonical form and gets no short form.
  The cost lands almost entirely on a human at a desktop, because a workflow
  file writes the canonical form on every platform anyway.
- Nothing can go stale. The task builds the harness before it runs it, so a
  change to the harness, to an action, or to a pinned version takes effect on
  the next invocation and any breakage surfaces immediately.
- The entry point no longer names cargo, so `cargo ra` leaves the glossary
  and the vision. The command is longer than the one the vision promised, and
  it says which layer answers.

[adr-003]: 003-mise.md
[glossary]: ../GLOSSARY.md
[vision]: ../VISION.md
