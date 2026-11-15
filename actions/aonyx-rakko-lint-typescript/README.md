# `aonyx-rakko-lint-typescript`

This crate provides `lint-typescript`, a Rakko action that lints TypeScript
and JavaScript with oxlint.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

[Oxlint] does the work: it discovers the files, reads its own configuration, and
applies the rules that the project turned on. So a run of the action agrees with
an editor and with a contributor that runs oxlint bare. The action declares the
version of oxlint that it runs. It starts that version, which [mise] installed.
It translates what oxlint reported into the outcome of the run.

A run only reports, so the action takes no argument. The action applies to a
project that holds files which oxlint lints, and it skips visibly otherwise.

Oxlint gives each diagnostic a severity, and the configuration of the project
decides that severity for each rule. This crate reports a warning and an error
alike. A run with either fails, because a rule at either severity is one that
the project asked oxlint to look for.

One run of oxlint answers everything that the action needs. The report carries
the diagnostics and the number of files that the run examined. So the project is
discovered once, by the tool that the contributor also runs.

The report is also the only thing that answers. Oxlint ends with success when
every rule that a file broke is one that the project weighs as a warning. It
ends without success for a project that holds no file to lint. So a check of the
status of the process alone passes a project with problems and fails a project
with none. A report that this crate cannot read stops the run, and the run does
not pass.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-lint-typescript`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_lint_typescript::LintTypeScript;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(LintTypeScript).run()
}
```

The action becomes the `lint-typescript` command of the harness.

## Asynchronous Runtime

A run starts a program and waits for it. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `LintTypeScript`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

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

[docs]: https://docs.rs/aonyx-rakko-lint-typescript/latest/aonyx_rakko_lint_typescript/struct.LintTypeScript.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[oxlint]: https://oxc.rs/docs/guide/usage/linter.html
[rakko]: https://rakko.aonyx.ai
[tokio]: https://tokio.rs
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
