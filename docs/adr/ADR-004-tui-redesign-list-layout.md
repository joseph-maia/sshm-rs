# ADR-004: TUI Redesign -- Table-to-List Layout with Modern Styling

## Status
Proposed

## Context
The current TUI uses a tabular layout (`ratatui::Table` widget) with six fixed columns
(Status, Name, User, Hostname, Port, Tags) and a large ASCII-art banner. The landing page
at `site/index.html` presents a mockup with a fundamentally different visual language:

- A compact text header instead of ASCII art
- A bordered, icon-decorated search bar
- A **list-style** layout where each host occupies a full-width row containing an icon,
  hostname, colored tag badges, and a right-aligned username
- Group headers rendered as standalone label rows with collapse arrows
- A bottom status bar showing keybinding hints in a `key label` pill format

The redesign must preserve all existing functionality (groups, multi-select, favorites,
status indicators, sidebar, overlays) while changing the visual presentation.

## Decision
Replace the `ratatui::Table` widget in `draw_table` with manual row rendering using
`Paragraph`/`Line`/`Span` primitives, and rework the header, search bar, and status bar
to match the mockup.

## Implementation Plan

### Step 1 -- Replace ASCII banner with compact header (Low complexity)
**File:** `src/ui/views/list.rs` (functions: `draw_title`, `draw_compact_title`, constant `ASCII_TITLE`)

- Remove the `ASCII_TITLE` constant entirely.
- Replace `draw_title` and `draw_compact_title` with a single `draw_header` function.
- Render two lines:
  - Line 1: "SSH Connection Manager" in bold cyan (`styles::cyan()` or a new `header_title` style).
  - Line 2: "{N} connections . {G} groups" in muted text. Compute N from
    `app.filtered_hosts.len()` and G from `app.groups.groups.len()`.
- Change `TITLE_HEIGHT` to 2 and `TITLE_HEIGHT_COMPACT` to 1 (just the title, no subtitle).
- Update the vertical layout constraint in `draw` from `Constraint::Length(title_height)`
  accordingly.
- **Affected downstream:** `src/ui/event.rs` imports `TITLE_HEIGHT` / `TITLE_HEIGHT_COMPACT`
  for mouse-click offset calculations. Update those constants or make the event handler use
  the new values.
- Update `visible_rows()` in `src/ui/app.rs` to reflect the reduced header height.

### Step 2 -- Redesign the search bar (Low complexity)
**File:** `src/ui/views/list.rs` (function: `draw_search_bar`)

- Add a search icon as the first `Span`: use Unicode magnifying glass U+1F50D or the
  simpler U+2315. Fallback: the text character "/" if Nerd Font is not available.
- Remove the "(/ to focus)" / "(Esc to unfocus)" instructional text from inside the bar;
  the status bar keybinding hints will cover discoverability.
- Keep the `Block` with `Borders::ALL` and `BorderType::Rounded`.
- When focused, tint the background with a subtle blue overlay:
  `Style::default().bg(Color::Rgb(0x14, 0x1a, 0x2a))` (matching the CSS
  `rgba(88,166,255,0.08)` on the dark background).
- Render cursor as a block character `\u{2588}` (full block) or `|` that toggles visibility
  based on a frame counter (for blink effect), or simply keep the current underscore `_`.

### Step 3 -- Convert table to list rendering (High complexity -- core change)
**File:** `src/ui/views/list.rs` (function: `draw_table`)

**Replace the entire `draw_table` function** with a new `draw_host_list` function.

#### 3a. Layout structure per host row
Each host row is a single `Line` composed of `Span` elements:
```
[indent][icon] [hostname................] [tag1] [tag2] [username]
```

- **Indent**: 2 spaces for hosts inside a group, 0 for ungrouped.
- **Icon**: Status-aware icon. Online = green circle U+25CF, Offline = red circle U+25CF,
  Unknown = hollow circle U+25CB, Connecting = dotted circle U+25CC. Alternatively, use
  Nerd Font server icon (see Step 6).
- **Hostname**: `host.name` (the SSH config alias). Takes `flex` space. Bold + cyan when
  selected, default fg otherwise. If the host is a favorite, prepend star U+2605 in yellow.
- **Tags**: Each tag rendered as a colored badge Span (see Step 4).
- **Username**: Right-aligned, muted color, format `host.user`.

#### 3b. Group header rows
Render group headers as a single `Line`:
```
[collapse_arrow] [GROUP_NAME] ([count])
```
- Collapse arrow: U+25BE (down-pointing triangle) when expanded, U+25B8 (right-pointing) when collapsed.
- Group name: uppercase, muted color, small letter-spacing effect (add a space between chars
  is too extreme -- just use uppercase).
- Style: `styles::muted()` foreground.

#### 3c. Selected row highlighting
- The selected row gets `bg(styles::selection_bg())` applied to the entire `Line`.
- The hostname span within the selected row switches to `styles::cyan()` foreground.
- Multi-selected rows get `bg(Color::Rgb(0x1e, 0x2a, 0x3a))` and a checkmark prefix.

#### 3d. Rendering approach
- Use a `Vec<Line>` and render via `Paragraph` inside a `Block` with rounded borders.
- Manual scrolling: slice the `Vec<Line>` using `app.table_offset..end` (same logic as
  current `visible_range`).
- Render a `Scrollbar` widget on the right side (keep existing scrollbar code).

#### 3e. Width management
- Hostname span: use `Constraint`-free approach. Calculate available width, subtract fixed
  elements (indent + icon + tags + user), and truncate hostname to fit.
- Alternatively, use a simpler approach: render the full line and let ratatui clip at the
  boundary. This works because `Paragraph` clips by default.

### Step 4 -- Tag badge rendering (Medium complexity)
**File:** `src/ui/views/list.rs` (inside the new `draw_host_list`)

Each tag becomes a `Span` with:
- Foreground color based on tag content (semantic mapping).
- Background color as a dimmed version of the foreground.

Tag color mapping (matching the CSS mockup):
| Tag contains | Foreground | Background (approx 12% opacity on dark bg) |
|---|---|---|
| "prod" | `#f85149` (red) | `Rgb(0x28, 0x14, 0x13)` |
| "staging" | `#f0883e` (orange) | `Rgb(0x28, 0x1c, 0x10)` |
| "dev" | `#3fb950` (green) | `Rgb(0x10, 0x22, 0x12)` |
| default | `#bc8cff` (purple) | `Rgb(0x20, 0x18, 0x2a)` |

Implementation:
- Create a helper function `fn tag_style(tag: &str) -> Style` in `styles.rs` or inline.
- Each tag Span: `Span::styled(format!(" {} ", tag), tag_style(tag))` -- note the padding
  spaces to simulate badge padding.
- Separate tags with a single space Span between them.

**New styles to add to `src/ui/styles.rs`:**
- `tag_prod_style() -> Style`
- `tag_staging_style() -> Style`
- `tag_dev_style() -> Style`
- `tag_default_style() -> Style`

Optionally, add `tag_bg_prod`, `tag_bg_staging`, etc. to `src/theme.rs` Theme struct so
users can customize tag colors via `theme.json`. This is a nice-to-have, not required for
the initial redesign.

### Step 5 -- Redesign the status bar (Medium complexity)
**File:** `src/ui/views/list.rs` (function: `draw_status_bar`)

The mockup shows keybinding hints in a pill/kbd format:
```
[Enter] connect  [a] add  [e] edit  [f] sftp  [?] help
```

- Replace the current left-side "? Help" text with a sequence of key-label pairs.
- Each pair: `Span` for the key (bold, primary color or slightly highlighted bg) + `Span`
  for the label (muted).
- Separate pairs with a `Span::raw("  ")` (double space).
- Keep the right side: connection count and sort mode.
- Keep the toast message logic (when a toast is active, it replaces the keybinding hints).
- Keep the multi-select mode status text.
- Keep the config warning indicator on the right.

Default keybinding hints to show:
- `Enter connect | a add | e edit | f sftp | ? help`
- When in search mode: `Enter validate | Tab switch | Esc close`
- When multi-selected: `Space toggle | b broadcast | d delete | Ctrl+a all | Esc clear`

### Step 6 -- Unicode and Nerd Font considerations (Low complexity)
**Files:** `src/ui/views/list.rs`, potentially `src/theme.rs`

- The mockup uses emoji (server icon, database icon). In a real terminal, emoji rendering
  is inconsistent -- they often occupy 2 cells and break alignment.
- **Recommendation**: Use Unicode geometric shapes (circles for status) as the app already
  does, NOT emoji. The status circles (U+25CF, U+25CB, U+25CC) work reliably.
- For the search icon, use U+2315 (TELEPHONE RECORDER) or simply the `/` character,
  which is thematic and universally supported.
- Do NOT depend on Nerd Fonts for the base experience. The app should look correct with
  any monospace font.
- Optional: detect Nerd Font availability (via env var or config flag) and swap in richer
  glyphs (e.g., nf-md-server for hosts, nf-md-magnify for search).

### Step 7 -- Update `visible_rows()` calculation (Low complexity)
**File:** `src/ui/app.rs`

- The `visible_rows()` method currently accounts for the old layout heights
  (title 5/1 + search 3 + table header 2 + status 1 + padding 2).
- New layout: header 2/1 + search 3 + list border 2 + status 1 = 8/7.
- Update the `reserved` calculation to match.

### Step 8 -- Update mouse click offset calculations (Low complexity)
**File:** `src/ui/event.rs`

- The mouse handler uses `TITLE_HEIGHT` and `TITLE_HEIGHT_COMPACT` to calculate which row
  was clicked. After changing header height, these constants must be updated.
- The table header row (column headers) is being removed, so the click offset calculation
  for "which host did I click" needs adjustment: previously it was
  `title_height + search_bar(3) + table_header(1) + border(1)`, now it will be
  `header_height + search_bar(3) + border(1)`.

## Alternatives considered

- **Keep Table widget, just restyle**: Pros: minimal code change. Cons: Table widget
  enforces columnar layout with fixed headers, making the list-style mockup impossible.
  Tags-as-badges and right-aligned usernames cannot be achieved within Table cells
  naturally.

- **Use ratatui::List widget instead of Paragraph**: Pros: built-in selection state and
  scrolling. Cons: `List` widget has its own highlight mechanism that is less flexible
  than manual `Line`-based rendering for per-span styling (e.g., different colors for
  hostname vs tags vs user within one row). The `Paragraph` approach gives full control.

- **Incremental migration (keep table, add badges)**: Pros: lower risk. Cons: the
  fundamental layout difference (columnar vs list-style) means the table columns for User,
  Hostname, Port become redundant in the new design, making this a half-measure.

## Consequences

- **Breaking visual change**: Users accustomed to the table layout will see a different UI.
  No data or functionality is lost.
- **`draw_table` deletion**: The largest function in `list.rs` (~190 lines) is replaced
  entirely. All overlay functions (`draw_delete_confirm`, `draw_info_overlay`, etc.) are
  untouched.
- **Theme extensibility**: Tag colors introduce a new styling concept. If added to
  `theme.json`, the Theme struct gains new fields (breaking existing `theme.json` files
  unless defaults are provided via `#[serde(default)]`).
- **Terminal compatibility**: Sticking with Unicode geometric shapes ensures broad terminal
  support. No emoji dependency.
- **Test impact**: If there are snapshot or integration tests for the TUI rendering, they
  will need updating. Currently there appear to be none.

## Complexity Estimates

| Step | Description | Complexity | Estimated Lines Changed |
|------|-------------|------------|------------------------|
| 1 | Replace ASCII banner with compact header | Low | ~30 |
| 2 | Redesign search bar | Low | ~20 |
| 3 | Convert table to list rendering | High | ~200 |
| 4 | Tag badge rendering + styles | Medium | ~50 |
| 5 | Redesign status bar | Medium | ~60 |
| 6 | Unicode/font considerations | Low | ~10 |
| 7 | Update visible_rows() | Low | ~5 |
| 8 | Update mouse click offsets | Low | ~10 |
| **Total** | | | **~385** |

Recommended implementation order: 1 -> 7 -> 8 -> 2 -> 4 -> 3 -> 5 -> 6

## Date
2026-03-11
