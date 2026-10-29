# Shared Build Directory for the Harness

Research for [rakko#457][issue-457] on the map [rakko#442][map-442], done on
2026-10-29. The Cargo book and its changelog were read on the `master` branch
of cargo. The cargo source was read at the tag [`rust-1.99.0`][cargo-tag] and
at the master commit [`06413ec`][cargo-master]. The mise documentation and
source were read at [v2026.10.21][mise-tag]. The experiments ran on macOS with
cargo 1.92.0, cargo 1.99.1 (the current stable release), and the nightly of
2026-10-25 (cargo 1.101.0-nightly), which has the new build-dir layout of
cargo 1.100.

The question: every worktree builds its harness from the start, so every new
session pays a cold build. The ticket measured that with one shared build
directory, set through `CARGO_BUILD_BUILD_DIR`, a second copy of the harness
of kawauso built in 0.7 s instead of 10.8 s. Can the documentation recommend
one build directory that all checkouts share, and what must the
recommendation say?

## Answer in Brief

- No. The documentation cannot recommend one shared build directory. Cargo
  gives the harness package the same cache entry in every checkout, and it
  decides freshness by file times. So a checkout can run a harness that cargo
  built from the sources of another checkout, and cargo prints nothing about
  it. Every experiment showed this, on cargo 1.92, 1.99.1, and the nightly.
  Cargo [#17312][cargo-17312] reports the same for worktrees. The Cargo team
  closed it as a duplicate of [#12516][cargo-12516], where it says that it
  has "not endorsed" this workflow.
- A value in the `env` of the mise task makes it worse. The harness inherits
  the variable, and so does every cargo command that its actions start. The
  crates of the project then share cache entries between worktrees too, and a
  test or lint run can report the result of another worktree.
- The shared directory has no cleanup of its own. `cargo clean` in any
  checkout deletes the whole directory, and the automatic garbage collection
  of cargo does not look at build directories. Two builds into the directory
  at the same time wait for each other.
- If the documentation mentions the option at all, the narrowest form is a
  `--config` argument on the `cargo run` line of the task, not a variable.
  That form keeps the setting away from the actions. The harness itself keeps
  the risk of the first point.
- The ticket expected that the worktrees of this repository overwrite each
  other in a shared directory. They do not. The harness of this repository
  depends on crates outside its own workspace by path, and cargo hashes such
  a path as an absolute path. Each checkout gets its own entries for those
  crates and for everything that depends on them, which is why 34 crates
  were rebuilt. The entries sit side by side, and the directory grows with
  each checkout.
- The Cargo team works on a per-user cache for registry and git sources
  ([#5931][cargo-5931]), which is the part that is safe to share. It has not
  shipped.

## What Build-Dir Is

- `build.build-dir`, or `CARGO_BUILD_BUILD_DIR`, holds the intermediate
  artifacts. The final artifacts, such as the binary of a bin crate and the
  output of `cargo doc`, stay in the target-dir
  ([build cache][book-build-cache]). The default is the value of
  `build.target-dir` ([config][book-config-build-dir]). So each checkout keeps
  its own binary in its own `target`, as the ticket measured.
- Cargo 1.91 stabilized the setting ([changelog][cargo-changelog],
  [PR 15833][cargo-15833]). Cargo 1.88 ignored `CARGO_BUILD_BUILD_DIR`
  without a warning in a local test, and built into `target`.
- There is no flag of its own. `--config build.build-dir=...` works, as for
  any key of the configuration.
- The value takes three template variables ([source][src-templates]):
  `{workspace-root}` is the directory of the root manifest,
  `{cargo-cache-home}` is `CARGO_HOME`, and `{workspace-path-hash}` is a hash
  of the canonical path of the root manifest, split into two directories. The
  templates also work in the variable. Each checkout has its own path, so
  `{workspace-path-hash}` gives each checkout a directory of its own, and
  nothing is shared (experiment F).
- The layout inside the build-dir is internal to cargo. Cargo 1.100 changes
  it ([#15010][cargo-15010]), and the Cargo team keeps the right to change
  the hash.
- The open issue [#16147][cargo-16147] proposes to move the default to
  `{cargo-cache-home}/build/{workspace-path-hash}`. That is one directory per
  workspace, not one for all.

## Which Cache Entries Two Checkouts Share

Cargo gives each unit a hash, which goes into the file names in the build-dir
([`compute_metadata`][src-metadata]). The hash covers the package id, the
features, the profile, the target, the version of rustc, and the hashes of
the dependencies. `RUSTFLAGS` go into the file name too. The package id
covers the source of the package ([`stable_hash`][src-stable-hash]):

| Source                       | What goes into the hash                        | Same hash in every checkout |
| ---------------------------- | ---------------------------------------------- | --------------------------- |
| A registry                   | The URL of the registry                        | Yes                         |
| A git repository             | The canonical URL of the repository            | Yes                         |
| A path inside the workspace  | The path relative to the root of the workspace | Yes                         |
| A path outside the workspace | The absolute path                              | No                          |

The harness is its own workspace in `tools/rakko`. So:

- A harness that takes the Rakko crates from git, like the one of kawauso,
  shares every dependency between checkouts. The harness package shares its
  entry too, because it lies inside its own workspace. That is the speedup
  that the ticket measured, and it is also the problem of the next section.
- The harness of this repository takes the Rakko crates by path from outside
  its workspace. Those crates, and the harness package that depends on them,
  get one entry per checkout (experiment A). Only the registry dependencies
  are shared. Cargo [#7645][cargo-7645] tracks this hashing of paths outside
  the workspace.

## A Checkout Can Run the Harness of Another

For a path package, cargo compares file times: if no source file is newer
than the dep-info file of the last build, the unit is fresh
([fingerprint][src-fingerprint]). With a shared entry, the last build can
come from another checkout. Cargo then links the binary of that build into
the target-dir of this checkout, and `cargo run` starts it.

The experiment used two copies, `a` and `b`, of a harness without
dependencies, at two paths, with one shared build-dir. The two copies differ
only in the line that they print. All files were dated before the first
build, as in two worktrees that already exist.

| Step | What was done          | What the harness printed |
| ---- | ---------------------- | ------------------------ |
| 1    | Build in `b`           | harness of copy b        |
| 2    | Build in `a`, no edit  | harness of copy b        |
| 3    | Edit `a`, build in `a` | harness of copy a        |
| 4    | Build in `b`, no edit  | harness of copy a        |

Cargo 1.92.0, 1.99.1, and the nightly of 2026-10-25 gave the same four
lines. The new layout of the nightly does not change the result.

When it happens in practice:

- Two worktrees have different harness sources, but the same dependency
  graph. For example, one branch changes only `main.rs` of the harness. A
  change of the Rakko revision or of `Cargo.lock` changes the graph, and the
  harness then gets a new entry.
- A new worktree writes all its files when it is created, so its first build
  is correct. The problem starts in an older worktree after a build in a
  newer one.
- Two projects can collide in the same way. The harness package of this
  repository is `harness` 0.1.0. A harness of another project with the same
  name, version, and dependency graph gets the same hash.

In [#12516][cargo-12516], the Cargo team says that it has "not endorsed" a
shared directory for several workspaces, and it gives cache entries that
conflict, lock contention, and cache poisoning as the problems. Checksums in
place of file times (`-Zchecksum-freshness`, unstable) would detect the
change. The same comment says that each worktree would then overwrite the
entries of the local packages of every other worktree.

## The Task Environment Reaches Every Cargo Command

The `env` of a mise task applies to the task, and its child processes
inherit it ([task env][mise-task-env]). The harness is such a child, and the
harness started from the task saw the variable (experiment A). Rakko removes
only some variables when it starts a tool. So `test-rust`, `lint-rust`,
`build-internal-docs`, and the other cargo actions also build into the shared
directory. The crates of the project lie inside its workspace, so they get
the same entries in every worktree, with the same freshness problem. A run
can then replay the cached result of another worktree.

A `--config` argument on the `cargo run` line applies to that cargo process
only. Cargo gives the program the variables of the package and the `[env]`
table of its configuration ([`fill_env`][src-fill-env]), not the other keys
of the configuration. In experiment B, and in the mise task of experiment G,
the harness saw no `CARGO_BUILD_BUILD_DIR`.

A value in the top-level `[env]` of `mise.toml` reaches further still. Mise
sets it in every shell where mise is active, and mise-action exports it to
the later steps of a CI job ([environments][mise-env]).

## Locks

- Cargo 1.96 and later take `.cargo-build-lock` in the profile directory of
  the build-dir as an exclusive lock, with the message "build directory"
  ([layout][src-layout]). Cargo 1.92 takes `.cargo-lock` exclusively in the
  profile directories of the target-dir and the build-dir.
- So two builds into one shared directory, with the same profile and target,
  run one after the other. In experiment D, the second build printed
  "Blocking waiting for file lock on build directory" and waited about 4 s.
- `cargo run` releases the lock before it starts the program. In experiment
  C, copy `b` built in 0.16 s while the harness of copy `a` still ran. The
  source agrees: the lock lives in the build runner, which ends before
  [`cargo run`][src-cargo-run] starts the program. That holds on Windows as
  well, where cargo waits for the program instead of replacing its own
  process. Only builds wait, not runs of the harness.
- Finer locks per unit exist behind `-Zfine-grain-locking`
  ([#4282][cargo-4282]). On 2026-09-24, the maintainer wrote in
  [#17508][cargo-17508] that "the fine grain locking design is currently not
  sound", and stopped that work until the new layout and a cache across
  workspaces are further along.
- `cargo clean` takes no lock ([source][src-clean]). It can delete the shared
  directory while another checkout builds into it.

## Cleanup

- `cargo clean` without `-p` deletes the target-dir and the build-dir
  ([source][src-clean]). In experiment E, `cargo clean` in copy `a` deleted
  the shared directory, and the target-dir of copy `b` stayed.
- `--release` and `--profile` limit it to one profile, which is still
  shared. `-p <package>` deletes the entries of that package for every
  checkout. No option limits it to one checkout.
- The automatic garbage collection, stable since cargo 1.88 with
  `cache.auto-clean-frequency`, covers only `CARGO_HOME`. The Cargo book says:
  "Support for tracking build artifacts is not yet implemented"
  ([global caches][book-global-caches]). The issue [#13136][cargo-13136] and
  the pull request [#16929][cargo-16929] for it are open.
- So the directory grows without a limit. Each new dependency version,
  feature set, flag set, and stable toolchain adds entries. The 441 MB of the
  ticket is one harness at one revision.

## Mise Templates for a Cache Path

Mise offers `xdg_cache_home` in every template ([templates][mise-templates]).
An `XDG_CACHE_HOME` that the user sets wins on every platform. Without it,
the defaults are ([source][mise-src-env], [directories][mise-directories]):

| Variable         | macOS              | Linux            | Windows                                  |
| ---------------- | ------------------ | ---------------- | ---------------------------------------- |
| `xdg_cache_home` | `~/Library/Caches` | `~/.cache`       | `%TEMP%`                                 |
| `xdg_data_home`  | `~/.local/share`   | `~/.local/share` | `%LOCALAPPDATA%`, else `~\AppData\Local` |

Facts for a value in `mise.toml`:

- On Windows, `xdg_cache_home` is the temporary directory, which the Storage
  Sense of Windows can clean.
- `{{ [xdg_cache_home, "rakko", "build"] | join_path }}` gives the native
  separator of the platform.
- Tera leaves cargo's own `{workspace-path-hash}` alone, because it reacts
  only to `{{`, `{%`, and `{#`.
- Mise does not expand `~` in an env value.
- A plain value in the task `env` overrides a value that the developer has
  set. The form `{ default = "..." }`, new in mise 2026.6.10
  ([PR 10441][mise-10441]), keeps the developer's value.

The narrowest form, tested on macOS only (experiment G):

```toml
[tasks.rakko]
raw_args = true
run = '''cargo run --quiet --config "build.build-dir='{{ [xdg_cache_home, "rakko", "build"] | join_path }}'" --manifest-path "{{ config_root }}/tools/rakko/Cargo.toml" --'''
```

The single quotes make the path a literal string of TOML, so the
backslashes of a Windows path need no escapes. The double quotes keep it one
argument in `sh` and in `cmd`. Windows was not tested.

## CI Runners

- A runner that GitHub hosts starts as a new virtual machine for each job
  ([GitHub][gh-hosted]). mise-action caches only the data directory of mise
  ([mise-action][mise-action]). A shared build-dir there starts empty and
  holds one checkout, so it gains nothing and loses nothing.
- A persistent runner that runs one job at a time keeps the directory
  between jobs. There, the jobs of one repository already reuse one
  checkout path, and with it the default `target`. A shared directory adds
  only the sharing between repositories, with the collision of the harness
  packages that the section above describes. It grows without a limit, a
  `cargo clean` in any job deletes it, and with one job at a time no lock
  waits.

## Where Cargo Is Going

- [#5931][cargo-5931] plans a per-user cache of compiled artifacts. Its
  first version covers only registry and git sources, which are immutable.
  That is the safe part of what the ticket measured.
- [#16147][cargo-16147] proposes the per-workspace default above. It moves
  the build out of the worktree but shares nothing.
- The Cargo book suggests the third-party tool sccache to share builds
  between workspaces ([build cache][book-build-cache]). Its limits were not
  checked here.

## Experiments

The scripts ran in a scratch directory with `CARGO_INCREMENTAL=0`.

| Experiment | What it did                                                             | Result                                                                 |
| ---------- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Stale      | Two copies of a harness without dependencies, one shared build-dir      | Each copy ran the harness of the other, on 1.92.0, 1.99.1, and nightly |
| A          | A harness with a path dependency outside its workspace, variable in env | Two entries for each crate; the harness saw `CARGO_BUILD_BUILD_DIR`    |
| B          | The same build with `--config build.build-dir=...`                      | The shared directory was used; the harness saw no variable             |
| C          | Build copy `b` while the harness of copy `a` sleeps for 8 s             | `b` built in 0.16 s, on 1.92.0 and 1.99.1                              |
| D          | Build copy `b` while a build script of copy `a` sleeps for 5 s          | "Blocking waiting for file lock on build directory", about 4 s         |
| E          | `cargo clean` in copy `a`                                               | The shared directory was gone; the target-dir of `b` stayed            |
| F          | `CARGO_BUILD_BUILD_DIR` with `{workspace-path-hash}`                    | One directory per copy                                                 |
| G          | The `--config` form of the task above, run through `mise run`           | The build went to the cache path; the harness saw no variable          |
| Old cargo  | Cargo 1.88.0 with `CARGO_BUILD_BUILD_DIR`                               | No warning; the build went to `target`                                 |

[book-build-cache]: https://doc.rust-lang.org/cargo/reference/build-cache.html
[book-config-build-dir]: https://doc.rust-lang.org/cargo/reference/config.html#buildbuild-dir
[book-global-caches]: https://doc.rust-lang.org/cargo/reference/config.html#cache
[cargo-12516]: https://github.com/rust-lang/cargo/issues/12516
[cargo-13136]: https://github.com/rust-lang/cargo/issues/13136
[cargo-15010]: https://github.com/rust-lang/cargo/issues/15010
[cargo-15833]: https://github.com/rust-lang/cargo/pull/15833
[cargo-16147]: https://github.com/rust-lang/cargo/issues/16147
[cargo-16929]: https://github.com/rust-lang/cargo/pull/16929
[cargo-17312]: https://github.com/rust-lang/cargo/issues/17312
[cargo-17508]: https://github.com/rust-lang/cargo/issues/17508
[cargo-4282]: https://github.com/rust-lang/cargo/issues/4282
[cargo-5931]: https://github.com/rust-lang/cargo/issues/5931
[cargo-7645]: https://github.com/rust-lang/cargo/issues/7645
[cargo-changelog]: https://doc.rust-lang.org/nightly/cargo/CHANGELOG.html
[cargo-master]: https://github.com/rust-lang/cargo/tree/06413ecf8cd3ec5b10b93ac2b85f6fad34f8597c
[cargo-tag]: https://github.com/rust-lang/cargo/tree/rust-1.99.0
[gh-hosted]: https://docs.github.com/en/actions/how-tos/manage-runners/github-hosted-runners/use-github-hosted-runners
[issue-457]: https://github.com/aonyx-ai/rakko/issues/457
[map-442]: https://github.com/aonyx-ai/rakko/issues/442
[mise-10441]: https://github.com/jdx/mise/pull/10441
[mise-action]: https://github.com/jdx/mise-action#readme
[mise-directories]: https://mise.jdx.dev/directories.html
[mise-env]: https://mise.jdx.dev/environments/
[mise-src-env]: https://github.com/jdx/mise/blob/v2026.10.21/crates/mise-util/src/env.rs#L73-L100
[mise-tag]: https://github.com/jdx/mise/tree/v2026.10.21
[mise-task-env]: https://mise.jdx.dev/tasks/task-configuration.html#env
[mise-templates]: https://mise.jdx.dev/templates.html
[src-cargo-run]: https://github.com/rust-lang/cargo/blob/rust-1.99.0/src/ops/cargo_run.rs
[src-clean]: https://github.com/rust-lang/cargo/blob/06413ecf8cd3ec5b10b93ac2b85f6fad34f8597c/src/ops/cargo_clean.rs#L126-L140
[src-fill-env]: https://github.com/rust-lang/cargo/blob/rust-1.99.0/src/compiler/compilation.rs
[src-fingerprint]: https://github.com/rust-lang/cargo/blob/06413ecf8cd3ec5b10b93ac2b85f6fad34f8597c/src/compiler/fingerprint/mod.rs#L66-L95
[src-layout]: https://github.com/rust-lang/cargo/blob/06413ecf8cd3ec5b10b93ac2b85f6fad34f8597c/src/compiler/layout.rs#L265-L309
[src-metadata]: https://github.com/rust-lang/cargo/blob/06413ecf8cd3ec5b10b93ac2b85f6fad34f8597c/src/compiler/build_runner/compilation_files.rs#L758-L907
[src-stable-hash]: https://github.com/rust-lang/cargo/blob/06413ecf8cd3ec5b10b93ac2b85f6fad34f8597c/src/workspace/source_id.rs#L567-L591
[src-templates]: https://github.com/rust-lang/cargo/blob/06413ecf8cd3ec5b10b93ac2b85f6fad34f8597c/src/context/mod.rs#L781-L810
