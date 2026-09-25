# Rakko Set MSRV

`rakko-set-msrv` provides the `set-msrv` command, which sets the minimum
supported Rust version (MSRV) of a project. Three places state that version,
and all three must agree: the `rust-version` of the root manifest, the Rust
pin in `mise.toml` that check-msrv runs the compiler on, and the entry of that
pin in `mise.lock`. When one of them is missed, check-msrv fails, or
`mise install --locked` refuses the lock. The command writes all three in one
run, and then it installs the new toolchain and checks the code on it, so that
one run tells the user whether the project compiles on its new MSRV.

No external tool makes this edit. `mise use` rewrites the whole list of Rust
pins, and `cargo msrv set` refuses the root of a workspace. The command
therefore edits the two configuration files itself, and it asks mise to bring
its own lock in line.

Every requirement in this document has an identifier, and the code that
implements or tests a requirement references the identifier in a comment.
[Tracey] checks that every requirement is implemented and tested. The key
words MUST and MUST NOT have the meaning that [RFC 2119] defines.

## Name

setmsrv[name]
The command MUST identify itself as `set-msrv`.

## Arguments

The user names the version, because the build failure that makes a bump
necessary names it. The version has three parts. Cargo reads `1.88` as
`1.88.0`, but mise resolves a pin of `1.88` to the newest `1.88.x`, so a
version with two parts would check another toolchain than the one that the
manifest promises. Cargo also refuses a version in which a number starts with
a zero, such as `1.089.0`.

The reason replaces the comment above `rust-version`. A comment that does not
match the number is worse than none, so a run cannot keep the old one, and a
blank reason would only delete it. The reason becomes one comment line, and a
comment in TOML ends at the line break and holds no other control character.

setmsrv[args.declare]
The command MUST declare two arguments: `msrv` and `reason`, each holding a
line of text, with documentation.

setmsrv[args.missing]
A run that gives `msrv` or `reason` no value MUST fail the construction of
the arguments, and the failure MUST report the argument.

setmsrv[args.form]
A value for `msrv` that is not three numbers separated by dots, or in which a
number other than `0` starts with a zero, MUST fail the construction of the
arguments, and the failure MUST report the argument.

setmsrv[args.reason]
A value for `reason` that holds nothing but whitespace, or that holds a
control character other than a tab, MUST fail the construction of the
arguments, and the failure MUST report the argument.

## Manifest

The root manifest declares the version in `[workspace.package]`, where the
members inherit it, or in `[package]` when the root is a single package. A
member that declares a version of its own is out of reach. The TOML that Cargo
reads allows no comment inside an inline table, so a manifest that declares
the version in one has no place for the reason.

setmsrv[manifest.version]
A run MUST set the `rust-version` of the `[workspace.package]` table of the
root manifest to the value of `msrv`. When that table declares none, a run
MUST set the `rust-version` of the `[package]` table instead.

setmsrv[manifest.comment]
A run MUST replace the comment lines directly above `rust-version` with one
comment line that holds the reason.

setmsrv[manifest.undeclared]
A run in a project whose root manifest declares no `rust-version` MUST fail
before it changes a file, and the failure MUST name the manifest.

setmsrv[manifest.inline]
A run in a project whose root manifest declares `rust-version` in an inline
table MUST fail before it changes a file, and the failure MUST name the
manifest.

## Pin

A project pins the toolchain that check-msrv uses as a Rust pin in
`mise.toml`, and that pin repeats the `rust-version` of the manifest. The pin
that matches the current version is therefore the pin to change. A project
without such a pin has not adopted check-msrv, and adding a pin is a step of
its own: the first pin of the list is the default toolchain, and a pin in the
wrong position changes it.

A project that pins its MSRV beside its default toolchain pins the same
version twice when the two agree. The first pin is the default, so a run moves
the last pin that matches, and the default stays where it is. A project whose
only pin at that version is the default builds on its MSRV, and its default
moves with the MSRV.

setmsrv[pin.version]
A run MUST set the last Rust pin in the `[tools]` table of `mise.toml` whose
version is the current `rust-version` to the value of `msrv`, and MUST leave
every other pin as it was. The pin MUST keep its position and its other keys.

setmsrv[pin.unmatched]
A run in a project whose `mise.toml` pins no Rust toolchain at the current
`rust-version` MUST fail before it changes a file, and the failure MUST name
the version.

## Files

A contributor wrote the two files, and a reviewer reads the diff of a run. A
run therefore changes the version, the comment, and the pin, and nothing
else.

setmsrv[files.layout]
A run MUST leave the rest of the root manifest and of `mise.toml` as it was:
its comments, its order, its layout, and the characters that end its lines.

setmsrv[files.unreadable]
A run that cannot read or parse the root manifest or `mise.toml` MUST fail
before it changes a file, and the failure MUST name the file.

## Lock

Mise generates `mise.lock`, and `mise install --locked` refuses a pin that
the lock does not hold. A run asks mise to lock the Rust pins again after it
wrote the new pin, so that mise writes its own format. A project that keeps
no lock has chosen not to, and a run leaves it without one.

Mise locks every Rust pin again. A pin that names an exact version keeps its
entry. A pin that names a channel or a partial version, such as `nightly` or
`1.94`, can move to the newest version that it resolves to on the machine of
the run, as it does in every lock of mise.

When mise fails, the files that the run wrote stay as they are. Git is the
undo, and the user fixes the cause, locks again, and then installs and
checks.

setmsrv[lock.update]
A run in a project that holds a `mise.lock` MUST have mise lock the Rust pins
of the project after it wrote both files.

setmsrv[lock.absent]
A run in a project that holds no `mise.lock` MUST NOT create one.

setmsrv[lock.failed]
A run whose mise does not start or does not lock MUST fail, and the failure
MUST carry the cause: the error that kept mise from starting, or what mise
reported.

## Install

The new pin names a toolchain that the machine may not hold yet. A run asks
mise to install it, because provisioning is the job of mise, and the command
only asks for it. The run asks for the toolchain at the new version and
nothing else. The check needs no tool that the harness does not already run
on, and a tool that fails to install would stop a run that has no use for it.
A hook that the project gives mise runs after the install as it always does,
and it can do more.

When the install fails, the files that the run wrote stay, and the lock
stays as mise wrote it. The user fixes the cause, installs again, and checks
the code.

setmsrv[install.run]
A run MUST have mise install the Rust toolchain at the value of `msrv` after
it wrote both files and, in a project that holds a `mise.lock`, after mise
locked the pins.

setmsrv[install.failed]
A run whose mise does not start or does not install MUST fail and MUST NOT
run the check. The failure MUST name the `mise install` that the run started
and carry the cause: the error that kept mise from starting, or what mise
reported. When mise started, the failure MUST name the check as the step to
run next.

## Check

The harness names the actions of a project, so it gives the command the
check that runs the compiler on the MSRV toolchain, as it gives the
pre-commit command the actions that guard a commit. The command names no
check of its own.

A run reports the outcome of the check as the check reported it, and then
reduces it to success or failure. A check that skipped did not compile the
code on the new version, so it cannot confirm the version.

setmsrv[check.given]
The command MUST run the check that the harness gave it, with no arguments,
after the install.

setmsrv[check.report]
A run MUST report the outcome of the check as the report of an action. A run
whose report does not reach the reader MUST fail, and the failure MUST name
the check.

setmsrv[check.failed]
A run whose check found problems, stopped, or skipped MUST fail, and the
failure MUST name the check and the new version.

setmsrv[check.passed]
A run whose check passed, or repaired what it found, MUST succeed.

[rfc 2119]: https://www.rfc-editor.org/rfc/rfc2119
[tracey]: https://tracey.bearcove.eu/
