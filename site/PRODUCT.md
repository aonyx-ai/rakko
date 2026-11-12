# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Rust developers who maintain projects. Two readers have equal weight:

- A developer who sets up a harness, so that Rakko runs the maintenance
  tasks of their project.
- An author who builds actions of their own, for the workflows of their own
  projects.

Jan David gave the reason for the equal weight: actions are specific to each
user. Aonyx has its own actions and uses Rakko in its own way, and other
people use it differently. So the site teaches people how they reach their
own goals with the tool, not how to copy the setup of Aonyx.

## Product Purpose

Rakko turns project maintenance into versioned Rust crates. A maintenance
task, such as formatting the TOML files or checking the licenses of the
dependencies, is a crate called an action. A project mounts the actions that
it wants in a small binary crate, the harness, and runs them as one command
line. An update to an action reaches the project as a pull request, like any
other dependency.

The website is where a user outside Aonyx learns Rakko: a landing page, a
tutorial, how-to guides, explanations, and a reference page that links to
the API documentation on docs.rs. Success means that a reader sets up a
harness, or builds an action of their own, from the site alone.

## Positioning

Rakko replaces the scripts and Justfile recipes that every project copies
and that drift apart. Each task is a crate with a version, tests, and a pin
of the tool that it runs, so a project gets a new release of a task through
its normal dependency updates, and CI shows whether the project still
passes. Rakko is the middle layer of three: mise installs the tools,
Rakko provides the actions and the machinery that runs them, and Clawless
turns the mounted actions into a command line.

## Operating Context

Readers arrive from crates.io, docs.rs, GitHub, and the README of the Rakko
repository. They read in a browser, often next to an editor and a terminal,
and copy commands and code from the pages. The how-to guides are long and
hold many code blocks: Rust, TOML, YAML, and shell commands.

## Capabilities and Constraints

- The site is a static site that Zola 0.23.6 builds from `site/`. It has no
  JavaScript and no Node toolchain. Styles are plain CSS in `site/static/`.
- The pages follow the four kinds of Diátaxis: tutorial, how-to guide,
  explanation, and reference. The navigation groups the pages by kind and is
  generated from the sections, so a new Markdown file adds a page.
- The Rust code on a page comes from compiled example crates in
  `site/examples/`. Zola highlights code at build time.
- The address is `rakko.aonyx.ai` for now. It can change when Aonyx moves its
  open-source projects under one site, and the site does not plan for that.
- The site starts with twelve pages and grows with Rakko: a new feature
  brings its own pages.
- The site documents Rakko and how people use it. It has no page for each
  crate of the catalog.

## Brand Commitments

- Rakko has an identity of its own. Aonyx appears as its maker, with its
  logo.
- The site looks like the documentation site of a developer tool, at the
  craft level of mise.jdx.dev: a landing page with a hero and a code sample,
  and documentation pages with a sidebar, a readable column, and an outline.
  Jan David chose this on 2026-11-12, after a first design in the style of
  a service manual made the landing page too dense and did not show that
  Rakko is a developer tool.
- The name Rakko (ラッコ) is Japanese for sea otter, the otter that keeps a
  pebble as its tool.
- Voice: welcoming. Each page starts with what the reader can do, and states
  a limit as encouragement. No coined phrases, such as "young API". No text
  about the version of a crate or the gate for 1.0.
- The actions, bundles, and commands of the repository are built for the
  workflows of Aonyx. The site presents an action of the reader's own as a
  reasonable choice.

## Evidence on Hand

- The Aonyx logo exists. Jan David supplies the file; the site must not draw
  or imitate it.
- No Rakko logo or mark exists.
- No testimonials, users, adoption numbers, or benchmarks exist. The site
  must not invent them. The only stated reason for the support promise is
  that Aonyx uses Rakko internally.
- The supported crates are `rakko`, `rakko-cli`, `rakko-tool`, and
  `rakko-test-utils`.

## Product Principles

- Teach people to reach their own goals, not to copy the setup of Aonyx.
- Every code sample that shows the API compiles in CI.
- Lead with what the reader can do.
- Stay small and static: one generator, no JavaScript, no Node.
