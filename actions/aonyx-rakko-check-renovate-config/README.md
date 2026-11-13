# Aonyx Rakko Check Renovate Config

`aonyx-rakko-check-renovate-config` provides `check-renovate-config`, an action
for [Rakko] that checks the Renovate configuration of a project.

`renovate-config-validator`, the validator that [Renovate] ships, does the work.
It finds the configurations of the project, reads each of them, and reports
every option that Renovate refuses. A run of the action agrees with a
contributor that runs `renovate-config-validator --strict`, and then
`renovate-config-validator --strict --no-global default.json`. The action
declares the version of Renovate that it runs. It also declares the range of
Node that the validator runs on. It starts the validator of that version, which
[mise] installed, on the Node that the project pins. It translates what the
validator reported into the outcome of the run.

A configuration that Renovate refuses on the default branch stops Renovate from
opening pull requests until someone fixes it. The action asks whether Renovate
accepts the configuration before it merges.

A run asks for strict validation. So an option that needs a migration fails the
run like an error and a warning. Each error and each warning becomes a finding
at the file that the validator named. A configuration that needs a migration
becomes one finding that names the options the migration changes.

A preset is a configuration that other configurations extend. Renovate reads a
preset such as `github>owner/repository` from the `default.json` in the root of
that repository. The validator does not find this file by itself, so the action
also validates a `default.json` in the root of the project. It does this for
every such file, also for one that belongs to another tool. The validator checks
the preset as the configuration of a repository. These checks are stricter than
the check of a preset in Renovate. So a preset that only fits inside a package
rule fails the run, such as one that sets `matchPackageNames` at the top.

The validator also reads the global configuration that a self-hosted Renovate
starts with. That is `config.js` in the root of the project, unless the
environment names another file. A `config.js` that is not a configuration of
Renovate therefore produces findings as well.

A run asks the validator for JavaScript regular expressions instead of RE2, so
every machine validates with the same engine. RE2 refuses some patterns that
JavaScript accepts. Such a pattern passes the action.

A run only reports, so the action takes no argument. The action applies to a
project that holds a Renovate configuration or a preset, and it skips visibly
otherwise.

## Usage

A harness is the small binary crate of a project that `mise run rakko` runs, and
it mounts the actions that the project uses. Add
`aonyx-rakko-check-renovate-config` to the harness of the project. Then mount
the action in the `main` of the harness:

```rust
use aonyx_rakko_check_renovate_config::CheckRenovateConfig;

fn main() -> std::process::ExitCode {
    rakko_cli::builder().mount(CheckRenovateConfig).run()
}
```

The action becomes the `check-renovate-config` command of the harness.

## Asynchronous Runtime

A run starts a program and waits for it. A [Tokio] runtime drives that. The
command line that `rakko-cli` builds starts one, so a harness needs no runtime
of its own. A caller that runs the action outside of that command line starts a
runtime first, because the run panics without one.

## Documentation

The [documentation of `CheckRenovateConfig`][docs] on docs.rs describes the
tools that the action runs, what a run reports, and when a run stops with an
error.

## Support

This action was built specifically for Aonyx's workflows. It might not fit
your needs, in which case we strongly encourage you to create your own action.
See the [guide] for instructions.

[docs]: https://docs.rs/aonyx-rakko-check-renovate-config/latest/aonyx_rakko_check_renovate_config/struct.CheckRenovateConfig.html
[guide]: https://github.com/aonyx-ai/rakko#support
[mise]: https://mise.jdx.dev
[rakko]: https://github.com/aonyx-ai/rakko
[renovate]: https://docs.renovatebot.com
[tokio]: https://tokio.rs
