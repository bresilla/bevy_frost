# Architecture

The kit has exactly **five tiers**. Anything new fits one of them — or
it doesn't ship.

```
TIER 1   Top level (egui::Context)
         ├─ Ribbons              persistent side rails
         └─ Free containers      floating overlays, no parent
                                   – command palette
                                   – context menu
                                   – maximize overlay
                                   – drop-ghost

TIER 2   Ribbon items
         ├─ Action button        RibbonRole::Icon   (one-shot)
         └─ Pane button          RibbonRole::Panel  (toggles a pane)

TIER 3   Pane                    floating window owned by a Panel button
         └─ hosts only containers — never raw widgets, never nested panes

TIER 4   Container               the bracketed accent-banner card
         ├─ widget               toggle, slider, drag_value, dropdown, …
         └─ sub-container        recurse into Tier 5

TIER 5   Sub-container           subsection / group_frame
         └─ widgets OR further sub-containers (recursive)
```

## The rules

1. **Ribbons and free containers are siblings**, never nested. They
   live on `ctx`.
2. **A pane is opened by exactly one ribbon button** (`RibbonRole::Panel`).
3. **A pane only contains containers.** No raw widgets at the pane
   level, ever.
4. **Containers contain widgets and/or sub-containers.** Sub-containers
   recurse.
5. **No tier skips down.** Widgets don't appear at Tier 1 / 2 / 3.
   Containers don't appear at Tier 1.
6. **One special case.** Some widgets are `maximizable(...)` —
   they can lift to a Tier-1 overlay (full-window) and snap back.
   Today: node graph, code editor.

## The two themes

`PRO` and `GAME` — set globally via `set_theme(...)`, applied each
frame by `apply_theme(ctx, accent, opacity)`. Custom themes are
`Theme { .. ..theme_game() }` literals.

## Where things live

| Tier              | Code                         |
|-------------------|------------------------------|
| Ribbons           | `ribbon/`                    |
| Free containers   | `command_palette.rs`, `widgets/context_menu.rs`, `maximize.rs`, `ribbon/ghost.rs` |
| Pane              | `floating.rs`                |
| Container         | `widgets/foldable.rs`        |
| Sub-container     | `widgets/subsection.rs`, `widgets/group.rs` |
| Widgets           | `widgets/`                   |
| Theme             | `style.rs`                   |

## Where new things go

| Kind of feature           | Tier |
|---------------------------|------|
| New action button         | 2 (`Icon`-role ribbon)  |
| New pane                  | 2 (`Panel`-role) + 3 (its sections) |
| New section / inspector   | 4 |
| New widget                | 4 |
| New nested-card pattern   | 5 |
| New always-on chrome      | 1 (free container) |
| Make a widget full-window | special — wrap in `maximizable(..)` |

If it doesn't fit, push back. Expanding the hierarchy is a deliberate
decision, not a default.
