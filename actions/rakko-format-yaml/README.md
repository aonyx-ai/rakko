# Rakko Format YAML

`rakko-format-yaml` provides `format-yaml`, an action for [Rakko] that formats
the YAML files of a project.

The action wraps [prettier] as a subprocess: prettier reads its own
configuration and formats the files that the action names. So a run agrees with
an editor and with a contributor that runs prettier bare. The action declares
the version of prettier that it runs. It also declares the range of Node that
prettier runs on. It starts that version, which [mise] installed, on the Node
that the project pins.

Prettier formats more than YAML, and this action wraps the YAML files alone, so
that a check names the language that needs attention. The actions for the other
languages sit in crates of their own.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `rakko-format-yaml` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use rakko_format_yaml::FormatYaml;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(FormatYaml).run()
}
```

The action becomes the `format-yaml` command of the harness.

## Documentation

The [documentation of `FormatYaml`][docs] on docs.rs describes the tools that
the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/rakko-format-yaml/latest/rakko_format_yaml/struct.FormatYaml.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[prettier]: https://prettier.io
[rakko]: https://github.com/aonyx-ai/rakko
