# Rakko Set MSRV

`rakko-set-msrv` provides the `set-msrv` command, which sets the minimum
supported Rust version (MSRV) of a project. Three places state that version,
and all three must agree: the `rust-version` of the root `Cargo.toml`, the
Rust pin in `mise.toml` that check-msrv runs the compiler on, and the entry of
that pin in `mise.lock`. The command writes all three in one run.

## Usage

Add `rakko-set-msrv` to the harness of the project, and mount the command
beside the actions:

```rust
use rakko_cli::ErasedCommand;
use rakko_set_msrv::SetMsrv;

rakko_cli::builder()
    .mount(rakko_baseline::bundle())
    .mount_commands([Box::new(SetMsrv) as Box<dyn ErasedCommand>])
    .run();
```

## Run

A run takes the new version and the reason for it:

```console
mise run rakko -- set-msrv --msrv 1.89.0 --reason "bon 3.11 requires Rust 1.89"
```

The run changes three things:

- It sets the `rust-version` of `[workspace.package]` in the root manifest,
  or of `[package]` when the root is a single package. The reason replaces the
  comment directly above the version.
- It sets the Rust pin in `mise.toml` that names the old version. The pin
  keeps its position, because the first pin is the default toolchain. When a
  separate MSRV pin shares its version with the default, the run moves the
  last pin that names it, and the default stays. A default that is the only
  pin at that version moves with the MSRV.
- It asks mise to lock the Rust pins again, when the project keeps a
  `mise.lock`. Mise locks every Rust pin, so a pin that names a channel or a
  partial version, such as `nightly` or `1.94`, can move as well.

Everything else in the files stays as it was. In a project whose pins name
exact versions, the diff shows the version, the comment, the pin, and the
lock entry. After the run, install the new toolchain and check the code on
it:

```console
mise install
mise run rakko -- check-msrv
```

The version must have three parts, such as `1.89.0`. Cargo reads `1.89` as
`1.89.0`, but mise resolves a pin of `1.89` to the newest `1.89.x`, so the two
would disagree about the toolchain. The reason is one line that must not be
blank, because it replaces the comment above the version. A manifest that
declares the version in an inline table has no place for that comment, so the
run refuses it.

The run refuses a project in which no Rust pin names the current
`rust-version`, and it changes nothing then. Such a project has not adopted
check-msrv yet, and the command does not add a pin, because a pin in the
wrong position changes the default toolchain.
