# Create llm-wiki-hugo-cms Project

## Context

llm-wiki is a git-backed wiki engine. A wiki repository has this
structure:

```
my-wiki/
├── wiki.toml                    ← wiki config + type registry
├── schemas/                     ← JSON Schema per type
├── inbox/                       ← drop zone (not published)
├── raw/                         ← immutable archive (not published)
└── wiki/                        ← compiled knowledge (this is the content)
    ├── concepts/
    │   ├── scaling-laws.md
    │   └── mixture-of-experts/
    │       ├── index.md
    │       └── moe-routing.png
    ├── sources/
    │   └── switch-transformer-2021.md
    ├── queries/
    │   └── moe-routing-comparison.md
    └── skills/
        └── ingest/
            └── index.md
```

Pages are Markdown files with YAML frontmatter. The `wiki/` directory
is the content root. Pages are either flat files (`slug.md`) or bundles
(`slug/index.md` with co-located assets).

The engine exposes 16 MCP tools for search, read, list, graph, etc.
But for static site generation we don't need MCP — we read the wiki
tree directly from the filesystem.

**llm-wiki-hugo-cms** turns a wiki repository into a Hugo site. The
wiki is the CMS — Hugo is the renderer. The project provides:

1. A Hugo site scaffold that reads directly from the wiki tree
2. Hugo configuration that maps wiki frontmatter to Hugo conventions
3. A GitHub Actions CI pipeline that builds and deploys to GitHub Pages

This prompt creates the *scaffold project*. Configuring a specific wiki
to use it (generating type-specific cascades, layouts, and frontmatter
mapping from the wiki's registered types) is handled by the
`configure-hugo` skill in `llm-wiki-skills`.

## Design documents to read first

- `docs/focused-llm-wiki-design.md` — the engine design, tool surface
- `docs/type-specific-frontmatter.md` — type system, JSON Schema,
  default types, frontmatter fields
- `docs/roadmap.md` — the llm-wiki-hugo-cms section at the bottom
- `docs/specifications/core/frontmatter-authoring.md` — all frontmatter
  fields, per-type templates
- `docs/specifications/core/repository-layout.md` — wiki repo structure,
  slug resolution, flat vs bundle pages

## Your Task

Create the complete `llm-wiki-hugo-cms` project with all files ready
to commit. The project is a Hugo site scaffold designed to be placed
inside (or alongside) a wiki repository.

## Key design decision: Hugo content directory = wiki/

Hugo's `contentDir` points directly at the wiki's `wiki/` directory.
No copy step, no transform step, no build script. Hugo reads the
Markdown files in place.

This works because:
- Wiki pages are already Markdown with YAML frontmatter
- Hugo ignores unknown frontmatter fields
- Hugo's bundle model (folder + `index.md`) matches the wiki's bundle
  model
- Assets co-located in bundles are served by Hugo automatically

What needs configuration:
- Frontmatter field mapping (wiki fields → Hugo fields)
- `wiki://` URI resolution in links
- Type-based sections and layouts
- Pages to exclude (status: stub, status: generated)
- Directories to exclude (inbox/, raw/, schemas/)

## Repository structure to create

```
llm-wiki-hugo-cms/
├── .github/
│   └── workflows/
│       └── hugo-deploy.yml       ← CI: build + deploy to GitHub Pages
├── archetypes/                   ← Hugo archetypes (optional, wiki uses its own scaffolding)
├── assets/
│   └── css/
│       └── custom.css            ← minimal custom styles
├── content/                      ← symlink or Hugo contentDir override → wiki/
├── data/                         ← Hugo data files (optional)
├── layouts/
│   ├── _default/
│   │   ├── baseof.html           ← base template
│   │   ├── list.html             ← section list
│   │   └── single.html           ← single page
│   ├── partials/
│   │   ├── head.html             ← <head> with meta from frontmatter
│   │   ├── header.html           ← site header + nav
│   │   ├── footer.html           ← site footer
│   │   ├── metadata.html         ← page metadata (type, status, confidence, owner)
│   │   ├── graph.html            ← mermaid graph rendering
│   │   ├── backlinks.html        ← pages that link to this page
│   │   └── superseded.html       ← supersession notice banner
│   ├── concepts/
│   │   └── single.html           ← concept page layout (sources, claims)
│   ├── sources/
│   │   └── single.html           ← source page layout
│   ├── skills/
│   │   └── single.html           ← skill page layout
│   └── shortcodes/
│       ├── wikilink.html         ← resolve wiki:// URIs
│       └── mermaid.html          ← mermaid diagram rendering
├── static/
│   └── js/
│       └── mermaid.min.js        ← mermaid library (or CDN reference)
├── hugo.toml                     ← Hugo configuration
├── README.md
├── CHANGELOG.md
├── LICENSE
└── Makefile                      ← convenience targets: serve, build, deploy
```

## File specifications

### hugo.toml

The Hugo configuration must:

- Set `contentDir` to `../wiki` (assumes the Hugo site is at the wiki
  repo root or one level deep — document both options)
- Configure frontmatter mapping:
  - `lastmod` from `last_updated`
  - `description` from `summary`
  - `draft` from `status` (draft/stub → true, active/generated → false)
- Configure taxonomies:
  - `tags` from `tags`
  - `authors` from `owner` (single value treated as list)
  - `types` from `type`
- Exclude directories: `inbox/`, `raw/`, `schemas/`
- Enable Mermaid rendering (for graph output)
- Set reasonable defaults: title from `wiki.toml` name, baseURL
  configurable

```toml
baseURL = "https://example.github.io/my-wiki/"
title = "My Wiki"                    # override with wiki name
languageCode = "en"

contentDir = "../wiki"               # point directly at wiki/

# Frontmatter mapping
[frontmatter]
  lastmod = ["last_updated", ":git"]
  date = ["last_updated"]

# Taxonomies from wiki frontmatter
[taxonomies]
  tag = "tags"
  author = "owner"
  type = "type"

# Ignore non-content directories if contentDir is repo root
ignoreFiles = ["^inbox/", "^raw/", "^schemas/", "LINT\\.md$"]

# Markup configuration
[markup]
  [markup.goldmark]
    [markup.goldmark.renderer]
      unsafe = true                  # allow raw HTML for mermaid
  [markup.highlight]
    style = "monokai"

# Output formats
[outputs]
  home = ["HTML", "RSS", "JSON"]
  section = ["HTML", "RSS"]

# Params available in templates
[params]
  description = "A wiki powered by llm-wiki and Hugo"
  mermaid = true                     # enable mermaid.js
  showMetadata = true                # show type, status, confidence
  showBacklinks = true               # show pages linking here
  showSuperseded = true              # show supersession banner
```

### Layout templates

#### layouts/_default/single.html

The single page template must:

- Render the page title from frontmatter `title` (or `name` for skills)
- Show the metadata partial (type badge, status, confidence, owner,
  last_updated)
- Show the superseded partial if `superseded_by` is set
- Render the Markdown body
- Show the backlinks partial
- Show related pages (from `sources`, `concepts` frontmatter fields)

#### layouts/_default/list.html

The section list template must:

- List child pages with title, summary, type badge, status
- Group by type if the section contains mixed types
- Show the section's own `index.md` content above the list

#### layouts/partials/metadata.html

Render a metadata block from frontmatter:

- Type badge (colored by type category: knowledge, source, extension)
- Status indicator (active, draft, stub, generated)
- Confidence level if present
- Owner if present
- Last updated date
- Tags as links to taxonomy pages

#### layouts/partials/superseded.html

If `.Params.superseded_by` is set, render a banner:

```
⚠️ This page has been superseded by [Page Title](/path/to/replacement/).
```

Resolve the `superseded_by` slug to a Hugo URL.

#### layouts/partials/backlinks.html

Find all pages that reference the current page in their `sources`,
`concepts`, or body links. Render as a "Pages that link here" section.

Note: Hugo's `.Site.Pages` can be filtered, but backlink detection
from body content requires a custom approach (regex on body for
`wiki://` URIs or `[[slug]]` patterns, or pre-computed data file).
Document the limitation and provide a pragmatic solution.

#### layouts/concepts/single.html, sources/single.html, skills/single.html

Type-specific layouts that extend the default single:

- **Concepts**: show sources list, claims table, confidence
- **Sources**: show concepts this source informs, claims
- **Skills**: show skill-specific fields (allowed-tools, context,
  agent, compatibility)

#### layouts/shortcodes/wikilink.html

Resolve `wiki://` URIs to Hugo-relative URLs:

```
{{< wikilink "wiki://research/concepts/moe" >}}
→ /concepts/moe/
```

Handle both `wiki://<name>/<slug>` (strip wiki name) and
`wiki://<slug>` (direct).

#### layouts/shortcodes/mermaid.html

Render a Mermaid diagram:

```
{{< mermaid >}}
graph LR
  A --> B
{{< /mermaid >}}
```

### GitHub Actions CI

#### .github/workflows/hugo-deploy.yml

The workflow must:

1. Trigger on push to `main` branch
2. Checkout the repository (which contains both the wiki and the Hugo
   site)
3. Install Hugo (extended version for SCSS support)
4. Build the site: `hugo --minify`
5. Deploy to GitHub Pages using `actions/deploy-pages`

```yaml
name: Deploy Hugo site to GitHub Pages

on:
  push:
    branches: ["main"]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: "pages"
  cancel-in-progress: false

defaults:
  run:
    shell: bash

jobs:
  build:
    runs-on: ubuntu-latest
    env:
      HUGO_VERSION: "0.147.0"
    steps:
      - name: Install Hugo
        run: |
          wget -O ${{ runner.temp }}/hugo.deb \
            https://github.com/gohugoio/hugo/releases/download/v${HUGO_VERSION}/hugo_extended_${HUGO_VERSION}_linux-amd64.deb
          sudo dpkg -i ${{ runner.temp }}/hugo.deb

      - name: Checkout
        uses: actions/checkout@v4
        with:
          submodules: recursive
          fetch-depth: 0        # full history for .GitInfo

      - name: Setup Pages
        id: pages
        uses: actions/configure-pages@v5

      - name: Build with Hugo
        env:
          HUGO_CACHEDIR: ${{ runner.temp }}/hugo_cache
          HUGO_ENVIRONMENT: production
          TZ: UTC
        run: |
          hugo \
            --gc \
            --minify \
            --baseURL "${{ steps.pages.outputs.base_url }}/"

      - name: Upload artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: ./public

  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    needs: build
    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

Adapt the Hugo working directory if the Hugo site is in a subdirectory
(e.g., `site/` within the wiki repo). Document both layouts:

**Option A — Hugo site at wiki repo root:**
```
my-wiki/
├── hugo.toml          ← contentDir = "wiki"
├── layouts/
├── wiki/
├── wiki.toml
└── ...
```

**Option B — Hugo site in a subdirectory:**
```
my-wiki/
├── site/
│   ├── hugo.toml      ← contentDir = "../wiki"
│   └── layouts/
├── wiki/
├── wiki.toml
└── ...
```

The CI workflow should detect which layout is used or be configurable.

### Makefile

```makefile
.PHONY: serve build clean

serve:
	hugo server --buildDrafts --navigateToChanged

build:
	hugo --gc --minify

clean:
	rm -rf public/ resources/
```

### README.md

Write a README that covers:

- What this project is (Hugo site scaffold for llm-wiki)
- The key insight: `contentDir = wiki/`, no copy/transform step
- Prerequisites (Hugo extended, a wiki repository)
- Two setup options:
  - Option A: Hugo site at wiki repo root
  - Option B: Hugo site in a subdirectory
- How to set up:
  1. Copy/clone the Hugo files into your wiki repo
  2. Adjust `hugo.toml` (baseURL, title, contentDir path)
  3. Run `hugo server` to preview
  4. Push to trigger GitHub Pages deployment
- Frontmatter mapping table (wiki field → Hugo field)
- How type-specific layouts work
- How `wiki://` URIs are resolved
- How to customize (themes, layouts, CSS)
- GitHub Pages deployment setup (enable Pages, set source to Actions)
- Limitations:
  - Backlink detection is approximate (frontmatter links only, not
    body `[[links]]` unless pre-computed)
  - `wiki://` URIs in body text need shortcode or render hook
  - Skill pages render as documentation, not as executable skills
- Link to the llm-wiki engine repo
- License

### CHANGELOG.md

Initial entry for v0.1.0.

### LICENSE

MIT OR Apache-2.0 dual license (same as llm-wiki engine).

## Frontmatter mapping reference

The templates must handle this mapping:

| Wiki field | Hugo usage | Template access |
|------------|-----------|-----------------|
| `title` | Page title | `.Title` |
| `name` | Page title (skills) | `.Params.name` (fall back to `.Title`) |
| `summary` | Meta description | `.Params.summary` or `.Description` |
| `description` | Meta description (skills) | `.Params.description` |
| `type` | Section/layout selection | `.Params.type` |
| `status` | Draft flag | `.Draft` (via frontmatter mapping) or `.Params.status` |
| `last_updated` | Last modified | `.Lastmod` (via frontmatter mapping) |
| `tags` | Taxonomy | `.Params.tags` |
| `owner` | Author taxonomy | `.Params.owner` |
| `tldr` | Highlighted summary | `.Params.tldr` |
| `confidence` | Badge | `.Params.confidence` |
| `sources` | Related pages list | `.Params.sources` (resolve slugs to pages) |
| `concepts` | Related pages list | `.Params.concepts` (resolve slugs to pages) |
| `superseded_by` | Banner + redirect | `.Params.superseded_by` |
| `read_when` | Not rendered (agent-facing) | Ignored in templates |
| `claims` | Claims table | `.Params.claims` |
| `allowed-tools` | Skill metadata | `.Params.allowed_tools` |
| `compatibility` | Skill metadata | `.Params.compatibility` |

## Wiki type → Hugo section mapping

Wiki pages live in arbitrary directories (`concepts/`, `sources/`,
`skills/`, or custom). Hugo uses the directory as the section. The
type-specific layouts are selected by matching the wiki `type` field
to a Hugo layout directory.

Use Hugo's `type` frontmatter or cascade to select layouts:

```toml
# hugo.toml — cascade type-based layouts
[[cascade]]
  [cascade.params]
    layout_type = "concept"
  [cascade._target]
    path = "concepts/**"

[[cascade]]
  [cascade.params]
    layout_type = "source"
  [cascade._target]
    path = "sources/**"
```

Or use a lookup in the template:

```html
{{ $layoutType := .Params.type | default "default" }}
{{ partial (printf "types/%s-metadata.html" $layoutType) . }}
```

Document both approaches and recommend the simpler one.

## Quality checklist

After creating all files, verify:

- [ ] `hugo server` runs without errors on a sample wiki
- [ ] `hugo.toml` contentDir points to wiki/ correctly
- [ ] Frontmatter mapping produces correct `.Lastmod`, `.Draft`,
  `.Description`
- [ ] Type badges render for concept, paper, article, skill, etc.
- [ ] Superseded banner appears when `superseded_by` is set
- [ ] Tags, owner, type taxonomies generate list pages
- [ ] Bundle pages (folder + index.md) render with co-located assets
- [ ] Flat pages (slug.md) render correctly
- [ ] Section index pages render with child page lists
- [ ] Mermaid diagrams render in the browser
- [ ] GitHub Actions workflow builds and deploys successfully
- [ ] inbox/, raw/, schemas/ directories are excluded from the site
- [ ] Pages with `status: stub` or `status: draft` are excluded from
  production build (but visible with `--buildDrafts`)
- [ ] README documents both setup options (root vs subdirectory)
