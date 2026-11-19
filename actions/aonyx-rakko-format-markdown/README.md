# `aonyx-rakko-format-markdown`

This crate provides `format-markdown`, a Rakko action that formats Markdown
files with prettier.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

The action wraps [prettier] as a subprocess. Prettier reads its own
configuration and formats the files that the action names. So a run agrees with
an editor and with a contributor that runs prettier bare. The action declares
the version of prettier that it runs. It starts that version, which [mise]
installed, on the Node that the project pins. The action requires no range of
Node.

Prettier formats more than Markdown, and this action wraps the Markdown files
alone, so that a check names the language that needs attention. The actions for
the other languages sit in crates of their own.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-format-markdown`
to the harness of the project. Then mount the action in the `main` of the
harness:

```rust
use aonyx_rakko_format_markdown::FormatMarkdown;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(FormatMarkdown).run()
}
```

The action becomes the `format-markdown` command of the harness.

## Documentation

The [documentation of `FormatMarkdown`][docs] on docs.rs describes the tools
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

[docs]: https://docs.rs/aonyx-rakko-format-markdown/latest/aonyx_rakko_format_markdown/struct.FormatMarkdown.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[prettier]: https://prettier.io
[rakko]: https://rakko.aonyx.ai
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
