# Rakko Format TOML

`rakko-format-toml` provides `format-toml`, an action for [Rakko] that formats
the TOML files of a project.

[Taplo] does the work. It discovers the TOML files, reads its own configuration,
and formats what that configuration selects. So a run of the action agrees with
an editor and with a contributor that runs taplo bare. The action declares the
version of taplo that it runs. It starts that version, which [mise] installed,
and translates what taplo reported into the outcome of the run.

A run reports by default and rewrites with the `fix` argument. A project that
holds no TOML file passes, because the run found nothing to format. The action
does not skip such a project, because taplo does not always report that it found
no file.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `rakko-format-toml` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use rakko_format_toml::FormatToml;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(FormatToml).run()
}
```

The action becomes the `format-toml` command of the harness.

## Documentation

The [documentation of `FormatToml`][docs] on docs.rs describes the tools that
the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/rakko-format-toml/latest/rakko_format_toml/struct.FormatToml.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[taplo]: https://taplo.tamasfe.dev
