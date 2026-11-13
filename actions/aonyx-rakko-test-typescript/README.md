# Aonyx Rakko Test TypeScript

`aonyx-rakko-test-typescript` provides `test-typescript`, an action for [Rakko]
that runs the tests of a Node project.

The [test runner] that Node carries does the work. It searches the project for
the files that hold tests. It runs each of them in a process of its own and
reports what every test did. Node is a toolchain of the project, so the action
declares the range of versions that it accepts. It starts the node that [mise]
installed for the project, at the version that the project pinned. It translates
what the runner reported into the outcome of the run.

The runner needs nothing from a project. A test file imports `node:test`, and
the project adds no dependency, no configuration file, and no script. So a
project that runs no tests yet can mount the action with no setup. Node also
removes the types of a TypeScript file before it runs the file. So a project
that writes its tests in TypeScript needs no build before the run.

Node decides which files hold tests, with the patterns that it carries. So a run
of the action covers the tests of a bare run and no others. A run only reports,
so the action takes no argument.

The report is the answer, and the status of the process decides only whether a
run without a failure is trusted. The runner reports every test that failed. So
a run that reported none and ended without success stopped for a reason of its
own. It stops the action instead of passing.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-test-typescript`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_test_typescript::TestTypeScript;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(TestTypeScript).run()
}
```

The action becomes the `test-typescript` command of the harness.

## Asynchronous Runtime

A run starts a program and waits for it. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `TestTypeScript`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-test-typescript/latest/aonyx_rakko_test_typescript/struct.TestTypeScript.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[test runner]: https://nodejs.org/api/test.html
[tokio]: https://tokio.rs
