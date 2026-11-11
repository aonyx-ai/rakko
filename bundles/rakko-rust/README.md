# Rakko Rust

`rakko-rust` is the bundle that any Rust project mounts: the formatter, the
linter, the test runner, the checks that guard what the project depends on,
and the build of its internal documentation. A project subscribes to the whole
set with one dependency and one line in its harness, and a release of the
bundle reaches the project as a pull request like any other dependency.

The bundle is the core that the Rust library bundle and the Rust binary bundle
contain, and a project mounts it through one of them. It contains no other
bundle, and it does not contain the baseline, so a Rust project mounts two
names: this family beside `rakko-baseline`.

## Usage

Add `rakko-rust` to the harness of the project, and let the `main` of the
harness mount what the bundle exports:

```rust
fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(rakko_rust::bundle()).run()
}
```

Each action of the bundle becomes one command of the harness, exactly as if
the harness had mounted that action itself. A harness that wants all of the
bundle but one action filters the list, because the list is an ordinary value.

## Actions

The bundle exports seven actions:

- `build-internal-docs` builds the documentation of the workspace with
  [rustdoc], including what the crates keep to themselves.
- `check-dependencies` examines the dependencies of the project with
  [cargo-deny], for advisories, licenses, and bans.
- `check-latest-deps` resolves every dependency to its newest version and runs
  the tests, so that a version floor stays a promise the project keeps.
- `check-unused-deps` finds the dependencies that no target loads, with
  [cargo-udeps].
- `format-rust` formats the Rust files with [rustfmt].
- `lint-rust` examines the Rust files with [clippy].
- `test-rust` runs the tests with [nextest].

## Tools

Rakko installs nothing, and an action whose tool [mise] does not report stops
instead of passing quietly. An action runs two kinds of tool.

The Rust toolchains are decisions of the project, so the project pins them in
its `mise.toml`, and each action accepts a range of them. Each action runs on
the first pin in its range, in the order of the `rust` entry. For example, with
`rust = ["1.99.1", "nightly-2026-11-02", "1.88.0"]`, the actions that run on a
stable toolchain run on 1.99.1. With `1.88.0` first, they run on 1.88.0.

- `rust`, with the `clippy` and `rustfmt` components, because cargo drives
  every action of the bundle, and clippy and rustfmt are what two of them run.
  The actions accept a stable toolchain from 1.85 on, pinned by a version such
  as `1.99.1`. A pin of the `stable` channel has no version to compare, so a
  run passes over it and takes the first pin with a version in the range. That
  pin can be the pin of the oldest supported version: with
  `rust = ["stable", "1.88.0"]`, the actions run on 1.88.0. A run stops when
  no pin with a version is in the range, and its error says to add such a pin.
- a second `rust` on the `nightly` channel, with the `rustfmt` component,
  because `format-rust` reads the unstable options of `.rustfmt.toml` and
  `check-unused-deps` needs the unstable option that records which crates a
  target loaded. Both refuse to do their work on a stable toolchain. Pin it by
  its date, such as `nightly-2026-08-11`, and not by the bare channel. Mise
  ignores `mise.lock` for Rust once it has installed any toolchain, so a bare
  `nightly` resolves to the newest nightly that the machine holds. The actions
  accept a nightly from 2025-02-20 on. A bare `nightly` counts for nothing,
  also on a machine where mise resolves it to a date.

The other tools belong to the actions. Each of these actions runs the version
that its crate pins in a `mise.rakko.toml`, and not the version that the
project pins. So the pin of the project must be that same version, until Rakko
writes the pin into the project itself. When that version is not installed, a
run stops, and its error gives the tool and the version:

- `cargo:cargo-deny`, for `check-dependencies`, which pins it itself.
- `cargo:cargo-nextest`, for `test-rust` and for the tests that
  `check-latest-deps` runs. The crate `rakko-nextest` pins it.
- `cargo:cargo-udeps`, for `check-unused-deps`, which pins it itself.

Each tool reads its own configuration, so a run of an action agrees with the
editor of a contributor and with a run of the tool alone.

## Documentation

The [documentation of the `bundle` function][docs] on docs.rs describes the list
of actions that the function returns.

## Support

This bundle was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[cargo-deny]: https://embarkstudios.github.io/cargo-deny/
[cargo-udeps]: https://github.com/est31/cargo-udeps
[clippy]: https://doc.rust-lang.org/clippy/
[docs]: https://docs.rs/rakko-rust/latest/rakko_rust/fn.bundle.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[nextest]: https://nexte.st
[rustdoc]: https://doc.rust-lang.org/rustdoc/
[rustfmt]: https://rust-lang.github.io/rustfmt/
