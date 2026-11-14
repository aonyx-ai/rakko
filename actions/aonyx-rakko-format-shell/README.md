# Aonyx Rakko Format Shell

`aonyx-rakko-format-shell` provides `format-shell`, an action for [Rakko] that
formats the shell scripts of a project.

[Shfmt] does the work. It finds the shell scripts, by their extension and by
their shebang. It reads its options from the `.editorconfig` files of the
project, and it formats each script that such a file does not mark with
`ignore = true`. So a run of the action agrees with an editor and with a
contributor that runs shfmt bare. The action declares the version of shfmt that
it runs. It starts that version, which [mise] installed, and translates what
shfmt reported into the outcome of the run.

A run reports by default and rewrites with the `fix` argument. A script that
shfmt cannot parse is a finding at the position of the error, also in a run
with `fix`, because a rewrite cannot repair it. A project that holds no shell
script is skipped, because shfmt found nothing to format.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-format-shell` to
the harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_format_shell::FormatShell;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(FormatShell).run()
}
```

The action becomes the `format-shell` command of the harness.

## Documentation

The [documentation of `FormatShell`][docs] on docs.rs describes the tools that
the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-format-shell/latest/aonyx_rakko_format_shell/struct.FormatShell.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[shfmt]: https://github.com/mvdan/sh
