# Aonyx Rakko Check TypeScript

`aonyx-rakko-check-typescript` provides `check-typescript`, an action for
[Rakko] that type checks the TypeScript of a project.

[Tsc] does the work. It reads the `tsconfig.json` of the project and collects
the files that the configuration selects. It reports every rule of the language
that the code breaks. So a run of the action agrees with an editor and with a
contributor that runs tsc bare. TypeScript and Node are toolchains of the
project, so the action declares the range of versions of each that it accepts.
It starts the tsc that [mise] installed for the project, at the version that the
project pinned, on the Node that the project pins. It translates what tsc
reported into the outcome of the run.

A run only reports, so the action takes no argument. It emits nothing either.
Tsc builds a project as well as it checks one, and a check that left output
behind writes files that nobody asked for.

Type checking is not linting. The compiler asks whether the program holds
together, and a linter asks whether the code does something that the project
turned a rule against. A design that rests on the compiler to reject a wrong
composition has no guarantee at all while no check runs the compiler. That check
is what this action is for.

The diagnostics of tsc are the answer. The status of the process decides only
whether a run without any is trusted. Tsc reports a diagnostic for everything
that it finds, including a configuration that it refuses and a project that it
finds no configuration of. So a run that reported nothing and ended without
success stopped for a reason of its own. It stops the action instead of passing.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-check-typescript`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_check_typescript::CheckTypeScript;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckTypeScript).run()
}
```

The action becomes the `check-typescript` command of the harness.

## Asynchronous Runtime

A run starts a program and waits for it. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `CheckTypeScript`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-check-typescript/latest/aonyx_rakko_check_typescript/struct.CheckTypeScript.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[tokio]: https://tokio.rs
[tsc]: https://www.typescriptlang.org/docs/handbook/compiler-options.html
