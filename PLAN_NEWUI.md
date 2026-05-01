# PLAN_NEWUI.md — Ground-up flex-based pane system

## Why throw out the old one

The current pane code (`crates/frostcore/src/floating.rs` +
`crates/frostcore/src/widgets/foldable.rs`) is built on:

- Manual rect math for every layout decision (title rect, body rect,
  per-section card rect).
- Per-pane `ctx.data` caches that lag by one frame
  (`section_widths_key`, `section_states_key`, `section_count_key`,
  `actual_h_key`, `h_body_avail_key`).
- Hand-rolled "shrink to fit" and "scale by openness" lerps that
  fight egui's own layout primitives (`item_spacing`,
  `allocate_exact_size`).
- Many narrow patches (clip rects, post-toggle stores, section-state
  caches) that fix one path and break another.

Each new edge case (right-anchored panes, 5+ sections, fold during
animation, theme that hides the section frame, etc.) takes another
patch. The bug-per-patch ratio is climbing. **It's a structural
problem with the layout approach, not a series of independent
fixes.**

The right move is to throw the manual layout out and let
[`egui_flex`](crates/frostcore/src/features/flex/mod.rs) handle every
layout decision: items declare `basis`, `grow`, `shrink`; the flex
engine guarantees children fit and the parent never overflows.

## Constraints

1. **Existing code stays unchanged.** `floating.rs`, `foldable.rs`,
   the old demo (`crates/bevy_frost/examples/demo.rs`), and the
   `PaneBuilder` API all keep working. The old pane is the control —
   if the new one falls behind, we still have a working demo to
   compare against.
2. **New code in new files.** No edits to the existing pane stack.
3. **Phased delivery.** Each phase ends with a runnable example and
   a user-visible checkpoint. The user runs `make run-newui`,
   confirms it works, and only then do we move to the next phase.
4. **Visible verification at every step.** No "trust me, it works
   under the hood." Each phase exposes its work in the example so
   the layout is provable by inspection.

## File layout

```
crates/frostcore/src/pane2.rs               ← new pane builder
crates/bevy_frost/examples/newui.rs         ← new minimal demo
Makefile                                    ← `make run-newui` target
```

Optionally later:

```
crates/frostcore/src/section2.rs            ← new container (Phase 3+)
```

The number `2` is just a placeholder so the names don't clash. After
Phase 5 we can rename `pane2` → `pane` and delete the old file.

## Phase 0 — Read egui_flex (no code changes)

**Goal:** record exactly which `egui_flex` primitives we'll use and
what their guarantees are. This is a doc-only phase that grounds
every layout decision in Phase 1+.

**Deliverable:** an "API reference" appendix appended to *this* file
listing:

- `Flex::horizontal()` / `Flex::vertical()` — root flex container
- `FlexItem::basis(f32)` — preferred size along main axis
- `FlexItem::grow(f32)` — share of leftover space
- `FlexItem::shrink()` — give up space when crowded
- `Flex::gap(Vec2)`, `align_items`, `justify`
- `Flex::width(Size)` / `height(Size)` — fixed or `Size::Auto`
- `flex.add_ui(item, |ui| …)` — render a flex item

Plus three small things to verify before writing pane2:

1. Can a flex item be `paint-only` (a 25 px title strip with no
   children)? → `flex.add_ui(item().basis(25.0), |ui| {
   ui.painter().rect_filled(...) })`.
2. Does `Flex::vertical()` inside a fixed-height parent make a
   `grow(1.0)` body item exactly fill the leftover?
3. Does egui's text rotation (`epaint::TextShape { angle: π/2 }`)
   compose with flex layout, or do we need to allocate the rotated
   bbox manually?

**Time:** 30 min reading, 30 min writing the appendix.

## Phase 1 — Empty pane skeleton

**Goal:** a `Pane2` that renders **only a title**. No containers, no
body content. Title rotates correctly for all 4 rail orientations.
Theme drives the title visual.

### Pane2 API

```rust
pub struct Pane2 {
    id: egui::Id,
    title: String,
    anchor: PaneAnchor,
    accent: egui::Color32,
}

pub enum PaneAnchor {
    /// Vertical pane on the left edge of the screen.
    LeftRail  { zone: RailZone },
    /// Vertical pane on the right edge.
    RightRail { zone: RailZone },
    /// Horizontal pane on the top edge.
    TopRail   { zone: RailZone },
    /// Horizontal pane on the bottom edge.
    BottomRail{ zone: RailZone },
}

pub enum RailZone { Start, Middle, End }

impl Pane2 {
    pub fn new(id: impl Into<egui::Id>, title: impl Into<String>,
               anchor: PaneAnchor, accent: egui::Color32) -> Self;

    pub fn show(self, ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui));
}
```

`body` is the closure for future content (containers in Phase 3+);
in Phase 1 we just pass a no-op `|_| {}`.

### Internal layout (Phase 1)

```
Pane2::show(ctx, body) {
    egui::Area::new(area_id)
        .fixed_pos(anchor_to_pos(self.anchor))
        .order(Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(theme.pane_fill)
                .stroke(theme.border)
                .show(ui, |ui| {
                    // Pick flex direction by orientation.
                    // Vertical pane (Left/Right rail) → Flex::horizontal()
                    //   ⇒ [title strip ⨯ body]
                    // Horizontal pane (Top/Bottom rail) → Flex::vertical()
                    //   ⇒ [title strip / body]
                    Flex::*
                        .gap(Vec2::ZERO)
                        .show(ui, |flex| {
                            // Title order depends on rail side:
                            //   LeftRail  → [title, body]
                            //   RightRail → [body, title]
                            //   TopRail   → [title, body]
                            //   BottomRail→ [body, title]

                            flex.add_ui(item().basis(TITLE_THICKNESS), |ui| {
                                paint_pane_title(ui, &self.title, orientation, accent);
                            });
                            flex.add_ui(item().grow(1.0), |ui| {
                                body(ui); // no-op in phase 1
                            });
                        });
                });
        });
}
```

### Title painting

`paint_pane_title` is the only "manual" piece — it paints the strip
fill (PRO solid accent OR GAME caution stripes) and the rotated
title text. It receives the strip's *flex-allocated* rect and paints
inside it. **No layout decisions** — flex already gave it a rect.

- PRO: solid accent rect + UPPERCASE title text in contrast colour.
- GAME: animated caution stripes (existing helper) + scramble-decode
  title (existing helper).

Text rotation: `epaint::TextShape` with `angle = ±π/2` for vertical
strips, `0` for horizontal strips. Centred in the strip's
perpendicular axis. Text reading direction by rail:
- LeftRail strips: bottom-to-top
- RightRail strips: top-to-bottom
- TopRail strips: left-to-right (no rotation)
- BottomRail strips: left-to-right (no rotation)

### Pane size

For Phase 1, hard-code: vertical panes 280×320, horizontal panes
560×220. We're testing the *layout structure*, not the resize logic
yet. Resize handles come in Phase 6.

### Phase 1 verification gate

Once Phase 2 wires the example up, the user runs `make run-newui`
and confirms:
- All 12 panes open.
- Title is on the right edge (left/right/top/bottom) per anchor.
- Title text reads in the right direction per rail.
- Theme switch flips PRO ↔ GAME visuals on every pane.
- No overflow: pane never exceeds its declared size; title strip is
  exactly `TITLE_THICKNESS` thick; body fills the rest.

## Phase 2 — newui.rs example with 12 panes + theme buttons

**Goal:** a Bevy example that opens all 12 anchor positions
simultaneously and lets the user cycle theme/mode.

### Layout

The new ribbon has 4 rails × 3 zones:

```
TOP rail:    [theme-cycle][mode-cycle]│[P-TS][P-TM][P-TE]
LEFT  P-LS                                          P-RS  RIGHT
LEFT  P-LM                                          P-RM  RIGHT
LEFT  P-LE                                          P-RE  RIGHT
BOTTOM rail:                          [P-BS][P-BM][P-BE]
```

12 P-* buttons, each toggles one Pane2 at its anchor.

### Theme buttons

Two extra buttons live in the TOP rail's start zone:

- **theme-cycle**: rotates `theme_pro(Dark)` → `theme_game(Dark)` →
  `theme_pro(Dark)` …
- **mode-cycle**: flips Dark ↔ Light within the active theme family.

Implemented as `Bevy resources` mutated on click; theme is applied
via `set_theme(...)` inside the `EguiPrimaryContextPass` system, same
as the old demo.

### File: `crates/bevy_frost/examples/newui.rs`

Skeleton (~150 lines):

```rust
use bevy::prelude::*;
use bevy_egui::*;
use bevy_frost::prelude::*;

#[derive(Resource, Default)]
struct PaneOpen([bool; 12]);

#[derive(Resource)]
struct ThemeFamily(/* PRO=0 GAME=1 */ u8);

#[derive(Resource)]
struct ThemeMode(/* Dark=0 Light=1 */ u8);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .add_plugins(FrostPlugin)
        .init_resource::<PaneOpen>()
        .insert_resource(ThemeFamily(0))
        .insert_resource(ThemeMode(0))
        .add_systems(EguiPrimaryContextPass, draw_ui)
        .run();
}

fn draw_ui(/* contexts, panes, themes */) {
    // Apply theme
    let th = match (family.0, mode.0) {
        (0, 0) => theme_pro(Mode::Dark),
        (0, 1) => theme_pro(Mode::Light),
        (1, 0) => theme_game(Mode::Dark),
        (1, 1) => theme_game(Mode::Light),
        _ => theme_pro(Mode::Dark),
    };
    set_theme(th);

    // Top ribbon: theme buttons + 3 zone buttons
    // Left ribbon: 3 zone buttons
    // ...

    // Draw any open Pane2:
    for (i, anchor) in PANE_ANCHORS.iter().enumerate() {
        if open.0[i] {
            Pane2::new(format!("p{}", i), format!("PANE {}", i), *anchor, accent)
                .show(ctx, |_body| {
                    // empty for phase 1
                });
        }
    }
}
```

### Makefile

```make
run-newui:
    @DISPLAY=$(DISPLAY) $(RUN_WITH) $(CARGO) run -p bevy_frost --example newui
```

### Phase 2 verification gate

User opens `make run-newui`, clicks all 12 panes, switches theme
twice, switches mode twice. Confirms:

- 12 panes line up against their respective screen edges in 12
  distinct positions.
- Title text is correctly rotated and reads in the right direction.
- PRO ↔ GAME swap reads the new theme's pane fill, title fill,
  border/no-border.
- Dark ↔ Light swap re-tints panel fills + text colours.
- No pane overflows its declared size or the screen edge.

**STOP HERE until user confirms Phase 1 + Phase 2 are visually correct.**

## Phase 3 — Add ONE container to Pane2

**Goal:** add a single `Section2` (foldable container) that lives
inside the `body` closure. Same `Section2` instance used in all 12
panes. No widgets inside it yet.

### Section2 API

```rust
pane.show(ctx, |body| {
    Section2::new("s1", "Section 1")
        .show(body, |inner| {
            // empty for phase 3
        });
});
```

### Section2 internal layout

```rust
Section2::show(ui, body) {
    let state = CollapsingState::load_with_default_open(ui.ctx(), id, true);
    let openness = state.openness(ui.ctx());

    // Direction matches the parent pane's body direction.
    // Vertical pane body  → Flex::vertical()    (containers stack down)
    // Horizontal pane body → Flex::horizontal() (containers stack right)
    Flex::*
        .show(ui, |flex| {
            // Title strip — fixed basis on the cross-axis.
            flex.add_ui(item().basis(STRIP_THICKNESS), |ui| {
                paint_section_title(ui, ..., accent);
                // click handling lives in this paint helper
            });
            // Body — `grow(1.0)` so it consumes leftover; `basis(0)`
            // when folded so it disappears.
            let body_basis = openness * default_body_size;
            flex.add_ui(item().basis(body_basis).grow(1.0), |ui| {
                if openness > 0.001 {
                    inner(ui);
                }
            });
            // explicit state.store(ctx) — egui 0.33 toggle doesn't
            // persist (lesson learned).
        });
}
```

### Why this should "just work" under flex

- Flex handles overflow: if 5 sections want 5×360 px in a 1000 px
  body, each gets ~200 px proportionally, none overflows.
- `item_spacing` is replaced by `Flex::gap()` — single source of
  truth, never gets double-counted.
- Fold animation: section's `body_basis` lerps from 0 → default; the
  flex layout handles the rest. No "scale per section" cache, no
  per-frame width re-computation.

### Phase 3 verification gate

- Each of the 12 panes shows one container.
- Click container title → folds; click again → unfolds. State
  persists across frames.
- In a horizontal pane: container fills the full body width, title
  strip on the side per rail orientation.
- In a vertical pane: container fills body height, title strip on
  top.

## Phase 4 — Multiple containers + fold-driven flex

**Goal:** stack N containers in one pane; flex distributes space.

The user adds 5 containers to one of the panes; we verify:

- All 5 fit; none overflows.
- Folding 4 of the 5 makes the 5th expand to fill (flex.grow).
- Folding all 5 leaves a row of thin title strips. Pane height
  unchanged.
- Resize the pane (Phase 6 if not before) and the containers
  reflow.

## Phase 5 — Real widgets inside containers

**Goal:** copy the per-section widget bodies from the existing
demo's `widgets_panel`, `elements_panel`, `editor_panel`. The widget
code itself doesn't change — only the surrounding pane/section
shell does.

After this phase the new system has feature parity with the old
demo for typical use.

## Phase 6 — Polish (after parity)

In whatever order the user wants:

- User-resize handles (drag pane edge to resize).
- Drag-to-reorder containers.
- Per-pane scroll for overflow-handling (when content exceeds even
  flex's shrink budget).
- Fade-in stagger animation (re-port from old code).
- Migrate old `demo.rs` to use Pane2 / Section2 OR delete the old
  pane stack.

## Out of scope for this plan

- Anything in `floating.rs` / `foldable.rs` / old demo. They stay
  exactly as they are.
- Container-inside-container (subsections) — defer.
- Auto-fold on overflow — flex prevents overflow without it.
- Snarl / code-editor / palette — those are separate widgets and
  drop into Phase 5 unchanged.

## Open questions to settle in Phase 0

1. Does `egui_flex` need any `frame_builder` magic for paint-only
   items, or does plain `add_ui` work for a "rect_filled inside this
   slot" style?
2. Does `Flex::vertical()` inside a fixed-height parent (`Frame` →
   `Ui`) produce a `grow(1.0)` item that actually fills the
   remainder, or do we need `h_full()` on the body item?
3. How does `egui_flex` interact with `egui::Area`? Is there a
   parent that forces a max height that flex respects?

If any of those answers turn out badly, Phase 1 might need a small
glue layer around flex (a "fixed-size frame" wrapper that gives flex
a parent with bounded height). That's not a redesign — just an
extra helper.

## Approval gate

This plan is the contract:

- I do **not** edit `floating.rs`, `foldable.rs`, or `demo.rs`.
- I do **not** start Phase 3 until you confirm Phase 1 + Phase 2
  look right on screen.
- I do **not** add containers, drag, scroll, or anything else
  before each phase's checkpoint passes.

If you reply "go", I start with Phase 0 (the `egui_flex` API
appendix), then Phase 1 + 2 land together (you can't see Phase 1
without Phase 2's example).

If you want a different split, say so before I start.

---

# Phase 0 — egui_flex API reference (recorded after reading source)

Source: `crates/frostcore/src/features/flex/mod.rs` (vendored
`egui_flex` 0.6.0, MIT). Everything below is verified against that
file, not docs.rs.

## Core types

| Type | Purpose |
|---|---|
| `Flex` | The container builder. `Flex::horizontal()` / `Flex::vertical()` / `Flex::new().direction(...)`. |
| `FlexInstance` | Handle inside `flex.show(ui, |flex| ... )`. You call `flex.add_ui(...)` to add children. |
| `FlexItem` | Per-child config: `basis`, `grow`, `shrink`, `align_self`, `min_size`, `frame`. |
| `Size::Points(f32)` / `Size::Percent(f32)` | Fixed pixel size or percent of parent. |

## Container methods we need

- `Flex::horizontal()` → `direction = FlexDirection::Horizontal`.
- `Flex::vertical()` → `direction = FlexDirection::Vertical`.
- `.size(Vec2)` → both axes fixed (`width`/`height` to
  `Size::Points`).
- `.w_full()` / `.h_full()` → axis fills 100 % of parent. We use this
  on the body-axis when our outer Frame already has a fixed size
  from the Area.
- `.gap(Vec2)` → spacing between children. We pass `Vec2::ZERO`
  because the title strip butts against the body with no visible
  gutter.
- `.align_items(FlexAlign::Stretch)` → default; cross-axis stretches
  to the container's full extent. This is exactly what we want — a
  25-tall title strip stretches the full width of a horizontal
  pane.
- `.id_salt(impl Into<Id>)` → so two flex containers in the same Ui
  don't clash on egui's auto-id.

## Item methods we need

- `item().basis(N)` → preferred main-axis size N px. With no
  `grow`, the item stays at exactly `N` px (modulo `shrink`).
- `item().grow(1.0)` → consumes leftover space. With `grow=1` set on
  exactly one item, that item fills whatever the title strip didn't
  use. With multiple `grow=1` items, they share leftover.
- `item().shrink()` → opt-in shrink-below-basis when crowded.
  Useful for "many sections in a small pane" later (Phase 4).

## The "paint-only" pattern (for the title strip)

`flex.add_ui(item, |ui| { … })` runs the closure with a Ui whose
`max_rect` is the slot the flex layout assigned. To paint a coloured
strip with rotated text inside it, we just:

```rust
flex.add_ui(item().basis(TITLE_THICKNESS), |ui| {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0, accent);
    paint_pane_title_text(ui, rect, text, orientation, accent);
    // No widgets allocated → no layout interference.
});
```

Verified: a paint-only flex child works (no widgets allocated). The
flex engine only cares that the child returns a `FlexContainerResponse`
which `add_ui` builds for us.

## Sizing the outer flex

For Phase 1 we know the pane size up front (fixed 280×320 vertical
or 560×220 horizontal). We open `egui::Area::fixed_pos(pos)`, then
inside use `egui::Frame::new()...show(ui, |ui| {…})`. The Frame's
inner Ui has `max_rect` = pane size.

Inside that Ui:

```rust
egui::Frame::new()...show(ui, |ui| {
    Flex::horizontal() // or vertical
        .gap(Vec2::ZERO)
        .w_full()
        .h_full()
        .show(ui, |flex| {
            flex.add_ui(item().basis(STRIP_T), title_paint);
            flex.add_ui(item().grow(1.0), |ui| body(ui));
        });
});
```

`w_full()`+`h_full()` makes the flex container fill the Frame's
inner area. The two children then split that area: the title gets
exactly `STRIP_T` on the main axis; the body gets the rest via
`grow(1.0)`.

## Open questions resolved

1. **Paint-only items?** Yes — `add_ui` with a closure that only
   paints works fine.
2. **Body fills remainder?** Yes — `grow(1.0)` on the body item
   takes everything the title strip didn't consume.
3. **Rotated text inside flex item?** Doesn't matter for layout —
   the item's inner Ui has a rect; we paint a `TextShape` with
   `angle = ±π/2` inside that rect. Flex doesn't see the rotated
   bbox (we don't allocate space for it; we just paint).

No glue layer needed. We can go straight to Phase 1.
