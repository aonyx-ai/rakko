# `aonyx-rakko-lint-yaml`

This crate provides `lint-yaml`, a Rakko action that lints YAML files with
yamllint.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

[Yamllint] does the work: it discovers the YAML files and reads its own
configuration. It applies the rules that the project turned on. So a run of the
action agrees with an editor and with a contributor that runs yamllint bare. The
action declares the version of yamllint that it runs. It starts that version,
which [mise] installed. It translates what yamllint reported into the outcome of
the run.

A run only reports, so the action takes no argument, and yamllint repairs
nothing that it finds. The action applies to a project that holds YAML files,
and it skips visibly otherwise.

Yamllint gives each problem a level, and the configuration of the project
decides that level for each rule. This crate reports a warning and an error
alike, because both are problems that the project asked yamllint to look for. A
run with either fails.

A run starts yamllint twice. The first run asks which files yamllint examines.
It needs this because a yamllint that collected no file writes the same empty
report as one that examined the whole project and found nothing. The second run
lints those files. A line of the report that this crate cannot read stops the
run, and the run does not pass.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-lint-yaml` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_lint_yaml::LintYaml;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(LintYaml).run()
}
```

The action becomes the `lint-yaml` command of the harness.

## Asynchronous Runtime

A run starts a program and waits for it. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `LintYaml`][docs] on docs.rs describes the tools that the
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

[docs]: https://docs.rs/aonyx-rakko-lint-yaml/latest/aonyx_rakko_lint_yaml/struct.LintYaml.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[tokio]: https://tokio.rs
[yamllint]: https://github.com/adrienverge/yamllint
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
