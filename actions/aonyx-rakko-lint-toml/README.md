# Aonyx Rakko Lint TOML

`aonyx-rakko-lint-toml` provides `lint-toml`, an action for [Rakko] that
validates the TOML files of a project.

[Taplo] does the work: it discovers the TOML files and reads its own
configuration. It validates what that configuration selects, so a run of the
action agrees with an editor and with a contributor that runs taplo bare. The
action declares the version of taplo that it runs. It starts that version, which
[mise] installed, and translates what taplo reported into the outcome of the
run.

A run only reports. Taplo repairs nothing that a validation finds, so the action
takes no argument. A project that holds no TOML file passes, because the run
found nothing to report. The action does not skip such a project, because taplo
does not always report that it found no file.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-lint-toml` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_lint_toml::LintToml;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(LintToml).run()
}
```

The action becomes the `lint-toml` command of the harness.

## Documentation

The [documentation of `LintToml`][docs] on docs.rs describes the tools that the
action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-lint-toml/latest/aonyx_rakko_lint_toml/struct.LintToml.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[taplo]: https://taplo.tamasfe.dev
