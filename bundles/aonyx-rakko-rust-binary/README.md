# Aonyx Rakko Rust Binary

`aonyx-rakko-rust-binary` is the bundle that a Rust project mounts when it ships
a binary: the formatter, the linter, the test runner, the checks that guard what
the project depends on, and the build of its internal documentation. A project
subscribes to the whole set with one dependency and one line in its harness,
and a release of the bundle reaches the project as a pull request like any
other dependency.

The bundle contains `aonyx-rakko-rust`, the core of the Rust family, and adds
nothing to it today. It exists so that a binary project names what it is, and so
that a check which only a binary runs reaches every such project as a release of
this bundle, without an edit to the harness of each one.

A project mounts this bundle or `aonyx-rakko-rust-library`, never both, because
both carry the core and a harness stops when two actions have one name. A
workspace that publishes a library and ships a binary mounts the library bundle,
which is the superset. Neither of the two contains the baseline, so a Rust
project mounts two names: this bundle beside `aonyx-rakko-baseline`.

## Usage

Add `aonyx-rakko-rust-binary` to the harness of the project, and let the `main`
of the harness mount what the bundle exports:

```rust
fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(aonyx_rakko_rust_binary::bundle()).run()
}
```

Each action of the bundle becomes one command of the harness, exactly as if
the harness had mounted that action itself. A harness that wants all of the
bundle but one action filters the list, because the list is an ordinary value.

## Actions

The bundle exports the eight actions of `aonyx-rakko-rust` and no other.

## Tools

The bundle asks for no tool beyond the ones that `aonyx-rakko-rust` names,
because it exports the actions of the core and nothing else.

## Documentation

The [documentation of the `bundle` function][docs] on docs.rs describes the list
of actions that the function returns.

## Support

This bundle was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-rust-binary/latest/aonyx_rakko_rust_binary/fn.bundle.html
[guide]: https://github.com/aonyx-ai/rakko#support
