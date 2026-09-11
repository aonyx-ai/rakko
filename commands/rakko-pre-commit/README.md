# Rakko Pre-Commit

`rakko-pre-commit` provides the `pre-commit` command, which runs the actions
that guard a commit. A harness gives the command two lists: the actions that
write to the tree, and the actions that only read it. The harness names the
actions, and the command drives them. The harness therefore contains no loop
over actions and no code that reports them.

## Usage

Add `rakko-pre-commit` to the harness of the project. Then let the `main` of
the harness give the command its two lists, and mount the command beside the
actions:

```rust
use rakko_action::ErasedAction;
use rakko_cli::ErasedCommand;
use rakko_format_toml::FormatToml;
use rakko_lint_toml::LintToml;
use rakko_pre_commit::PreCommit;

let pre_commit = PreCommit::new(
    [Box::new(FormatToml) as Box<dyn ErasedAction>],
    [Box::new(LintToml) as Box<dyn ErasedAction>],
);

rakko_cli::builder()
    .mount(rakko_baseline::bundle())
    .mount_commands([Box::new(pre_commit) as Box<dyn ErasedCommand>])
    .run();
```

Each list holds its actions in the order in which they run. A list is any
iterator of erased actions. The lists are independent of the mount, so a
harness that wants a command for one of these actions mounts that action as
well.

An action that writes is not only a formatter. A generator whose output
derives from formatted files writes as well, and it goes in the first list,
after the formatters.

## Run

A run drives the actions that write, one after another and in the order of
their list. Then it drives the actions that read in the same way. Each action
reports when it ends, as a run of that action alone reports. An action that
found problems does not end the run, so one run reports every problem of the
tree.

```console
mise run rakko -- pre-commit --fix
```

The `--fix` flag lets the actions that write repair what they find, and only
these actions receive it. A run without the flag reports what a commit has to
repair, and it changes nothing.

The command fails when an action found problems or stopped, so the commit
waits. A run in which the actions that write repaired everything that they
found succeeds, because the repair is what the flag asked for. The commit
still waits in that case, because [pre-commit] stops a commit whose files a
hook changed.

## Hook

The hook that Git runs before a commit calls the command with `--fix`. With
[pre-commit], a local hook starts the task of the harness:

```yaml
repos:
  - repo: local
    hooks:
      - id: rakko
        name: rakko
        entry: mise
        args: ["run", "rakko", "--", "pre-commit", "--fix"]
        language: system
        pass_filenames: false
```

Git runs the hook outside the environment of mise, so the hook starts mise,
and mise runs the harness.

[pre-commit]: https://pre-commit.com
