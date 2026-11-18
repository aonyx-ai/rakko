# `rakko-cli`

This crate turns the Rakko actions that a project chooses into a command line.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

A bundle is a crate that exports a list of actions. A harness selects actions
one by one or through a bundle. This crate gives each action a command with
its help text and its flags.

A harness can also mount commands that it writes itself, and groups. A group
is a named list of actions that runs as one command, such as the actions that
run before a commit. Every harness also gets the command `doctor`, which
checks that the project pins the tools that the mounted actions run.

## Usage

Add `rakko-cli` to the harness, together with the crates of the actions,
bundles, and commands that the harness mounts. Then run the command line from
the `main` of the harness. This example mounts the bundle
`aonyx-rakko-baseline`:

```rust
fn main() -> std::process::ExitCode {
    rakko_cli::builder()
        .mount(aonyx_rakko_baseline::bundle())
        .run()
}
```

The harness gives each action, bundle, command, or group to `mount` as it is.

Run `doctor --fix` before the first run, and again when an update of Rakko
changes a tool. It writes the pins of the tools that the mounted actions run.
It does not write the pin of a toolchain, such as Node or Rust. The project
pins each toolchain itself. For example, several actions of
`aonyx-rakko-baseline` run on Node, so the project pins Node. Until the pins of
the tools are current, every command stops. Only the help, the list of the
commands, and `doctor` still run. A toolchain pin that an action does not
accept stops only that action, and `doctor` reports it.

## Documentation

The [documentation of the crate][docs] on docs.rs describes the builder, the
commands, the groups, and `doctor`.

## Support

Aonyx uses Rakko internally, and we maintain and support this crate. The
section [Support] in the README of the repository lists the crates that Aonyx
supports for other authors.

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/rakko-cli
[rakko]: https://rakko.aonyx.ai
[support]: https://github.com/aonyx-ai/rakko#support
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
