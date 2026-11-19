# `aonyx-rakko-format-typescript`

This crate provides `format-typescript`, a Rakko action that formats
TypeScript and JavaScript with oxfmt.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

[Oxfmt] does the work. The action gives oxfmt the files of TypeScript and of
JavaScript, and oxfmt reads its own configuration and formats what its ignore
files leave. So a run of the action agrees with an editor and with a contributor
that runs oxfmt bare. The action declares the version of oxfmt that it runs. It
starts that version, which [mise] installed, on the Node that the project pins.
The action requires no range of Node.

Oxfmt formats more languages than the action asks it about. The action passes
oxfmt only the extensions of TypeScript and of JavaScript. Oxfmt then leaves the
Markdown, the JSON, and the YAML of a project to the actions that format them.

A run reports by default and rewrites with the `fix` argument. A run whose
oxfmt found no file of the language skips visibly, and says so.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-format-typescript`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_format_typescript::FormatTypeScript;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(FormatTypeScript).run()
}
```

The action becomes the `format-typescript` command of the harness.

## Documentation

The [documentation of `FormatTypeScript`][docs] on docs.rs describes the tools
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

[docs]: https://docs.rs/aonyx-rakko-format-typescript/latest/aonyx_rakko_format_typescript/struct.FormatTypeScript.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[oxfmt]: https://oxc.rs/docs/guide/usage/formatter.html
[rakko]: https://rakko.aonyx.ai
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
