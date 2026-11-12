---
name: Rakko
description: Project maintenance as versioned Rust crates.
colors:
  bg: "#ffffff"
  bg-soft: "#f6f7f8"
  bg-mute: "#eceef1"
  text: "#1d2025"
  text-2: "#565d67"
  text-3: "#676e78"
  divider: "#e3e6ea"
  border: "#d3d7dd"
  brand: "#c2410c"
  brand-hover: "#a8380a"
  brand-text: "#b13c0b"
  brand-soft: "#fdf0e9"
  on-brand: "#ffffff"
  terminal: "#1d2025"
  terminal-text: "#e7e8ea"
  terminal-accent: "#f2773d"
  code-keyword: "#24508a"
  code-entity: "#5b3f8f"
  code-string: "#2d6a3c"
  code-const: "#0f6a78"
  code-comment: "#6a727c"
  code-tag: "#2d6a3c"
  code-var: "#8a4f05"
  code-invalid: "#a3201b"
  bg-dark: "#16171a"
  bg-soft-dark: "#1d1f23"
  bg-mute-dark: "#272a2f"
  text-dark: "#e7e8ea"
  text-2-dark: "#a7aeb7"
  text-3-dark: "#959ca6"
  divider-dark: "#2a2d32"
  border-dark: "#3a3e45"
  brand-dark: "#f2773d"
  brand-hover-dark: "#f58c5a"
  brand-text-dark: "#f58c5a"
  brand-soft-dark: "#35211a"
  on-brand-dark: "#16171a"
  terminal-dark: "#0f1012"
  code-keyword-dark: "#8fb5ef"
  code-entity-dark: "#c4b3f0"
  code-string-dark: "#93cf9f"
  code-const-dark: "#74c7d5"
  code-comment-dark: "#8f98a2"
  code-tag-dark: "#93cf9f"
  code-var-dark: "#e2b56d"
  code-invalid-dark: "#ff8a80"
typography:
  display:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "clamp(3rem, 2rem + 3.5vw, 4.5rem)"
    fontWeight: 750
    lineHeight: 1.05
    letterSpacing: "-0.035em"
  headline:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "clamp(1.75rem, 1.4rem + 1vw, 2.25rem)"
    fontWeight: 700
    lineHeight: 1.25
    letterSpacing: "-0.02em"
  page-title:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "clamp(1.875rem, 1.5rem + 1.2vw, 2.375rem)"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  tagline:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "clamp(1.625rem, 1.2rem + 1.5vw, 2.375rem)"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  heading:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 650
    lineHeight: 1.25
    letterSpacing: "-0.01em"
  subtitle:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 650
    lineHeight: 1.25
  lede:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 400
    lineHeight: 1.6
  body:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.75
  label:
    fontFamily: "system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 500
    lineHeight: 1.4
  code:
    fontFamily: "ui-monospace, SF Mono, SFMono-Regular, Menlo, Consolas, Liberation Mono, monospace"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.7
    fontFeature: '"liga" 0, "calt" 0'
rounded:
  inline: "0.25rem"
  md: "0.5rem"
  lg: "0.75rem"
  full: "50%"
spacing:
  gutter: "clamp(1.25rem, 4vw, 3rem)"
  topbar: "4rem"
  measure: "37rem"
  content: "43rem"
  page: "72rem"
  frame: "90rem"
  sidebar: "17rem"
  outline: "14rem"
components:
  button-brand:
    backgroundColor: "{colors.brand}"
    textColor: "{colors.on-brand}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "0.625rem 1.25rem"
  button-brand-hover:
    backgroundColor: "{colors.brand-hover}"
    textColor: "{colors.on-brand}"
  button-alt:
    backgroundColor: "{colors.bg}"
    textColor: "{colors.text}"
    typography: "{typography.label}"
    rounded: "{rounded.md}"
    padding: "0.625rem 1.25rem"
  code-panel:
    backgroundColor: "{colors.bg-soft}"
    textColor: "{colors.text}"
    typography: "{typography.code}"
    rounded: "{rounded.md}"
    padding: "1rem 1.25rem 1.125rem"
  inline-code:
    backgroundColor: "{colors.bg-mute}"
    textColor: "{colors.text}"
    rounded: "{rounded.inline}"
    padding: "0.15em 0.4em"
  code-card:
    backgroundColor: "{colors.bg-soft}"
    textColor: "{colors.text}"
    typography: "{typography.code}"
    rounded: "{rounded.lg}"
  terminal-strip:
    backgroundColor: "{colors.terminal}"
    textColor: "{colors.terminal-text}"
    typography: "{typography.code}"
    padding: "0.875rem 1.25rem"
  sidebar-link:
    textColor: "{colors.text-2}"
    typography: "{typography.label}"
    padding: "0.3125rem 0"
  sidebar-link-current:
    textColor: "{colors.brand-text}"
  pager-link:
    backgroundColor: "{colors.bg}"
    textColor: "{colors.brand-text}"
    rounded: "{rounded.md}"
    padding: "0.75rem 1rem"
  layer-current:
    backgroundColor: "{colors.brand-soft}"
    textColor: "{colors.brand-text}"
    padding: "1.125rem 1.5rem"
---

# Design System: Rakko

## Overview

**Creative North Star: "The Developer Tool, Played Straight"**: the site of
Rakko is the documentation site of a developer tool, with no twist. It holds the
craft level of [mise.jdx.dev][mise]: a sticky top bar, a landing page with a
hero and a code sample, and documentation pages with a sidebar, a readable
column, and an outline. A Rust developer recognizes every part on sight, so the
attention goes to the code and the text. One system runs through the landing
page and every documentation page.

The ground is white (near-black in dark mode), the text near-black, and
secondary text a cool slate. Rakko orange is the only color with a voice. It
marks the name, the primary button, links, the current page, and focus.
Faces are the system UI face and the system monospace, with no web fonts.
Code sits in soft gray panels with 8px corners; cards have 12px corners; 1px
dividers separate sections. Depth is nearly flat: one soft shadow, under the
code card of the hero.

The site has no JavaScript. Every responsive change is plain CSS, and the
only motion is a 150 ms ease-out on the color and border of links and
buttons. This design replaced a "service manual" design whose landing page
was too dense and did not show that Rakko is a developer tool.

**Key Characteristics:**

- The first view proves the tool with code: the harness that a project
  writes, and the command that runs it.
- Orange is the brand color, and it is reserved for the name, the primary
  action, links, the current page, and focus.
- System faces only: system UI for text, system monospace for code.
- Soft code panels, 1px dividers, 8px and 12px corners, and one shadow.
- Light and dark mode from the same tokens, chosen by the system setting.
- No JavaScript; the layout adapts at 75rem, 60rem, and 40rem.

## Colors

A cool, near-neutral palette of white, slate, and near-black, with one warm
orange as the brand.

### Primary

- **Rakko Orange** (`brand`): the fill of the primary button and of the skip
  link, the focus outline, the caret, and the hover border of pager and
  index links. In dark mode it lightens (`brand-dark`) and the text on it
  turns near-black (`on-brand-dark`).
- **Rakko Orange Deep** (`brand-hover`): the primary button on hover.
- **Rakko Orange Text** (`brand-text`): orange as text. It colors the name in
  the hero, links in running text, the current page in the sidebar, the
  title of a pager link, and the hover state of navigation links. It is a
  shade darker than `brand` so that small text keeps its contrast.
- **Orange Wash** (`brand-soft`): the row of the current layer on the landing
  page, and the text selection.

### Neutral

- **White Ground** (`bg`): the page.
- **Soft Panel** (`bg-soft`): code panels, the code card, table headers, and
  the sidebar at phone width.
- **Mute Panel** (`bg-mute`): the chip behind inline code.
- **Near-Black Text** (`text`): headings and body text.
- **Slate** (`text-2`): secondary text, such as ledes, summaries,
  descriptions, and navigation links at rest.
- **Light Slate** (`text-3`): the quietest text, such as the language label
  of a code panel, list markers, and the line under the hero buttons.
- **Divider** (`divider`): 1px rules between sections, rows, and columns.
- **Border** (`border`): the outline of the secondary button, the code card,
  the layers list, and pager and index links.
- **Terminal** (`terminal`, `terminal-text`, `terminal-accent`): the dark
  strip under the hero code that shows the command, with the `$` prompt in
  the accent. It is dark in both modes.

### Syntax

Zola marks each code token with a class (`.z-1` to `.z-9`, from the
github-light theme), and the style sheet gives those classes the colors of
the site: a muted blue for keywords, violet for entities, green for strings
and tags, teal for constants, slate for comments, amber for variables, and
red for invalid code. Each color has a lighter partner in dark mode. The
generated `giallo.css` is not linked.

### Named Rules

**The One Orange Rule.** Orange marks the name, the primary action, links,
the current page, and focus, and nothing else. It never decorates a rule, a
border at rest, or a heading.

**The Text Orange Rule.** Small text in orange uses `brand-text`, never
`brand`; `brand` is for fills, outlines, and borders.

## Typography

**Display Font:** system-ui (with -apple-system, Segoe UI, Roboto, Helvetica
Neue, and Arial)
**Body Font:** the same system UI face
**Label/Mono Font:** ui-monospace (with SF Mono, Menlo, Consolas, and
Liberation Mono)

**Character:** The faces of the reader's own operating system, so the site
reads like the tools next to it. Weight and tight tracking carry the
hierarchy, not a display face. Ligatures are off in code, so `->` and `--`
show the characters that a reader types.

### Hierarchy

- **Display** (750, fluid 3rem to 4.5rem, line height 1.05, -0.035em): the
  name "Rakko" in the hero, in orange. Once per site.
- **Tagline** (650, fluid 1.625rem to 2.375rem, 1.2): the line under the name
  in the hero.
- **Page Title** (700, fluid 1.875rem to 2.375rem, 1.2): the title of a
  documentation page or section index.
- **Headline** (700, fluid 1.75rem to 2.25rem, 1.25): the heading of a
  section of the landing page.
- **Heading** (650, 1.5rem): a second-level heading in running text, with a
  divider and 1.5rem of space above it. Third-level headings are 650 at
  1.1875rem.
- **Subtitle** (650, 1.125rem): the heading of a feature or a documentation
  kind on the landing page.
- **Lede** (400, 1.125rem, 1.6): the lede of the hero and the summary under
  a page title, in slate.
- **Body** (400, 1rem, 1.75): running text, at a measure of 37rem, about 75
  characters.
- **Label** (500, 0.875rem): the top bar, the sidebar, the facts strip, and
  the footer. The outline uses 0.8125rem, and the language label of a code
  panel and the "Previous" and "Next" of the pager use 0.75rem.
- **Code** (400, 0.875rem, 1.7): code panels. The hero card uses 0.8125rem.
  Inline code is 0.875em of its text.

### Named Rules

**The System Face Rule.** The site loads no font. Text uses the system UI
face and code the system monospace.

**The Measure Rule.** Prose holds a measure of 37rem. Code panels and tables
may run to the 43rem column.

**The Whole Flag Rule.** Inline code never wraps, so a flag such as `--fix`
or a path stays whole on one line.

## Layout

A centered frame of 90rem holds the top bar, the documentation grid, and the
footer. The landing page sits in a narrower 72rem frame. The side gutter is
fluid, from 1.25rem to 3rem.

The landing page stacks bands of 3.5rem to 5.5rem of vertical space with a
1px divider between them. The hero is two equal columns: the copy on the
left, the code card on the right. A strip of facts between two dividers
follows it. The features run in four columns; the layers band and the band
on actions of your own split 5 to 7.

A documentation page is a grid of a 17rem sidebar and the main column. The
main column holds the page in a 43rem column, with a 14rem outline on the
right. The sidebar and the outline are sticky under the 4rem top bar and
scroll on their own.

Responsive changes, all in CSS:

- **Below 75rem:** the outline is hidden.
- **Below 60rem:** the hero and the split bands become one column, the
  features and documentation kinds two. The sidebar moves below the page on
  a soft panel, and a "Menu" link in the top bar jumps to it.
- **Below 40rem:** the top bar stops being sticky, stacks the navigation
  under the wordmark, and drops "Get Started", which the hero and the
  sidebar already offer. Features and kinds become one column, and code
  panels in running text bleed to the edges of the screen.

**The No-Script Rule.** Every responsive change is CSS. Navigation that
needs a toggle becomes a link to a place on the page.

## Elevation & Depth

The system is flat. Depth comes from tonal panels (`bg-soft` on `bg`) and 1px
dividers and borders. The top bar is translucent, at 92% of the ground with a
blur behind it, so it reads as a layer over the page without a shadow.

### Shadow Vocabulary

- **Code Card Lift** (`box-shadow: 0 12px 32px -12px rgb(0 0 0 / 0.14)`): the
  code card of the hero, and nothing else.

**The One Shadow Rule.** Only the code card of the hero casts a shadow. Every
other surface is flat.

## Shapes

Corners are gently rounded and come in a short scale: 4px for the chip of
inline code, 8px for buttons, code panels, and pager and index links, and
12px for the larger cards, the code card of the hero and the list of layers.
The Aonyx logo in the footer is clipped to a circle. Lines are 1px, with one
exception: each feature on the landing page stands under a 2px divider. At
phone width a code panel in running text loses its corners and runs to the
edges.

## Components

### Buttons

Plain and confident, in the convention of developer tool sites.

- **Shape:** gently rounded (8px), with 0.625rem by 1.25rem of padding, 600
  weight at 0.9375rem.
- **Primary:** an orange fill with white text; it darkens on hover. Get
  Started carries an arrow drawn as an inline stroke.
- **Secondary:** a white fill with a 1px border; the border darkens to light
  slate on hover.
- **Focus:** a 2px orange outline, 2px away from the button, as on every
  focusable element.
- **Motion:** 150 ms ease-out on the background and the border color.

### Code Panels

- **Style:** a soft gray panel with 8px corners, monospace at 0.875rem and a
  line height of 1.7, and a tab size of 4. The panel scrolls sideways and
  never wraps code.
- **Head:** the language ("Rust", "Shell", "TOML", "YAML", "JSON") in light
  slate at 0.75rem above the code, or the file name in monospace when the
  block gives one. The head stays in place when the code scrolls.
- **Inline code:** a mute chip with 4px corners, at 0.875em; inside a link it
  takes the orange of the link.

### Hero Code Card

The signature of the landing page and the proof that Rakko is a developer
tool. A 12px card with a 1px border and the one shadow. It holds two
panels, each under a bar with its path. `tools/rakko/Cargo.toml` lists the
dependencies of a harness on three actions, from the front matter of the
landing page. `tools/rakko/src/main.rs` mounts each action by its type; the
template reads it at build time from the compiled example crate
`site/examples/landing`. Both keep their indentation and scroll sideways. A
dark terminal strip under them shows the command
`mise run rakko -- format-rust --fix` behind an orange `$` prompt.

### Lists and Cards

- **Layers:** a bordered list with 12px corners. Each row gives a name in a
  7rem column and its role beside it; the row of Rakko sits on the orange
  wash with its name in orange.
- **Features:** four columns for actions, the harness, bundles, and updates.
  Each stands under a 2px divider with a title and a slate description.
- **Index links and pager links:** blocks with a 1px border and 8px corners,
  whose border turns orange on hover. Pager links give "Previous" or "Next"
  in light slate above the title in orange.

### Navigation

- **Top bar:** sticky, 4rem high, translucent with a blur, and a 1px divider
  under it. The wordmark "Rakko" sits on the left in 750 at 1.25rem with
  tight tracking; the links (Get Started, Guides, Concepts, Reference, and
  the GitHub mark) sit on the right in slate, and turn orange on hover.
- **Sidebar:** groups by kind of page, each under a bold heading that links
  to its index, with a divider between groups. Links are slate, near-black
  on hover, and the current page is orange at 600 weight.
- **Outline:** "On this page" and the headings of the page, at 0.8125rem,
  behind a 1px divider on its left.
- **Phone:** see Layout.

### Running Text

Links in text are orange, 500 weight, with an underline at 35% of the color
that becomes solid on hover. The landing page uses this style for links in
its introduction, facts, overview, and layers. Linked tool names
in headings keep the heading weight. Headings link to themselves without
looking like links. Tables scroll sideways, with a soft header row, 1px cell
borders, and faint banding.

### Footer

A 1px divider, then the Aonyx logo clipped to a circle beside "Rakko is
built by Aonyx", and the license line with an underlined orange link to
GitHub.

## Do's and Don'ts

### Do

- **Do** keep orange to the name, the primary action, links, the current
  page, and focus.
- **Do** use `brand-text` for orange text and `brand` for fills and outlines.
- **Do** keep prose at the 37rem measure, and let code and tables run to
  43rem.
- **Do** keep inline code on one line, and keep the indentation of code in
  panels with a sideways scroll.
- **Do** show real code from a compiled example crate wherever a page shows
  the API.
- **Do** separate sections with 1px dividers and soft panels, not shadows.
- **Do** define every color as a token on `:root` with a partner in the dark
  mode block.
- **Do** limit motion to 150 ms ease-out on color and border, and drop it
  for reduced motion.

### Don't

- **Don't** load a web font; use the system UI face and the system
  monospace.
- **Don't** add JavaScript; responsive navigation uses links to places on
  the page.
- **Don't** put an eyebrow or kicker label above a heading.
- **Don't** add a shadow to any surface other than the code card of the
  hero.
- **Don't** use orange for decorative rules or borders at rest.
- **Don't** make the landing page dense or editorial; it is a developer tool
  landing page with a hero, a code sample, and short bands.
- **Don't** draw or imitate the Aonyx logo; use the supplied file.

[mise]: https://mise.jdx.dev
