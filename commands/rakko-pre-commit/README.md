# Rakko Pre-Commit

`rakko-pre-commit` provides the `pre-commit` command, which runs the actions
that guard a commit. A harness gives the command one list of these actions, in
any order. The harness selects the actions, and the command line of the
harness orders and drives them. The harness therefore contains no loop over
actions, no code that reports them, and no split into the actions that write
and the actions that read.

The command is a group, and the actions in its list are its members. The
group is a name and this list, and nothing else. The command line of the
harness takes the flags of the command from the arguments of the members, and
it runs the members. So the command defines no flag of its own.

## Usage

Add `rakko-pre-commit` to the harness of the project. Then let the `main` of
the harness give the command its list. Mount the command as a group beside the
actions:

```rust
use rakko::{ErasedAction, Group};
use rakko_format_toml::FormatToml;
use rakko_lint_toml::LintToml;
use rakko_pre_commit::PreCommit;

let pre_commit = PreCommit::new([
    Box::new(LintToml) as Box<dyn ErasedAction>,
    Box::new(FormatToml),
]);

rakko_cli::builder()
    .mount(rakko_baseline::bundle())
    .mount_groups([Box::new(pre_commit) as Box<dyn Group>])
    .run();
```

A list is any iterator of erased actions. The list holds each action only
once. A list with two actions of the same name stops the harness when it
mounts the command. The list is independent of the mount, so a harness that
wants a command for one of these actions mounts that action as well.

Each action declares whether a run reads or writes the project, and the
command line orders the run from these declarations. A declaration can depend on
the arguments of the run: a formatter writes only when the run asks it to
repair. An action that declares nothing writes. The list decides the order of
two actions that write. This order matters when they touch the same files. Two
formatters can do this, and so can a generator whose output derives from
formatted files.

## Run

The command line binds every action to the values of the run before the run
starts. Each action receives the values of the arguments that it declares, and
no other value. An action whose values do not convert stops the run before any
action runs, and the error gives the action and the argument. The run then asks
each action how it uses the project. An action that declares a kind of access
that the command line does not know also stops the run before any action runs.
The error gives the action and tells the user to update Rakko, because only a
newer release can know the kind. Otherwise, the run drives the actions that
write, one after another and in the order of the list. When the last of them
ended, the run starts all the actions that read at the same time. Each action
reports when it ends, so a fast check does not wait for a slow one. The report
has the text that a run of that action alone shows. An action that found
problems does not end the run, so one run reports every problem of the tree.

```console
mise run rakko -- pre-commit --fix
```

The command has a `--fix` flag when an action in the list declares a `fix`
argument, as each formatter does. The flag reaches every action that declares
`fix`, and no other action. Its help shows one line for each of these actions,
and each line starts with the name of the action. A formatter writes only with
the flag, so a run without the flag drives the formatters among the actions
that read. Such a run reports what a commit has to repair. An action that
writes whatever its arguments, such as an action that declares nothing, still
writes in a run without the flag.

When every action ended, the run shows one summary. The summary gives each
action that found problems or stopped, in the order of the list, and then
counts them:

```text
lint-yaml: found problems
test-rust: stopped
pre-commit: 1 of 11 actions found problems, 1 stopped
```

The command fails when an action found problems or stopped, so the commit waits.
It exits as a run of one action does. The code is 2 when an action stopped,
otherwise 1 when an action found problems, and otherwise 0. So CI can tell a
tree with problems from a check that could not answer. A run in which the
actions that write repaired everything that they found exits with 0, because the
repair is what the flag asked for. An action that skipped does not fail the run
either. The commit still waits after a repair, because [pre-commit] stops a
commit whose files a hook changed.

With `--json`, the standard output carries one document: the summary. It holds
the name `pre-commit`, the number of actions in each state, and the report of
each action in the order of the list. Each report has the shape that a run of
that action alone writes. The reports of the actions go to the standard error
as text. With `--quiet`, the reports of the actions do not appear, and the
summary still gives the actions that found problems or stopped.

## Hook

The hook that Git runs before a commit calls the command with `--fix`. The
command has this flag only while an action in the list declares `fix`. A
harness whose list holds no such action calls the command without the flag.
With [pre-commit], a local hook starts the task of the harness:

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
