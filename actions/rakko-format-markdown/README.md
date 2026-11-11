# Rakko Format Markdown

`rakko-format-markdown` provides `format-markdown`, an action for [Rakko] that
formats the Markdown files of a project.

The action wraps [prettier] as a subprocess. Prettier reads its own
configuration and formats the files that the action names. So a run agrees with
an editor and with a contributor that runs prettier bare. The action declares
the version of prettier that it runs. It also declares the range of Node that
prettier runs on. It starts that version, which [mise] installed, on the Node
that the project pins.

Prettier formats more than Markdown, and this action wraps the Markdown files
alone, so that a check names the language that needs attention. The actions for
the other languages sit in crates of their own.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `rakko-format-markdown` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use rakko_format_markdown::FormatMarkdown;

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

[docs]: https://docs.rs/rakko-format-markdown/latest/rakko_format_markdown/struct.FormatMarkdown.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[prettier]: https://prettier.io
[rakko]: https://github.com/aonyx-ai/rakko
