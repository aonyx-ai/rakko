# `aonyx-rakko-lint-rust`

This crate provides `lint-rust`, a Rakko action that lints Rust code with
clippy.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

[Clippy] does the work: cargo reads the manifests of the project, selects the
lints that the project configured, and examines every target with every feature.
A run of the action agrees with an editor and with a contributor that runs
clippy bare. The action starts the cargo that [mise] installed for the project,
at the version that the project pinned. It translates what cargo reported into
the outcome of the run.

A run only reports, and it takes no argument. Every diagnostic that clippy
raises becomes a finding, whether the project warns about the lint or denies it,
so a run with a warning fails. The action applies to a project that holds a
manifest of cargo, and it skips visibly otherwise.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-lint-rust` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_lint_rust::LintRust;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(LintRust).run()
}
```

The action becomes the `lint-rust` command of the harness.

## Documentation

The [documentation of `LintRust`][docs] on docs.rs describes the tools that the
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

[clippy]: https://doc.rust-lang.org/clippy/
[docs]: https://docs.rs/aonyx-rakko-lint-rust/latest/aonyx_rakko_lint_rust/struct.LintRust.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
