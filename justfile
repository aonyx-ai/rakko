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
    just prettier true
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

# Build the Rust documentation and force the rustdoc lints to run
build-rustdoc:
    cargo doc --workspace --no-deps --document-private-items

# Check that Rakko builds with the latest dependencies
check-latest-deps force="false":
    #!/usr/bin/env -S mise exec -- bash

    # Abort if git is not clean
    if [[ {{ force }} != "true" && -n $(git status --porcelain) ]]; then
        echo "Git working directory is not clean. Commit or stash changes before running this recipe. Aborting."
        git status --porcelain

        # Print diff on GitHub Actions
        if [ -n "$GITHUB_ACTIONS" ]; then
            git diff
        fi

        exit 1
    fi

    # Update dependencies to latest versions
    cargo update

    # Run tests to ensure the latest versions are compatible
    RUSTFLAGS="-D deprecated" cargo test --all-features --all-targets --locked

# Check that dependencies have compatible open-source licenses and trusted sources
check-dependencies:
    cargo deny check bans licenses sources

# Check that Rakko builds with the minimal dependencies
check-minimal-deps force="false":
    #!/usr/bin/env -S mise exec -- bash
    set -euo pipefail

    # Abort if git is not clean
    if [[ {{ force }} != "true" && -n $(git status --porcelain) ]]; then
        echo "Git working directory is not clean. Commit or stash changes before running this recipe. Aborting."
        git status --porcelain

        # Print diff on GitHub Actions
        if [ -n "${GITHUB_ACTIONS:-}" ]; then
            git diff
        fi

        exit 1
    fi

    # Install the nightly toolchain if not already installed
    rustup install nightly

    # Update dependencies to minimal versions
    rustup run nightly cargo update -Z direct-minimal-versions

    # Run the tests on the toolchain that the project pins. Only the resolution
    # needs the nightly, and a test that starts cargo would otherwise inherit
    # the nightly through RUSTUP_TOOLCHAIN and miss the components of the pin.
    RUSTFLAGS="-D deprecated" cargo test --all-features --all-targets --locked

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
check-msrv:
    #!/usr/bin/env -S mise exec -- bash

    # Get the MSRV from the Cargo.toml
    MSRV=$(cat Cargo.toml | grep 'rust-version =' | head -n 1 | cut -d '"' -f 2)

    # Install the MSRV toolchain if not already installed
    rustup install "${MSRV}"

    # Run tests using the MSRV
    RUSTFLAGS="-D deprecated" rustup run "${MSRV}" cargo check --all-features --all-targets

# Check that all dependencies in Cargo.toml are used
check-unused-deps:
    #!/usr/bin/env -S mise exec -- bash

    # Install the nightly toolchain if not already installed
    rustup install nightly

    # Check for unused dependencies
    rustup run nightly cargo udeps

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
format-yaml fix="false": (prettier fix "{yaml,yml}")

# Lint GitHub Actions workflows
lint-github-actions:
    zizmor -p .

# Lint Markdown files
lint-markdown:
    markdownlint **/*.md

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
lint-yaml:
    yamllint .

# Run a subset of checks as pre-commit hooks
pre-commit:
    @just pre-commit-checks

# Auto-format files with prettier
prettier fix="false" extension="*":
    prettier {{ if fix == "true" { "--write" } else { "--list-different" } }} --ignore-unknown "**/*.{{ extension }}"

# Run the tests
#
# The recipe runs the harness instead of nextest, for the reason that
# `format-toml` gives. The action tests every workspace of the repository.
test-rust:
    mise run rakko -- test-rust
