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
rakko_cli::builder().mount(rakko_rust::bundle()).run();
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

Each action runs the tool that [mise] installed for the project, at the
version that the project pinned. Rakko installs nothing, and an action whose
tool mise does not report stops instead of passing quietly, so a project that
mounts this bundle pins these tools in its `mise.toml`:

- `rust`, with the `clippy` and `rustfmt` components, because cargo drives
  every action of the bundle, and clippy and rustfmt are what two of them run.
- a second `rust` on the `nightly` channel, with the `rustfmt` component,
  because `format-rust` reads the unstable options of `.rustfmt.toml` and
  `check-unused-deps` needs the unstable option that records which crates a
  target loaded. Both refuse to do their work on a stable toolchain.
- `cargo:cargo-deny`, for `check-dependencies`.
- `cargo:cargo-nextest`, for `test-rust` and for the tests that
  `check-latest-deps` runs.
- `cargo:cargo-udeps`, for `check-unused-deps`.

Each tool reads its own configuration, so a run of an action agrees with the
editor of a contributor and with a run of the tool alone.

[cargo-deny]: https://embarkstudios.github.io/cargo-deny/
[cargo-udeps]: https://github.com/est31/cargo-udeps
[clippy]: https://doc.rust-lang.org/clippy/
[mise]: https://mise.jdx.dev
[nextest]: https://nexte.st
[rustdoc]: https://doc.rust-lang.org/rustdoc/
[rustfmt]: https://rust-lang.github.io/rustfmt/
