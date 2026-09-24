# ADR-015: Nightly Toolchain

## Status

Accepted

## Context

Rakko needs a nightly toolchain next to the pinned default. Rustfmt reads
unstable options from `.rustfmt.toml`, cargo-udeps needs an unstable flag to
record the crates that a target loaded, and the minimal-versions check needs
an unstable flag of cargo. [ADR-003] kept this toolchain on the floating
nightly channel. A pin that names a date would be reproducible, but nothing
would move it, so it would quietly age.

The channel did not stay where [ADR-003] put it. Mise came to pin all three
toolchains, and the pin of the nightly named the bare channel. The lockfile of
mise was to record the day that the channel resolved to, so that every
machine used the same nightly. It did not. Mise links each Rust toolchain that
it installs to the directory of rustup, and it takes that link for a
toolchain that a user linked by hand. For such a toolchain it skips the
lockfile, and the channel resolves to the newest nightly on the machine, or
during an install to the newest that rustup offers. From at least 2026-09-10,
every run on the hosted runners formatted with the nightly of its day, the
self-hosted runners used whichever nightly their restored cache held, and
laptops agreed only because each of them held a single nightly. Nothing
reported the drift.

A drift of the nightly shows up as a formatting check that fails on one
machine and passes on another, in a pull request that changed nothing about
the formatting. On a persistent self-hosted runner that restores no cache,
each runner keeps the nightly that it happens to hold.

Renovate can move a pin that none of its managers knows. A custom manager
finds the pin in the files that hold it, and a custom datasource names the
newest release. This answers the reason that [ADR-003] gave against a dated
pin.

## Decision

The nightly toolchain is pinned by its date, and Renovate moves the date.

1. **The pin names a day.** `mise.toml` pins a dated nightly, such as
   `nightly-2026-08-11`, and never the bare channel. A dated pin resolves to
   the same toolchain on every machine without help from the lockfile.

2. **Renovate moves the pin on a schedule.** A pull request moves the date
   forward, and its CI shows whether the newer nightly changes the formatting
   or breaks a check. The pull request changes the lockfile together with the
   pin, so that an install that requires the lockfile accepts it.

3. **Projects pin the same way.** A project that mounts the Rust actions
   meets the same failure with the bare channel, so the guidance for the Rust
   bundle names a dated pin.

This supersedes the floating nightly channel in point 3 of [ADR-003]. The
rest of [ADR-003] stands, including the choice to reach the additional
toolchains through rustup. This ADR does not decide how often the date moves,
which nightly Renovate offers, or where its configuration lives. These are
settings, and they can change without a new decision.

## Alternatives

We considered these alternatives and rejected them for the reasons below.

### The Floating Channel

This is the choice of [ADR-003]. It needs no automation, and the nightly is
never old. But two machines format with two nightlies whenever they last
installed on different days, and nothing in the repository records which
nightly a run used. The failure of the lockfile only made this visible.

### The Channel With the Lockfile

The bare channel can stay in `mise.toml` while the lockfile records the day.
This is what the repository tried. It depends on mise honoring the lockfile
for Rust, and mise does not. If a release of mise fixes this, the question can
come back, because the dated pin then only repeats what the lockfile holds.

### A Dated Pin That Moves by Hand

The dated pin can stay without the automation. It is reproducible, but it
ages until someone remembers to move it, which is the reason that [ADR-003]
turned it down. The automation is what changed.

## Consequences

- Every machine and every job formats with the same nightly, and the pin in
  the repository says which one.
- A new nightly arrives as a pull request with its own CI, and not as a
  failed check in a pull request that has nothing to do with it.
- The nightly lags behind the channel by up to one interval of the schedule.
  The nightly serves unstable options and checks, and no code depends on a
  feature that only the newest nightly has, so a lag of days costs nothing.
- A nightly that breaks a check blocks only its own pull request, and someone
  has to look at it. Until then, the pin stays on the last nightly that
  passed.
- Every machine collects one toolchain for each move of the pin. A persistent
  self-hosted runner or a laptop keeps every nightly that it installed until
  someone removes it.

[adr-003]: 003-mise.md
