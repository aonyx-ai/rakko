# Run all recipes inside the mise environment, so that every recipe reaches
# the tool versions that `mise.toml` pins.
set shell := ["mise", "exec", "--", "sh", "-cu"]

[private]
default:
    @just --list

# Build the internal documentation of the Rust code
#
# The recipe runs the harness instead of cargo, for the reason that
# `format-toml` gives. The action documents every workspace of the repository,
# so the recipe names no package of its own, and it documents every feature,
# where the bare cargo that it replaces documented the default ones. The
# recipe that this one replaces was named `build-rustdoc` and claimed to force
# the rustdoc lints to run, but nothing denied them, so a broken link between
# two items left it green. The action reads the report of the build instead,
# and a diagnostic of rustdoc now fails the recipe.
build-internal-docs:
    mise run rakko -- build-internal-docs

# Check that Rakko builds with the latest dependencies
#
# The recipe runs the harness instead of cargo, for the reason that
# `format-toml` gives. The action resolves and tests in a copy of the project,
# so the recipe guards nothing and takes no argument: it leaves the lockfile
# and the working tree of a contributor as they are, where the recipe that it
# replaces rewrote `Cargo.lock` and refused to run on a tree with changes in
# it. The action covers every workspace of the repository, so the harness is
# checked as well, and it reports every diagnostic of the build as a finding,
# where the bare cargo that it replaces denied deprecations alone.
check-latest-deps:
    mise run rakko -- check-latest-deps

# Check that dependencies have compatible open-source licenses and trusted sources
#
# The recipe runs the harness instead of cargo-deny, for the reason that
# `format-toml` gives. The action checks every workspace of the repository,
# where the bare cargo-deny that it replaces reached only the workspace of the
# crates, so the harness is covered now and carries a `deny.toml` of its own.
# A warning still leaves the recipe passing, and the run counts the warnings
# that it read.
check-dependencies:
    mise run rakko -- check-dependencies

# Check that Rakko builds with the minimal dependencies
#
# The recipe runs the harness instead of rustup and cargo, for the reason that
# `format-toml` gives. The action resolves and tests in a copy of the project,
# so the recipe guards nothing and takes no argument: it leaves the lockfile
# and the working tree of a contributor as they are, where the recipe that it
# replaces rewrote `Cargo.lock` and refused to run on a tree with changes in
# it. The action resolves the floors on the nightly toolchain that `mise.toml`
# pins, so the recipe installs no toolchain. It covers every workspace of the
# repository, so the harness is checked as well, and it reports every
# diagnostic of the build as a finding, where the bare cargo that it replaces
# denied deprecations alone.
check-minimal-deps:
    mise run rakko -- check-minimal-deps

# Check that the specs and the requirement references in the code are valid
#
# The recipe runs the harness instead of tracey, for the reason that
# `format-toml` gives. The action stops the daemon that would answer from a
# stale scan, and it builds the comparison that a pull request needs in a copy
# of the repository instead of moving the HEAD of the checkout.
check-specs:
    mise run rakko -- check-specs

# Check that Rakko builds with the MSRV
#
# The recipe runs the harness instead of rustup and cargo, for the reason that
# `format-toml` gives. The action reads the `rust-version` of every workspace
# and checks each workspace on the toolchain that it declares, so the recipe
# reads no version of its own and installs no toolchain. `mise.toml` pins the
# toolchain of the MSRV next to the default one, because the harness installs
# nothing. The action reports every diagnostic of the older compiler, so a
# warning now fails the recipe, where the bare cargo that it replaces failed
# on an error and on a deprecation alone.
check-msrv:
    mise run rakko -- check-msrv

# Check that all dependencies in Cargo.toml are used
#
# The recipe runs the harness instead of rustup and cargo-udeps, for the
# reason that `format-toml` gives. The action examines every workspace of the
# repository on the nightly toolchain that `mise.toml` pins, so the recipe
# installs no toolchain. It examines every target of every package with every
# feature, where the bare cargo-udeps that it replaces examined the default
# targets with the default features, so a dependency that only a test or only
# a feature declares answers now as well.
check-unused-deps:
    mise run rakko -- check-unused-deps

# Format JSON files
#
# The recipe runs the harness instead of prettier, for the reason that
# `format-toml` gives. The action names the files that prettier examines, so
# the recipe passes no pattern of its own.
format-json fix="false":
    mise run rakko -- format-json {{ if fix == "true" { "--fix" } else { "" } }}

# Format Markdown files
#
# The recipe runs the harness instead of prettier, for the reason that
# `format-toml` gives.
format-markdown fix="false":
    mise run rakko -- format-markdown {{ if fix == "true" { "--fix" } else { "" } }}

# Format Rust files
#
# The recipe runs the harness instead of rustfmt, for the reason that
# `format-toml` gives. The action formats every workspace of the repository
# on the nightly toolchain that `mise.toml` pins, so the harness needs no
# call of its own, and the recipe installs no toolchain.
format-rust fix="false":
    mise run rakko -- format-rust {{ if fix == "true" { "--fix" } else { "" } }}

# Format TOML files
#
# The recipe runs the harness instead of taplo, so the action does the work
# that this recipe used to do itself. The harness reports findings and exit
# codes uniformly, and a fix that rewrote files exits zero.
format-toml fix="false":
    mise run rakko -- format-toml {{ if fix == "true" { "--fix" } else { "" } }}

# Format YAML files
#
# The recipe runs the harness instead of prettier, for the reason that
# `format-toml` gives.
format-yaml fix="false":
    mise run rakko -- format-yaml {{ if fix == "true" { "--fix" } else { "" } }}

# Lint GitHub Actions workflows
#
# The recipe runs the harness instead of zizmor, for the reason that
# `format-toml` gives. The action names the project to zizmor, which then
# collects the files, so the recipe passes no path of its own. It asks for
# the same pedantic persona that this recipe used to ask for, and it also
# asks zizmor to stop at a file that it collected and cannot read, where the
# bare zizmor dropped such a file with a warning and ended with success.
lint-github-actions:
    mise run rakko -- lint-github-actions

# Lint Markdown files
#
# The recipe runs the harness instead of markdownlint, for the reason that
# `format-toml` gives. The action names the project to markdownlint, which
# then walks it, so the recipe passes no pattern of its own. The pattern that
# it used to pass reached one directory deep, because `sh` does not expand
# `**` recursively, so the run now covers every Markdown file of the
# repository instead of the twelve below `adrs`.
lint-markdown:
    mise run rakko -- lint-markdown

# Lint Rust files
#
# The recipe runs the harness instead of clippy, for the reason that
# `format-toml` gives. The action lints every workspace of the repository,
# so the harness needs no call of its own.
lint-rust:
    mise run rakko -- lint-rust

# Lint TOML files
#
# The recipe runs the harness instead of taplo, for the reason that
# `format-toml` gives. It takes no argument, because taplo repairs nothing
# that a validation finds.
lint-toml:
    mise run rakko -- lint-toml

# Lint YAML files
#
# The recipe runs the harness instead of yamllint, for the reason that
# `format-toml` gives. The action names the project to yamllint, which then
# walks it, so the recipe passes no pattern of its own. A rule of the warning
# level now fails the recipe, where the bare yamllint that it replaces ended
# with success.
lint-yaml:
    mise run rakko -- lint-yaml

# Run the actions that guard a commit
#
# The recipe runs the harness instead of the private recipes that it used to
# chain, for the reason that `format-toml` gives. The harness names the
# actions that guard a commit and their order, so the recipe lists nothing of
# its own, and the flag lets the formatters rewrite the tree before the checks
# read it. Each action reports on its own, so the reports no longer
# interleave, and the run drives every action, where the chain that it
# replaces stopped at the first formatter that failed. The checks ran at the
# same time before and run one after another now, until the harness learns
# what each action reads and writes.
pre-commit:
    mise run rakko -- pre-commit --fix

# Run the tests
#
# The recipe runs the harness instead of nextest, for the reason that
# `format-toml` gives. The action tests every workspace of the repository.
test-rust:
    mise run rakko -- test-rust
