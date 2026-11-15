# `aonyx-rakko-rust-binary`

This crate bundles the maintenance actions that Aonyx runs for Rust projects
that ship binaries.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

A project subscribes to the whole set with one dependency and one line in its
harness, and a release of the bundle reaches the project as a pull request
like any other dependency.

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

The bundle exports the nine actions of `aonyx-rakko-rust` and no other.

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

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/aonyx-rakko-rust-binary/latest/aonyx_rakko_rust_binary/fn.bundle.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[rakko]: https://rakko.aonyx.ai
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
