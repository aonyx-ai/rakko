# Aonyx Rakko Lint Shell

`aonyx-rakko-lint-shell` provides `lint-shell`, an action for [Rakko] that
lints the shell scripts of a project.

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

[docs]: https://docs.rs/aonyx-rakko-lint-shell/latest/aonyx_rakko_lint_shell/struct.LintShell.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[shellcheck]: https://www.shellcheck.net
[shfmt]: https://github.com/mvdan/sh
