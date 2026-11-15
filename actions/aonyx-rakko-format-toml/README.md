# `aonyx-rakko-format-toml`

This crate provides `format-toml`, a Rakko action that formats TOML files with
taplo.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

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
it mounts the actions that the project uses. Add `aonyx-rakko-format-toml` to
the harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_format_toml::FormatToml;

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

## License

Copyright (c) 2026 Aonyx B.V.

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE][license-apache]
  or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT][license-mit]
  or <http://opensource.org/licenses/MIT>)

at your option.

[docs]: https://docs.rs/aonyx-rakko-format-toml/latest/aonyx_rakko_format_toml/struct.FormatToml.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[taplo]: https://taplo.tamasfe.dev
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
