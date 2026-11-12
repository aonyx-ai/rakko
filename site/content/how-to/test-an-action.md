+++
title = "Test an Action"
description = "Test outcomes and make sure that read-only runs leave project files unchanged."
weight = 30
+++

Test your action in a temporary project and inspect its outcome. The
[`rakko-test-utils`][test-utils] helpers also detect file changes when a run
declares that it only reads the project.

This guide tests `check-readme` from [Build an Action][build-an-action].
You need an action to test, but no harness.

## Add the Test Dependencies

In your action crate, add `rakko-test-utils`, `tempfile` for temporary
projects, and Tokio to run asynchronous tests:

```sh
cargo add --dev rakko-test-utils tempfile
cargo add --dev tokio --features macros,rt
```

Choose a release of `rakko-test-utils` that uses the same release of `rakko`
as your action. Otherwise, the helpers expect a different `Action` trait and
the tests do not compile.

## Test a Run

Put the tests in `tests/`, such as `tests/check_readme.rs`. Start the file with
these imports:

{{ <example
  crate="check-readme"
  file="tests/check_readme.rs"
  region="imports" /> }}

A test creates a project in a temporary directory, runs the action there, and
checks the outcome:

{{ <example
  crate="check-readme"
  file="tests/check_readme.rs"
  region="fails" /> }}

`Context` uses the temporary directory as the project root.
`run_checking_access` runs the action with your context and arguments, then
returns its outcome. The action is asynchronous, so `#[tokio::test]` supplies
the runtime that the test needs.

To test a project that passes, create its files before the run:

{{ <example
  crate="check-readme"
  file="tests/check_readme.rs"
  region="passes" /> }}

## Test With Arguments

The fields of `CheckReadmeArgs` are private. Use `from_values` to construct
arguments through the same interface as the command line:

{{ <example
  crate="check-readme"
  file="tests/check_readme.rs"
  region="values" /> }}

A boolean flag that the user gave has the value `true`. The test then runs the
action with these arguments:

{{ <example crate="check-readme" file="tests/check_readme.rs" region="fix" /> }}

Test `access` directly. The method reads no files and starts no tools:

{{ <example
  crate="check-readme"
  file="tests/check_readme.rs"
  region="access" /> }}

## The Access Check

`run_checking_access` reads the access declaration before it runs the action.
For a read-only run, it compares project files before and after the run.
Added, removed, or changed files fail the test, and the message lists them.
For a run that declares a write, the helper returns the outcome without
comparing files.

The comparison excludes these files and directories at every depth:

- Directories named `.git`.
- Directories named `target`.
- Files whose names end in `.tsbuildinfo`.

The harness trusts access declarations and can run readers at the same time.
If an action writes after declaring a read, it can change another action's
input. Use this helper to catch those changes in tests.

## Test an Action That Runs a Tool

A test can inspect a tool declaration without installing the tool:

{{ <example crate="check-toml" file="tests/check_toml.rs" region="tools" /> }}

This test reads the version from the same [mise] file as the action. So it still
passes when Renovate updates the pin.

A test that runs the tool needs the tool at the version that your crate pins.
Install it in the root of the crate before you run the tests:

```sh
MISE_ENV=rakko mise install
```

`run_checking_access` calls your action directly and bypasses the platform
skip. If the tool has no build for the test platform, the action returns an
error. Limit tests that run the tool to its supported platforms.
[Run a Tool From an Action][run-a-tool] explains platform declarations.

## Compare Paths

Windows uses different path separators from macOS and Linux. Use `path` to
convert a literal such as `src/lib.rs` to the platform's path format. Use
`path_text` when you need that path as text, for example in an expected tool
report.

## Run the Tests

Run the tests from your action crate:

```sh
cargo test
```

[build-an-action]: @/how-to/build-an-action.md
[mise]: https://mise.jdx.dev
[run-a-tool]: @/how-to/run-a-tool.md
[test-utils]: https://docs.rs/rakko-test-utils
