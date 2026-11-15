# `aonyx-rakko-lint-markdown`

This crate provides `lint-markdown`, a Rakko action that lints Markdown files
with markdownlint.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

[Markdownlint] does the work: it discovers the Markdown files, reads its own
configuration, and applies the rules that the project turned on. So a run of the
action agrees with an editor and with a contributor that runs markdownlint,
except on a configuration file that markdownlint cannot parse.
The action declares the version of markdownlint that it runs. It also declares
the range of Node that markdownlint runs on. It starts that version, which
[mise] installed, on the Node that the project pins. It translates what
markdownlint reported into the outcome of the run.

A run only reports, so the action takes no argument. Markdownlint can repair a
part of what it finds. It does not report the files that it rewrote, so a repair
here has no file. It applies to a project that holds Markdown files, and it
skips visibly otherwise.

Markdownlint ignores a configuration file that it cannot parse, and it says
nothing about it. A run of the action that examines Markdown files stops on such
a file instead, with the message of markdownlint. A run on another configuration
does what the project asked it not to do. The action does not change the
configuration that markdownlint applies.

Markdownlint reports as JSON, and this crate reads that JSON. The shape belongs
to a version of markdownlint, so a report that the crate cannot read stops the
run, and the run does not pass.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-lint-markdown` to
the harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_lint_markdown::LintMarkdown;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(LintMarkdown).run()
}
```

The action becomes the `lint-markdown` command of the harness.

## Asynchronous Runtime

A run starts a program and waits for it. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `LintMarkdown`][docs] on docs.rs describes the tools that
the action runs, what a run reports, and when a run stops with an error.

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

[docs]: https://docs.rs/aonyx-rakko-lint-markdown/latest/aonyx_rakko_lint_markdown/struct.LintMarkdown.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[markdownlint]: https://github.com/DavidAnson/markdownlint
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[tokio]: https://tokio.rs
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
