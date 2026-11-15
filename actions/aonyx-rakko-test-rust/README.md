# `aonyx-rakko-test-rust`

This crate provides `test-rust`, a Rakko action that runs Rust tests with
nextest.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

[Nextest] does the work: cargo builds every target of every package with every
feature, and nextest reads its own configuration and runs the tests. So a run of
the action agrees with a contributor that runs nextest bare. The action declares
the version of nextest that it runs. It starts that version, which [mise]
installed, with the cargo of the stable toolchain that the project pins. It
translates what nextest and cargo reported into the outcome of the run.

A run only reports, and it takes no argument. A test that failed becomes a
finding at the position where it panicked. A build that does not finish becomes
findings from the diagnostics of the compiler. The action skips visibly in a
project whose cargo discovers no workspace.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-test-rust` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_test_rust::TestRust;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(TestRust).run()
}
```

The action becomes the `test-rust` command of the harness.

## Documentation

The [documentation of `TestRust`][docs] on docs.rs describes the tools that the
action runs, what a run reports, and when a run stops with an error.

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

[docs]: https://docs.rs/aonyx-rakko-test-rust/latest/aonyx_rakko_test_rust/struct.TestRust.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[nextest]: https://nexte.st
[rakko]: https://rakko.aonyx.ai
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
