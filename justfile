# Run all recipes inside the mise environment, so that every recipe reaches
# the tool versions that `mise.toml` pins.
set shell := ["mise", "exec", "--", "sh", "-cu"]

[private]
default:
    @just --list

[private]
pre-commit-checks: pre-commit-fix pre-commit-verify

# Every recipe that rewrites the working tree, in sequence: they overlap each
# other, and nothing may read a file while one of them is writing it. The
# formatters run before the generation so that what is generated is derived
# from formatted sources.
[private]
pre-commit-fix:
    just format-json true
    just format-markdown true
    just format-yaml true
    just format-toml true
    just format-rust true

# Every recipe that only reads, in parallel: the tree has stopped changing, so
# what each of them sees is what the commit will contain.
[private]
pre-commit-verify:
    #!/usr/bin/env -S mise exec -- bash
    set -uo pipefail

    # Each check runs as a background job, and its output streams as it
    # arrives, so lines from different checks interleave. The recipe waits for
    # every job and fails if any of them failed.
    pids=()
    for recipe in check-specs lint-github-actions lint-markdown lint-rust lint-yaml test-rust; do
        just "$recipe" &
        pids+=("$!")
    done

    status=0
    for pid in "${pids[@]}"; do
        wait "$pid" || status=1
    done
    exit "$status"

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
check-specs:
    #!/usr/bin/env -S mise exec -- bash
    # A shebang recipe bypasses the `shell` setting above, so it enters the
    # mise environment itself.
    set -euo pipefail

    # tracey is compiled without logging and warns about its default log
    # filter on every run. Turning logging off silences the warning.
    export RUST_LOG=off

    # tracey answers queries from a daemon per workspace, which picks up file
    # changes with a delay of a few seconds. Stop the daemon before the check,
    # so that a fresh daemon scans the workspace before it answers. The LSP
    # and MCP servers reconnect and start a new daemon on their next call.
    tracey kill >/dev/null

    # Broken or stale references, duplicate or malformed requirement IDs
    tracey query validate --deny warnings

    # A staged change to the text of a requirement needs a version bump.
    # tracey compares the index with HEAD. On a pull request, GitHub Actions
    # checks out a merge commit and stages nothing, so move HEAD to the base
    # branch, the first parent of the merge commit. The index keeps the
    # content of the pull request, and tracey compares the two.
    if [ -n "${GITHUB_BASE_REF:-}" ] && git rev-parse --quiet --verify HEAD^2 >/dev/null; then
        git reset --quiet --soft HEAD^1
    fi
    tracey pre-commit

    # Coverage is information, not a gate: a spec can land before its
    # implementation. The gaps are listed here and in the tracey dashboard.
    tracey query status

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

# Run a subset of checks as pre-commit hooks
pre-commit:
    @just pre-commit-checks

# Run the tests
#
# The recipe runs the harness instead of nextest, for the reason that
# `format-toml` gives. The action tests every workspace of the repository.
test-rust:
    mise run rakko -- test-rust
