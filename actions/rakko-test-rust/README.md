# Rakko Test Rust

`rakko-test-rust` provides `test-rust`, an action for [Rakko] that runs the
tests of a project.

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
it mounts the actions that the project uses. Add `rakko-test-rust` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use rakko_test_rust::TestRust;

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

[docs]: https://docs.rs/rakko-test-rust/latest/rakko_test_rust/struct.TestRust.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[nextest]: https://nexte.st
[rakko]: https://github.com/aonyx-ai/rakko
