# `aonyx-rakko-check-mise-lock`

This crate provides `check-mise-lock`, a Rakko action that compares the mise
lockfile with the tools that a project pins.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

A lockfile of [mise] records the version of each tool that a project pins. For a
tool that mise downloads, it also records the address and checksum of the file
for each platform. An install in locked mode stops when the lockfile has no
entry for the platform of the machine. It examines only that one platform, so a
missing entry shows only when a new machine, a new platform, or a new
contributor installs the tool.

Mise has no operation that only checks, so the action runs `mise lock` and
compares the lockfiles of the project before and after the run. A lockfile that
`mise lock` leaves as it is passes. Each tool, version, and platform that
`mise lock` adds, changes, or removes is a finding.

A run reports by default, and it locks a disposable copy of the project, so the
checkout of a contributor stays as it was. With `fix`, the run locks the project
itself, and each difference is a repair. A project that configures no tool to
lock skips visibly.

`mise lock` targets the platforms that the setting `lockfile_platforms` of the
project names, and the platform of the machine that runs it as well. In the
default mode of mise, it also removes the entries of every other platform. A
project therefore names every platform on which something locks it: its jobs,
its contributors, and a bot that moves its pins, such as Renovate. Otherwise the
entries that one machine writes are entries that the next run on another machine
removes. The lockfile also depends on the version of mise, and when a
contributor gets another answer than CI, CI decides.

`mise lock` asks the network for the downloads of each platform. When a download
fails, mise ends with an error, and the run stops with the error message of
mise. Some failures only make mise warn and go on. Examples are a failed query
for the tags of a release, after which mise guesses the tag. In the mode
`merge`, a failed download does the same. The run then stops with each warning,
because otherwise the comparison misses a gap in the entry of that tool. Only a
few kinds of warning stop nothing. They say nothing about what mise locked, such
as the notice of an older format of the lockfile. A failure of the network is
temporary, and a second run answers the question. Other causes last, such as a
release without the attestations that mise checks. The message of mise in the
error tells which of the two it is. The run makes mise show its warnings, also
when the environment, the project, or the global configuration of a contributor
asks mise to be quiet.

The action compares the lockfiles, and not the dependency graphs that mise keeps
beside them for tools such as those of npm.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-check-mise-lock`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_check_mise_lock::CheckMiseLock;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckMiseLock).run()
}
```

The action becomes the `check-mise-lock` command of the harness.

## Asynchronous Runtime

A run starts mise and git and waits for them. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `CheckMiseLock`][docs] on docs.rs describes the tools that
the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/aonyx-rakko-check-mise-lock/latest/aonyx_rakko_check_mise_lock/struct.CheckMiseLock.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[tokio]: https://tokio.rs
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
