# `aonyx-rakko-lint-shell`

This crate provides `lint-shell`, a Rakko action that lints shell scripts with
ShellCheck.

[Rakko] turns project maintenance into versioned Rust crates. An action is
a crate that does one maintenance task. A project runs its chosen actions
through a harness, a small Rust program.

Two tools do the work. [Shfmt] finds the shell scripts, by their extension and
by their shebang, and it leaves out each script that an `.editorconfig` file
marks with `ignore = true`. So the action lints the scripts that `format-shell`
formats. [ShellCheck] examines those scripts. It reads the `.shellcheckrc`
files of the project, so a run agrees with a contributor that runs ShellCheck
bare on the same scripts. The action declares the versions of both tools. It
starts those versions, which [mise] installed, and translates what ShellCheck
reported into the outcome of the run.

A run only reports. Each comment of ShellCheck is a finding over the range of
the script that ShellCheck gave, with its level and the code of its check. A
comment at the style level fails the run like one at the error level. A project
that wants a check to stay quiet turns it off with its code in `.shellcheckrc`.
A project that holds no shell script is skipped, because shfmt found nothing
to lint.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add `aonyx-rakko-lint-shell` to the
harness of the project. Then mount the action in the `main` of the harness:

```rust
use aonyx_rakko_lint_shell::LintShell;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(LintShell).run()
}
```

The action becomes the `lint-shell` command of the harness.

## Documentation

The [documentation of `LintShell`][docs] on docs.rs describes the tools that the
action runs, what a run reports, and when a run stops with an error.

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

[docs]: https://docs.rs/aonyx-rakko-lint-shell/latest/aonyx_rakko_lint_shell/struct.LintShell.html
[guide]: https://rakko.aonyx.ai/how-to/build-an-action/
[mise]: https://mise.jdx.dev
[rakko]: https://rakko.aonyx.ai
[shellcheck]: https://www.shellcheck.net
[shfmt]: https://github.com/mvdan/sh
[license-apache]: LICENSE-APACHE
[license-mit]: LICENSE-MIT
