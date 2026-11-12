---
version: 1
slug: "templates-index-html"
primary_target: "templates/index.html"
related_targets:
  [
    "templates/base.html",
    "templates/doc.html",
    "templates/page.html",
    "templates/section.html",
    "static/style.css",
  ]
---

# The Rakko Website

## Scope

The whole site in `site/`: the landing page (Persuade) and the twelve
documentation pages with their section indexes (Read). One system runs
through both.

- Audience: Rust developers who set up a harness, and authors who build
  actions of their own, with equal weight.
- Action: open Get Started. Proof: the harness code that a project writes,
  and the three layers, mise, Rakko, and Clawless.
- Constraints: Zola templates and plain CSS, no JavaScript, light and dark,
  phone width, long guides with many code blocks.

## Direction contract

THESIS: The documentation site of a developer tool, played straight. The
landing page shows the code that a project writes, and the documentation has
the sidebar, outline, and code panels that a Rust developer expects. It
refuses a dense, editorial landing page that hides that Rakko is a developer
tool.

OWN-WORLD: White ground (near-black in dark mode), near-black text, slate for
secondary text, and Rakko orange as the brand color for the name, the primary
button, links, and the current page. The system UI face for text and the
system monospace for code. Code sits in soft panels with 8px corners, cards
have 12px corners, and 1px dividers separate sections. One soft shadow, on
the code card of the hero.

STORY: The visitor sees that Rakko is a Rust developer tool, reads the
harness that a project writes, learns the four parts (actions, the harness,
bundles, and updates) and the three layers, and opens Get Started. The
tutorial explains what each Rust action runs.

FIRST VIEWPORT: On the left, "Rakko" in orange at display size, the tagline
under it, the lede, the buttons Get Started (orange) and Build an Action
(outline), and the line about what the reader needs. On the right, a code
card with two files: `tools/rakko/Cargo.toml`, which depends on three actions,
from the front matter, and `tools/rakko/src/main.rs`, which mounts each
action by its type, from the compiled example crate `site/examples/landing`.
The command
`mise run rakko -- format-rust --fix` sits in a dark strip under them. Below
both, a strip of three facts. Jan David chose the two files in live mode on
2026-11-12, so the hero shows concrete actions and not a bundle.

FORM: The category standard, chosen by Jan David in his own words on
2026-11-12 ("orient it closer towards what a landing page for a developer
tool would look like"), with mise.jdx.dev as the bar. It replaces the
Service Manual. seed:6740ce09fa1390b9. Signature interaction: none beyond the
conventions, with links and buttons that change color on hover. Motion
grammar: 150 ms ease-out on the color and border of links and buttons,
nothing else animates.

<!-- markdownlint-disable MD013 -->

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

<!-- markdownlint-enable MD013 -->

## Unresolved

- The Aonyx logo ships as the PNG that Jan David gave (white ground, black
  line art), clipped to its circle.
