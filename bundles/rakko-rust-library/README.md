# Rakko Rust Library

`rakko-rust-library` is the bundle that a Rust project mounts when it publishes
a library: everything that any Rust project runs, and the checks that guard the
promises a library makes to whoever depends on it. A project subscribes to the
whole set with one dependency and one line in its harness, and a release of the
bundle reaches the project as a pull request like any other dependency.

The bundle contains `rakko-rust`, the core of the Rust family, and adds three
checks to it. A project mounts this bundle or `rakko-rust-binary`, never both,
because both carry the core and a harness stops when two actions have one name.
A workspace that publishes a library and ships a binary mounts this one, which
is the superset. Neither of the two contains the baseline, so a Rust project
mounts two names: this bundle beside `rakko-baseline`.

## Usage

Add `rakko-rust-library` to the harness of the project, and let the `main` of
the harness mount what the bundle exports:

```rust
rakko_cli::builder().mount(rakko_rust_library::bundle()).run();
```

Each action of the bundle becomes one command of the harness, exactly as if
the harness had mounted that action itself. A harness that wants all of the
bundle but one action filters the list, because the list is an ordinary value.

## Actions

The bundle exports ten actions: the seven of `rakko-rust` and three of its own.
The three guard what a published library promises, and a binary promises none
of it:

- `check-minimal-deps` resolves every dependency to the floor that the
  manifests state and runs the tests, so that a floor stays a promise the
  library keeps.
- `check-msrv` compiles the project with the toolchain that the `rust-version`
  of the manifests names, because only that compiler answers for the promise.
- `test-rust-docs` runs the examples in the documentation with [cargo], which
  the test runner of `test-rust` leaves out.

## Tools

Each action runs the tool that [mise] installed for the project, at the
version that the project pinned. Rakko installs nothing, and an action whose
tool mise does not report stops instead of passing quietly. The core states
what any Rust project pins, and the three checks here add one entry to that
list:

- a `rust` whose version repeats the `rust-version` of the manifests, because
  `check-msrv` runs the compiler on the toolchain that the promise names. A
  pin that stops agreeing with the manifests fails the check instead of moving
  it.

The other two need nothing beyond the core: `check-minimal-deps` resolves on
the nightly `rust` and runs the tests with `cargo:cargo-nextest`, and
`test-rust-docs` builds the examples with cargo.

[cargo]: https://doc.rust-lang.org/cargo/
[mise]: https://mise.jdx.dev
