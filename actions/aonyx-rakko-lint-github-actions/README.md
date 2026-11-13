# Aonyx Rakko Lint GitHub Actions

`aonyx-rakko-lint-github-actions` provides `lint-github-actions`, an action for
[Rakko] that audits the GitHub Actions workflows of a project.

[Zizmor] does the work: it collects the workflows, reads its own configuration,
and applies the audits that it knows. So a run of the action agrees with an
editor and with a contributor that runs zizmor bare. The action declares the
version of zizmor that it runs. It starts that version, which [mise] installed.
The action translates what zizmor reported into the outcome of the run.

Zizmor publishes no build for Windows on ARM. So the declaration leaves that
platform out, and a run of the action skips there.

Zizmor looks for patterns that turn a workflow into a way into the repository.
Examples are templates that expand attacker-controlled text into shells, jobs
that carry more permissions than needed, and actions with no digest pins. Two
questions belong to other actions. Those actions wrap yamllint and prettier. One
question is whether the same file is well-formed YAML. The other is whether it
obeys the layout rules of the project.

A run only reports, so the action takes no argument. The action applies to a
project that holds GitHub Actions workflows, and it skips visibly otherwise.

A finding of zizmor names one or more places of a workflow: where the finding
is, and what a reader needs to read it. Zizmor draws them together in one block
of source. An outcome of Rakko has no place for one block, so each place becomes
a finding of its own at the range that zizmor named. The audit and the severity
in each message say which of them belong together.

Zizmor gives each finding a severity. The crate reports every severity, because
each one is a pattern that zizmor was asked to look for. A run with any of them
fails.

A run asks zizmor for the pedantic persona, which reports code smells of a
workflow as well. Zizmor takes a persona on its command line alone, and its
configuration file has no key for one. A project cannot ask for a persona of its
own. A run also asks zizmor to stop at a file that it collected and cannot read.
So no workflow leaves the audit through a warning.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add
`aonyx-rakko-lint-github-actions` to the harness of the project. Then mount the
action in the `main` of the harness:

```rust
use aonyx_rakko_lint_github_actions::LintGitHubActions;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(LintGitHubActions).run()
}
```

The action becomes the `lint-github-actions` command of the harness.

## Asynchronous Runtime

A run starts a program and waits for it. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `LintGitHubActions`][docs] on docs.rs describes the tools
that the action runs, what a run reports, and when a run stops with an error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-lint-github-actions/latest/aonyx_rakko_lint_github_actions/struct.LintGitHubActions.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[tokio]: https://tokio.rs
[zizmor]: https://docs.zizmor.sh
