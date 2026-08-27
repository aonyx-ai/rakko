# ADR-003: Mise as the Provisioning Layer

## Status

Accepted

## Context

Rakko depends on external tools: the Rust toolchains, the formatters and
linters, and later every tool that an action wraps. Something must install
these tools at pinned versions on every developer machine and in CI. Today
[Flox][flox] fills this role. `.flox/env/manifest.toml` lists the tools, and
Flox installs them from Nixpkgs.

Flox conflicts with two goals from the [vision][vision]. First, Flox has no
Renovate integration. No pull request arrives when a tool releases a new
version, so our pins quietly fall behind. Second, Flox builds on Nix, and Nix
does not run on Windows. The vision promises that a contributor on Windows
clones a repository and everything works.

The Rust toolchain adds a requirement of its own. The project needs three
toolchains side by side: the pinned default for builds, Clippy, and tests; a
nightly toolchain for `cargo fmt --unstable-features`, `cargo udeps`, and the
minimal-versions check; and the toolchain for the minimal supported Rust
version (MSRV) from `rust-version` in `Cargo.toml`. Today rustup multiplexes
these, with the default pinned in `rust-toolchain.toml` — a second pinning
mechanism that Renovate does not update either. A replacement must supply all
three toolchains, and it should collapse the pins into one mechanism.

The vision already names [mise][mise] as the bottom layer of the three-layer
architecture. This ADR records that decision and its reasons before the
migration starts.

## Decision

We replace Flox with mise as the provisioning layer. A `mise.toml` at the
repository root pins every external tool, and mise installs the pins on
developer machines and in CI.

1. **One file pins every tool.** `mise.toml` is the single inventory of
   external tools. Renovate has a first-class mise manager, so every pin gets
   update pull requests like a Cargo dependency.

2. **Mise pins the Rust toolchain.** The default toolchain, with the `clippy`
   and `rustfmt` components, is a pin in `mise.toml` like any other tool.
   Mise delegates to rustup: it installs rustup when it is missing, installs
   the toolchain through it, and selects it by setting `RUSTUP_TOOLCHAIN`. We
   remove `rust-toolchain.toml`, so the toolchain has one pin, and Renovate
   updates it.

3. **Additional toolchains go through rustup.** Because mise manages Rust
   through rustup, rustup is always present. Rustup gives a per-invocation
   selection — `rustup run <toolchain>` or `cargo +<toolchain>` —
   [precedence over][rustup-overrides] the `RUSTUP_TOOLCHAIN` variable. A
   recipe therefore runs nightly or MSRV commands next to the mise-pinned
   default, exactly as it does today. The nightly channel stays floating,
   because a dated nightly pin would be reproducible but has no update
   automation, so it would quietly age. The MSRV check keeps reading
   `rust-version` from `Cargo.toml`.

4. **Non-Rust tools come from mise backends.** Mise installs tools through
   backends such as npm, pipx, ubi, and cargo, next to its own registry. One
   mechanism covers the Rust toolchains and every other tool, and Renovate
   updates these pins too.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### Keeping Flox

Flox provides a reproducible environment from Nixpkgs, including system
libraries and C compilers, and it has carried the project so far. But Flox
has no Renovate integration, does not run on Windows, and leaves the Rust
toolchain pin in a second mechanism. These gaps are the reason this decision
exists, and none of them has a fix on Flox's horizon.

### Asdf

Asdf established the one-file-per-project tool manager, and Renovate has a
manager for its `.tool-versions` file. But asdf is built on bash and does not
run on Windows. Every tool needs a plugin, and plugin quality varies. Mise
began as a rewrite of asdf, reads the same plugin ecosystem, and adds the
backends and the Windows support that asdf lacks. Asdf offers nothing that
mise does not.

### Nix Directly

We can drop Flox and maintain our own Nix environment. This keeps the
reproducible environment and removes one layer. But Nix still does not run on
Windows, and Renovate updates flake inputs, not the versions of individual
packages inside Nixpkgs. Direct Nix also demands more expertise than Flox,
not less, and expertise is the scarce resource.

### Manual Installation

A README can list the required tools, and each contributor installs them with
their package manager. This needs no extra tool, but it pins nothing.
Versions drift between machines and CI, and no bot opens update pull
requests. Onboarding becomes a checklist that each machine follows by hand.

## Consequences

- One file pins every external tool, and Renovate opens a pull request when
  one of them releases a new version. The invisible-updates problem from the
  vision is closed for this repository.
- Updates to the Rust toolchain become ordinary Renovate pull requests, and
  CI on the pull request shows whether the new compiler breaks the workspace.
- Provisioning stops being the Windows blocker. The remaining blockers — the
  shell recipes in the justfile — are what Rakko itself exists to replace.
- We lose the hermetic environment. Flox supplied system libraries and C
  compilers, such as libiconv and clang, from Nixpkgs. With mise, the host
  platform provides them, for example through the Xcode Command Line Tools on
  macOS. A tool without a mise backend must also come from the host.
- Without `rust-toolchain.toml`, a shell without mise sees rustup's default
  toolchain, not the project pin. Editors, agents, and scripts must enter the
  environment through mise activation, shims, or `mise exec`.
- The migration touches every place that assumes Flox: the `shell` setting in
  the justfile, the CI workflows and their cache keys, and the activation
  hook that builds tracey from source, which needs a new home such as a mise
  task.
- Mise has a stable command-line interface to locate pinned tools, such as
  `mise which`. This is the interface through which Rakko actions will later
  resolve the tools they wrap. That design gets its own ADR when the first
  such action arrives.

[flox]: https://flox.dev
[mise]: https://mise.jdx.dev
[rustup-overrides]: https://rust-lang.github.io/rustup/overrides.html
[vision]: ../VISION.md
