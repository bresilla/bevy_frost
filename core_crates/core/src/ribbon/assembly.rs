//! App-declared ribbon **assembly** — a richer model than the
//! original [`super::layout`] / [`super::static_ribbon`] shapes.
//!
//! # What it lets you describe
//!
//! * **One ribbon per edge.** Left / Right / Top / Bottom. The app
//!   declares which edges have ribbons and what goes on them.
//!
//! * **Two roles.** A ribbon is either a `Panel` (buttons open
//!   exclusive menus, floating panel anchored to its cluster) or an
//!   `Icon` (buttons fire actions, no panel).
//!
//! * **Three layout modes.**
//!   - `Centered` — one cluster centred along the edge,
//!   - `OneSided(end)` — one cluster anchored at one end of the edge,
//!   - `TwoSided` — two clusters, one at each end. Each cluster is a
//!     separate "corner": e.g. on the Left edge, `Start` cluster
//!     buttons live at the top-left corner, `End` cluster buttons at
//!     the bottom-left. A button's cluster dictates the anchor of the
//!     panel it opens.
//!
//! * **Exclusivity is per ribbon.** Opening any button's panel on
//!   ribbon `R` closes whatever other button on `R` had a panel
//!   open — regardless of cluster.
//!
//! * **Width is per cluster.** A Left/TwoSided ribbon stores two
//!   widths — one for the `Start` cluster's panels, one for `End`.
//!   Panels on opposite clusters of the same ribbon can therefore
//!   have different widths, and resizing one doesn't touch the other.
//!
//! * **Drag rules are per ribbon.** Each ribbon decides whether its
//!   buttons can be reordered (`draggable`) and whether it'll
//!   **accept** buttons dragged from other ribbons (`accepts`).
//!
//! * **Nested ribbons.** An icon-ribbon button can declare a
//!   `child_ribbon` — another ribbon that becomes active when its
//!   parent is pressed. (Paint support lands in a follow-up; field
//!   is reserved now so callers can plumb it through today.)
//!
//! # What this module currently delivers
//!
//! Phase 1 (this commit): types + layout + click dispatch (non-drag,
//! non-nested). Phase 2 will add drag with the accept-list + child
//! ribbons.

#[cfg(feature = "bevy")]
use bevy::prelude::*;
use egui;
use std::collections::HashMap;

use super::paint::{
    EDGE_GAP, SIDE_BTN_GAP, SIDE_BTN_SIZE, paint_ribbon_button, paint_ribbon_glyph,
    ribbon_button_fg,
};

// ─── Enums: edge / cluster / mode / role ────────────────────────────

/// Which screen edge a ribbon sits on.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RibbonEdge {
    Left,
    Right,
    Top,
    Bottom,
}

impl RibbonEdge {
    /// `true` for vertical rails (Left, Right) — buttons stack
    /// vertically. `false` for horizontal bars (Top, Bottom).
    pub fn is_vertical(self) -> bool {
        matches!(self, RibbonEdge::Left | RibbonEdge::Right)
    }
}

/// A cluster position along a ribbon. `Start` is the "top" or
/// "left" end depending on edge orientation, `End` is the opposite
/// corner, and `Middle` is the midpoint of the edge (only meaningful
/// in `Centered` / `ThreeSided` layouts).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RibbonCluster {
    Start,
    Middle,
    End,
}

/// How buttons distribute along a ribbon's edge.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RibbonMode {
    /// All buttons in one cluster, centred along the edge.
    Centered,
    /// All buttons in one cluster, hugging one end.
    OneSided(RibbonCluster),
    /// Two independent clusters, one at each end.
    TwoSided,
    /// Three independent clusters: one at each end *and* one
    /// centred. Buttons declare which cluster they belong to
    /// (`Start`, `Middle`, or `End`).
    ThreeSided,
}

/// Whether a ribbon's buttons open exclusive menus (panels) or just
/// fire actions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RibbonRole {
    /// Buttons open an exclusive floating panel. One panel per
    /// ribbon open at a time.
    Panel,
    /// Buttons are one-shot actions — no panel, no selected state.
    Icon,
}

// ─── Declarations: ribbons + items ──────────────────────────────────

/// Declaration of one ribbon. The app supplies a slice of these to
/// [`draw_assembly`]; frost places each ribbon + its buttons.
#[derive(Clone, Debug)]
pub struct RibbonDef {
    /// Stable id for this ribbon. Must be unique across the app.
    pub id: &'static str,
    /// Which screen edge this ribbon lives on.
    pub edge: RibbonEdge,
    /// Whether buttons here open panels or fire actions.
    pub role: RibbonRole,
    /// How buttons cluster along the edge.
    pub mode: RibbonMode,
    /// Can buttons be reordered *within* this ribbon?
    /// (Phase-1 stub — drag lands with phase 2.)
    pub draggable: bool,
    /// Ids of other ribbons this one will accept dropped buttons
    /// from. Use `&["*"]` to accept any. Phase-1 stub.
    pub accepts: &'static [&'static str],
}

/// What gets painted inside a ribbon button. Three forms:
///
/// * `Text` — short text label (1–3 chars typically), the kit's
///   original behaviour.
/// * `Icon` — Fluent UI System Icon name, looked up via
///   [`crate::icons::icon`] and rendered as a glyph in the bundled
///   icon font.
/// * `Svg` — raw SVG markup, painted via egui's image loader. The
///   host must install an SVG image loader (e.g.
///   `egui_extras::install_image_loaders` with the `svg` feature)
///   for this to render.
#[derive(Clone, Copy, Debug)]
pub enum RibbonGlyph {
    Text(&'static str),
    Icon(&'static str),
    Svg(&'static str),
}

impl From<&'static str> for RibbonGlyph {
    fn from(s: &'static str) -> Self {
        let trimmed = s.trim_start();
        if trimmed.starts_with("<svg") || trimmed.starts_with("<?xml") {
            RibbonGlyph::Svg(s)
        } else {
            RibbonGlyph::Text(s)
        }
    }
}

/// Declaration of one button that lives inside a ribbon.
#[derive(Clone, Copy, Debug)]
pub struct RibbonItem {
    /// Stable id for this button — also the `RibbonOpen` key when
    /// this button's panel is the active one on its ribbon.
    pub id: &'static str,
    /// Id of the ribbon that owns this button.
    pub ribbon: &'static str,
    /// Which cluster the button belongs to. Ignored for `Centered`
    /// ribbons. For `OneSided(end)` ribbons must match `end` (wrong
    /// values are coerced).
    pub cluster: RibbonCluster,
    /// Slot index within the cluster — 0 is nearest the anchor end.
    pub slot: u32,
    /// What to paint inside the button — text, Fluent icon, or SVG.
    /// See [`RibbonGlyph`].
    pub glyph: RibbonGlyph,
    /// Hover tooltip.
    pub tooltip: &'static str,
    /// If set, this icon-ribbon button pops a nested ribbon on press.
    /// Phase-1 stub — carried on the struct so callers can already
    /// wire the data; renderer lands with phase 2.
    pub child_ribbon: Option<&'static str>,
    /// Per-item override for the ribbon's [`RibbonRole`]. `None`
    /// means "inherit from the parent ribbon's `role`". `Some(...)`
    /// lets a single button behave differently from its rail —
    /// most useful for dropping a one-shot `RibbonRole::Icon`
    /// button into a `RibbonRole::Panel` ribbon (or vice-versa)
    /// without spawning a separate rail just for that button.
    pub role: Option<RibbonRole>,
}

impl RibbonItem {
    /// Effective role for this item — `role` if set, else the
    /// owning ribbon's role.
    #[inline]
    pub fn effective_role(&self, ribbon: &RibbonDef) -> RibbonRole {
        self.role.unwrap_or(ribbon.role)
    }
}

// ─── State resources ────────────────────────────────────────────────

/// Per-ribbon exclusive slot — the id of the button whose panel is
/// currently open on that ribbon, if any. Key is the ribbon id.
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Default, Debug, Clone)]
pub struct RibbonOpen {
    pub per_ribbon: HashMap<&'static str, &'static str>,
}

impl RibbonOpen {
    pub fn get(&self, ribbon: &'static str) -> Option<&'static str> {
        self.per_ribbon.get(ribbon).copied()
    }

    pub fn is_open(&self, ribbon: &'static str, item: &'static str) -> bool {
        self.per_ribbon.get(ribbon).copied() == Some(item)
    }

    /// Toggle: clicking the currently-open button closes it;
    /// clicking a different button swaps.
    pub fn toggle(&mut self, ribbon: &'static str, item: &'static str) {
        let current = self.per_ribbon.get(ribbon).copied();
        if current == Some(item) {
            self.per_ribbon.remove(ribbon);
        } else {
            self.per_ribbon.insert(ribbon, item);
        }
    }

    /// Close every currently-open pane across every ribbon. Use
    /// from command-palette actions that open ONE pane — call
    /// this first so previously-open panes don't leak through.
    pub fn close_all(&mut self) {
        self.per_ribbon.clear();
    }

    /// Force a specific pane open, closing any other panes on
    /// the same ribbon. Equivalent to `close_all()` +
    /// `toggle(ribbon, item)` with the guarantee that `item`
    /// ends up open afterwards regardless of the previous state.
    pub fn set(&mut self, ribbon: &'static str, item: &'static str) {
        self.per_ribbon.insert(ribbon, item);
    }
}

/// Panel widths keyed by `(ribbon_id, cluster)`. Widths persist
/// across frames so a cluster's panels remember the user's drag.
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Default, Debug, Clone)]
pub struct RibbonWidth {
    pub per_cluster: HashMap<(&'static str, RibbonCluster), f32>,
}

impl RibbonWidth {
    pub fn get(&self, ribbon: &'static str, cluster: RibbonCluster) -> Option<f32> {
        self.per_cluster.get(&(ribbon, cluster)).copied()
    }

    pub fn set(&mut self, ribbon: &'static str, cluster: RibbonCluster, value: f32) {
        self.per_cluster.insert((ribbon, cluster), value);
    }
}

/// Runtime overrides for where each button lives — written by the
/// drag system so a user-dragged button "sticks" in its new spot
/// across frames. The key is the item id; the value is its current
/// `(ribbon, cluster, slot)`. Any item not in the map uses its
/// declared position from the static `&[RibbonItem]` slice.
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Default, Debug, Clone)]
pub struct RibbonPlacement {
    pub overrides: HashMap<&'static str, (&'static str, RibbonCluster, u32)>,
}

impl RibbonPlacement {
    /// Resolved position for `item`, folding in any user drag.
    pub fn resolve(&self, item: &RibbonItem) -> (&'static str, RibbonCluster, u32) {
        self.overrides
            .get(item.id)
            .copied()
            .unwrap_or((item.ribbon, item.cluster, item.slot))
    }
}

/// Drag state for the in-flight button, if any. `cursor` is updated
/// every frame while the drag is active; `source` is snapshotted at
/// drag-start so the reflow logic can reference the origin even
/// after `overrides` has moved on.
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Default, Debug, Clone)]
pub struct RibbonDrag {
    pub item: Option<&'static str>,
    pub cursor: Option<egui::Pos2>,
    pub source: Option<(&'static str, RibbonCluster, u32)>,
}

// ─── Layout ─────────────────────────────────────────────────────────

/// Resolve an item's effective cluster, folding in the ribbon's
/// layout mode — `Centered` ribbons treat every item as `Middle`;
/// `OneSided(end)` coerces everything to `end`; `TwoSided` /
/// `ThreeSided` pass the item's own cluster through (coercing an
/// out-of-range `Middle` back to `Start` on `TwoSided`).
fn effective_cluster(mode: RibbonMode, item: RibbonCluster) -> RibbonCluster {
    match mode {
        RibbonMode::Centered => RibbonCluster::Middle,
        RibbonMode::OneSided(end) => end,
        RibbonMode::TwoSided => match item {
            RibbonCluster::Middle => RibbonCluster::Start,
            other => other,
        },
        RibbonMode::ThreeSided => item,
    }
}

/// Resulting position for one button on the screen.
#[derive(Clone, Copy, Debug)]
struct ButtonPlacement {
    anchor: egui::Align2,
    offset: egui::Vec2,
}

/// Compute where a button should land on screen given its ribbon's
/// edge + mode and the button's own cluster + slot. Centred ribbons
/// additionally need the total button count (`cluster_total`) so the
/// row-width can be computed; `None` falls back to 0 for
/// non-centred ribbons.
/// Per-edge content insets. Each field is the coordinate where the
/// rail along the *perpendicular* axis starts its first button (or
/// where the last button ends, mirrored). When the perpendicular
/// rail at that edge exists the inset = its outer button edge plus
/// one inter-button gap so corners read as one tight row of icons;
/// when the perpendicular rail is absent the inset collapses to
/// `EDGE_GAP` (just screen-edge padding) so the rail is free to
/// render right up to the screen corner.
///
/// In practice:
/// * `left` / `right` inset Top and Bottom rails (so they don't
///   overlap the side rails' corner buttons).
/// * `top` / `bottom` inset Left and Right rails (so they don't
///   overlap the horizontal rails' corner buttons).
#[derive(Clone, Copy, Debug, Default)]
pub struct SideInsets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

fn compute_side_insets(ribbons: &[RibbonDef]) -> SideInsets {
    // First-button offset with a perpendicular rail present = that
    // rail's button outer edge (`EDGE_GAP + SIDE_BTN_SIZE`) plus
    // one button gap. Without it: just the screen-edge padding.
    let with_rail = EDGE_GAP + SIDE_BTN_SIZE + SIDE_BTN_GAP;
    let inset = |present: bool| if present { with_rail } else { EDGE_GAP };
    SideInsets {
        left: inset(ribbons.iter().any(|r| r.edge == RibbonEdge::Left)),
        right: inset(ribbons.iter().any(|r| r.edge == RibbonEdge::Right)),
        top: inset(ribbons.iter().any(|r| r.edge == RibbonEdge::Top)),
        bottom: inset(ribbons.iter().any(|r| r.edge == RibbonEdge::Bottom)),
    }
}

/// The first declared ribbon is the persistent/main bar. It owns its
/// edge end-to-end; perpendicular ribbons yield their corner cells to
/// it. This keeps one stable app bar no matter what optional rails are
/// declared after it.
fn is_main_ribbon(ribbons: &[RibbonDef], def: &RibbonDef) -> bool {
    ribbons.first().is_some_and(|first| first.id == def.id)
}

fn insets_for_ribbon(ribbons: &[RibbonDef], def: &RibbonDef, base: SideInsets) -> SideInsets {
    let mut out = base;

    // The first declared ribbon is the persistent/main bar and owns
    // its edge end-to-end. Perpendicular ribbons always yield their
    // shared corner to it.
    if is_main_ribbon(ribbons, def) {
        if def.edge.is_vertical() {
            out.top = EDGE_GAP;
            out.bottom = EDGE_GAP;
        } else {
            out.left = EDGE_GAP;
            out.right = EDGE_GAP;
        }
        return out;
    }

    // For the remaining rails, corner ownership follows declaration
    // order: earlier ribbons own the shared corner, later ribbons
    // yield. This avoids the broken case where BOTH perpendicular
    // rails inset away from a corner, leaving neither rail reaching
    // the end. In the demo order (Top main, Left, Right, Bottom),
    // Left/Right yield to the Top main bar, but they own the lower
    // corners; the Bottom bar then yields to Left/Right.
    let Some(def_idx) = ribbons.iter().position(|r| r.id == def.id) else {
        return out;
    };
    let earlier = |edge: RibbonEdge| {
        ribbons
            .iter()
            .position(|r| r.edge == edge)
            .is_some_and(|idx| idx < def_idx)
    };

    match def.edge {
        RibbonEdge::Left | RibbonEdge::Right => {
            out.top = if earlier(RibbonEdge::Top) {
                base.top
            } else {
                EDGE_GAP
            };
            out.bottom = if earlier(RibbonEdge::Bottom) {
                base.bottom
            } else {
                EDGE_GAP
            };
        }
        RibbonEdge::Top | RibbonEdge::Bottom => {
            out.left = if earlier(RibbonEdge::Left) {
                base.left
            } else {
                EDGE_GAP
            };
            out.right = if earlier(RibbonEdge::Right) {
                base.right
            } else {
                EDGE_GAP
            };
        }
    }

    out
}

fn place_button(
    def: &RibbonDef,
    cluster: RibbonCluster,
    slot: u32,
    cluster_total: u32,
    insets: SideInsets,
) -> ButtonPlacement {
    let step = SIDE_BTN_SIZE + SIDE_BTN_GAP;
    let s = slot as f32;

    // Centred-cluster offset along its axis. For horizontal bars we
    // centre around the mid-point of the INSET region (between the
    // left and right rails), which shifts away from screen-centre
    // when only one rail is present.
    let centred_offset_h = || -> f32 {
        let n = cluster_total.max(1) as f32;
        let row = n * SIDE_BTN_SIZE + (n - 1.0).max(0.0) * SIDE_BTN_GAP;
        let shift = (insets.left - insets.right) * 0.5;
        shift + -(row - SIDE_BTN_SIZE) * 0.5 + s * step
    };
    let centred_offset_v = || -> f32 {
        let n = cluster_total.max(1) as f32;
        let row = n * SIDE_BTN_SIZE + (n - 1.0).max(0.0) * SIDE_BTN_GAP;
        -(row - SIDE_BTN_SIZE) * 0.5 + s * step
    };

    match (def.edge, cluster) {
        // ── Vertical rails (Left / Right) ─────────────────────────
        //
        // Side rails own their corners — first/last button sit at
        // the screen corner regardless of whether Top/Bottom rails
        // exist. Pane offsets shift around the perpendicular rail
        // separately (see `panel_anchor_offset`) so the pane
        // doesn't cover the perpendicular rail's button when it
        // opens.
        (RibbonEdge::Left, RibbonCluster::Start) => ButtonPlacement {
            anchor: egui::Align2::LEFT_TOP,
            offset: egui::vec2(EDGE_GAP, insets.top + s * step),
        },
        (RibbonEdge::Left, RibbonCluster::Middle) => ButtonPlacement {
            anchor: egui::Align2::LEFT_CENTER,
            offset: egui::vec2(EDGE_GAP, centred_offset_v()),
        },
        (RibbonEdge::Left, RibbonCluster::End) => ButtonPlacement {
            anchor: egui::Align2::LEFT_BOTTOM,
            offset: egui::vec2(EDGE_GAP, -insets.bottom - s * step),
        },
        (RibbonEdge::Right, RibbonCluster::Start) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_TOP,
            offset: egui::vec2(-EDGE_GAP, insets.top + s * step),
        },
        (RibbonEdge::Right, RibbonCluster::Middle) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_CENTER,
            offset: egui::vec2(-EDGE_GAP, centred_offset_v()),
        },
        (RibbonEdge::Right, RibbonCluster::End) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_BOTTOM,
            offset: egui::vec2(-EDGE_GAP, -insets.bottom - s * step),
        },

        // ── Horizontal bars (Top / Bottom) ────────────────────────
        //
        // X offsets use `insets.left/right` directly — the inset
        // already includes the screen-edge padding plus the side
        // rail's button + one button-gap when a side rail is
        // present, so the first button sits one normal button gap
        // away from the side rail's button (seamless corner).
        (RibbonEdge::Top, RibbonCluster::Start) => ButtonPlacement {
            anchor: egui::Align2::LEFT_TOP,
            offset: egui::vec2(insets.left + s * step, EDGE_GAP),
        },
        (RibbonEdge::Top, RibbonCluster::Middle) => ButtonPlacement {
            anchor: egui::Align2::CENTER_TOP,
            offset: egui::vec2(centred_offset_h(), EDGE_GAP),
        },
        (RibbonEdge::Top, RibbonCluster::End) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_TOP,
            offset: egui::vec2(-insets.right - s * step, EDGE_GAP),
        },
        (RibbonEdge::Bottom, RibbonCluster::Start) => ButtonPlacement {
            anchor: egui::Align2::LEFT_BOTTOM,
            offset: egui::vec2(insets.left + s * step, -EDGE_GAP),
        },
        (RibbonEdge::Bottom, RibbonCluster::Middle) => ButtonPlacement {
            anchor: egui::Align2::CENTER_BOTTOM,
            offset: egui::vec2(centred_offset_h(), -EDGE_GAP),
        },
        (RibbonEdge::Bottom, RibbonCluster::End) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_BOTTOM,
            offset: egui::vec2(-insets.right - s * step, -EDGE_GAP),
        },
    }
}

/// Panel anchor — the `egui::Align2` you hand to `floating_window`
/// for the panel a given (ribbon, cluster) owns.
pub fn panel_anchor(def: &RibbonDef, cluster: RibbonCluster) -> egui::Align2 {
    let cluster = effective_cluster(def.mode, cluster);
    match (def.edge, cluster) {
        (RibbonEdge::Left, RibbonCluster::Start) => egui::Align2::LEFT_TOP,
        (RibbonEdge::Left, RibbonCluster::Middle) => egui::Align2::LEFT_CENTER,
        (RibbonEdge::Left, RibbonCluster::End) => egui::Align2::LEFT_BOTTOM,
        (RibbonEdge::Right, RibbonCluster::Start) => egui::Align2::RIGHT_TOP,
        (RibbonEdge::Right, RibbonCluster::Middle) => egui::Align2::RIGHT_CENTER,
        (RibbonEdge::Right, RibbonCluster::End) => egui::Align2::RIGHT_BOTTOM,
        (RibbonEdge::Top, RibbonCluster::Start) => egui::Align2::LEFT_TOP,
        (RibbonEdge::Top, RibbonCluster::Middle) => egui::Align2::CENTER_TOP,
        (RibbonEdge::Top, RibbonCluster::End) => egui::Align2::RIGHT_TOP,
        (RibbonEdge::Bottom, RibbonCluster::Start) => egui::Align2::LEFT_BOTTOM,
        (RibbonEdge::Bottom, RibbonCluster::Middle) => egui::Align2::CENTER_BOTTOM,
        (RibbonEdge::Bottom, RibbonCluster::End) => egui::Align2::RIGHT_BOTTOM,
    }
}

// ─── Draw ──────────────────────────────────────────────────────────

/// What happened during a `draw_assembly` call that the caller
/// needs to react to. One entry per button press this frame.
#[derive(Clone, Copy, Debug)]
pub struct RibbonClick {
    /// Id of the clicked button.
    pub item: &'static str,
    /// Id of the ribbon it lives on.
    pub ribbon: &'static str,
    /// Role of the owning ribbon — `Panel` clicks are already
    /// dispatched to `RibbonOpen::toggle` by `draw_assembly`;
    /// `Icon` clicks are yours to handle.
    pub role: RibbonRole,
}

/// Turn a button's anchor + offset into an actual screen rect. Used
/// both for drag ghost painting and for drop-target hit-testing.
fn screen_rect(ctx: &egui::Context, p: ButtonPlacement) -> egui::Rect {
    let screen = ctx.content_rect();
    let size = egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE);
    let min = match p.anchor {
        egui::Align2::LEFT_TOP => egui::pos2(screen.min.x + p.offset.x, screen.min.y + p.offset.y),
        egui::Align2::LEFT_CENTER => egui::pos2(
            screen.min.x + p.offset.x,
            screen.center().y - size.y * 0.5 + p.offset.y,
        ),
        egui::Align2::LEFT_BOTTOM => egui::pos2(
            screen.min.x + p.offset.x,
            screen.max.y - size.y + p.offset.y,
        ),
        egui::Align2::RIGHT_TOP => egui::pos2(
            screen.max.x - size.x + p.offset.x,
            screen.min.y + p.offset.y,
        ),
        egui::Align2::RIGHT_CENTER => egui::pos2(
            screen.max.x - size.x + p.offset.x,
            screen.center().y - size.y * 0.5 + p.offset.y,
        ),
        egui::Align2::RIGHT_BOTTOM => egui::pos2(
            screen.max.x - size.x + p.offset.x,
            screen.max.y - size.y + p.offset.y,
        ),
        egui::Align2::CENTER_TOP => egui::pos2(
            screen.center().x - size.x * 0.5 + p.offset.x,
            screen.min.y + p.offset.y,
        ),
        egui::Align2::CENTER_BOTTOM => egui::pos2(
            screen.center().x - size.x * 0.5 + p.offset.x,
            screen.max.y - size.y + p.offset.y,
        ),
        egui::Align2::CENTER_CENTER => egui::pos2(
            screen.center().x - size.x * 0.5 + p.offset.x,
            screen.center().y - size.y * 0.5 + p.offset.y,
        ),
    };
    egui::Rect::from_min_size(min, size)
}

/// Temp-data key set by [`draw_assembly`] when the user presses the
/// empty part of the first/persistent ribbon. Host integrations use
/// this to begin native window dragging for decorationless windows.
fn main_bar_empty_drag_started_id() -> egui::Id {
    egui::Id::new("frost_main_bar_empty_drag_started")
}

/// Returns true for the frame where the primary pointer pressed the
/// empty strip of the first declared ribbon.
///
/// This deliberately excludes every visible ribbon button rect, so
/// dragging icons keeps doing ribbon-item drag/reorder and only
/// blank chrome starts a native window move.
#[must_use]
pub fn main_bar_empty_drag_started(ctx: &egui::Context) -> bool {
    ctx.data(|d| {
        d.get_temp::<bool>(main_bar_empty_drag_started_id())
            .unwrap_or(false)
    })
}

/// Check whether `source` ribbon is allowed to drop buttons into
/// `target` ribbon — always true within the same ribbon, otherwise
/// requires `target.accepts` to contain the source id or `"*"`.
fn accepts_drop(source: &RibbonDef, target: &RibbonDef) -> bool {
    if source.id == target.id {
        return true;
    }
    target
        .accepts
        .iter()
        .any(|&id| id == source.id || id == "*")
}

/// Clusters that a given mode exposes as drop-targets. `Centered` /
/// `OneSided` collapse to a single slot.
fn clusters_for_mode(mode: RibbonMode) -> &'static [RibbonCluster] {
    match mode {
        RibbonMode::Centered => &[RibbonCluster::Middle],
        RibbonMode::OneSided(RibbonCluster::Start) => &[RibbonCluster::Start],
        RibbonMode::OneSided(RibbonCluster::Middle) => &[RibbonCluster::Middle],
        RibbonMode::OneSided(RibbonCluster::End) => &[RibbonCluster::End],
        RibbonMode::TwoSided => &[RibbonCluster::Start, RibbonCluster::End],
        RibbonMode::ThreeSided => &[
            RibbonCluster::Start,
            RibbonCluster::Middle,
            RibbonCluster::End,
        ],
    }
}

/// Full-edge strip rectangle — the area along a ribbon's edge
/// where drops are valid. Width (or height, for horizontal bars) is
/// the `EDGE_GAP + SIDE_BTN_SIZE + EDGE_GAP` the rail occupies.
fn ribbon_strip_rect(def: &RibbonDef, ctx: &egui::Context, insets: SideInsets) -> egui::Rect {
    let screen = ctx.content_rect();
    let thickness = EDGE_GAP * 2.0 + SIDE_BTN_SIZE;
    match def.edge {
        // Side rails own their corner buttons — drop strips run
        // the full screen height so drops near the corner still
        // resolve to the correct rail.
        RibbonEdge::Left => egui::Rect::from_min_max(
            screen.min,
            egui::pos2(screen.min.x + thickness, screen.max.y),
        ),
        RibbonEdge::Right => egui::Rect::from_min_max(
            egui::pos2(screen.max.x - thickness, screen.min.y),
            screen.max,
        ),
        // Horizontal bars are trimmed by the side-rail insets so
        // they don't claim corner cells the side rails occupy.
        RibbonEdge::Top => egui::Rect::from_min_max(
            egui::pos2(screen.min.x + insets.left, screen.min.y),
            egui::pos2(screen.max.x - insets.right, screen.min.y + thickness),
        ),
        RibbonEdge::Bottom => egui::Rect::from_min_max(
            egui::pos2(screen.min.x + insets.left, screen.max.y - thickness),
            egui::pos2(screen.max.x - insets.right, screen.max.y),
        ),
    }
}

/// Drop region for a specific cluster of a ribbon — a sub-rect of
/// [`ribbon_strip_rect`]. `Centered` / `OneSided` return the whole
/// strip; `TwoSided` splits in halves along the edge axis;
/// `ThreeSided` splits in thirds.
fn cluster_region(
    def: &RibbonDef,
    cluster: RibbonCluster,
    ctx: &egui::Context,
    insets: SideInsets,
) -> egui::Rect {
    let strip = ribbon_strip_rect(def, ctx, insets);
    let cluster = effective_cluster(def.mode, cluster);

    match def.mode {
        RibbonMode::Centered | RibbonMode::OneSided(_) => strip,
        RibbonMode::TwoSided => {
            if def.edge.is_vertical() {
                let mid = strip.center().y;
                match cluster {
                    RibbonCluster::Start => {
                        egui::Rect::from_min_max(strip.min, egui::pos2(strip.max.x, mid))
                    }
                    _ => egui::Rect::from_min_max(egui::pos2(strip.min.x, mid), strip.max),
                }
            } else {
                let mid = strip.center().x;
                match cluster {
                    RibbonCluster::Start => {
                        egui::Rect::from_min_max(strip.min, egui::pos2(mid, strip.max.y))
                    }
                    _ => egui::Rect::from_min_max(egui::pos2(mid, strip.min.y), strip.max),
                }
            }
        }
        RibbonMode::ThreeSided => {
            if def.edge.is_vertical() {
                let h = strip.height() / 3.0;
                let t1 = strip.min.y + h;
                let t2 = strip.min.y + h * 2.0;
                match cluster {
                    RibbonCluster::Start => {
                        egui::Rect::from_min_max(strip.min, egui::pos2(strip.max.x, t1))
                    }
                    RibbonCluster::Middle => egui::Rect::from_min_max(
                        egui::pos2(strip.min.x, t1),
                        egui::pos2(strip.max.x, t2),
                    ),
                    RibbonCluster::End => {
                        egui::Rect::from_min_max(egui::pos2(strip.min.x, t2), strip.max)
                    }
                }
            } else {
                let w = strip.width() / 3.0;
                let t1 = strip.min.x + w;
                let t2 = strip.min.x + w * 2.0;
                match cluster {
                    RibbonCluster::Start => {
                        egui::Rect::from_min_max(strip.min, egui::pos2(t1, strip.max.y))
                    }
                    RibbonCluster::Middle => egui::Rect::from_min_max(
                        egui::pos2(t1, strip.min.y),
                        egui::pos2(t2, strip.max.y),
                    ),
                    RibbonCluster::End => {
                        egui::Rect::from_min_max(egui::pos2(t2, strip.min.y), strip.max)
                    }
                }
            }
        }
    }
}

/// Draw every ribbon + every button in the assembly. Runs panel
/// exclusivity for `Panel` ribbons (click toggles `RibbonOpen`),
/// handles drag-to-swap routing across ribbons (gated by the
/// `accepts` list), and returns a list of every click this frame so
/// the caller can handle `Icon` actions.
pub fn draw_assembly(
    ctx: &egui::Context,
    accent: egui::Color32,
    ribbons: &[RibbonDef],
    items: &[RibbonItem],
    open: &mut RibbonOpen,
    placement: &mut RibbonPlacement,
    drag: &mut RibbonDrag,
    active: impl Fn(&'static str) -> bool,
) -> Vec<RibbonClick> {
    let insets = compute_side_insets(ribbons);
    // Publish which screen edges currently host a ribbon. `Pane2`
    // reads this to size itself against actual ribbon presence —
    // a top-rail pane on a screen with no bottom ribbon can extend
    // all the way to the bottom edge instead of reserving
    // `RAIL_INSET` for a phantom ribbon. See
    // `frost_core::pane::published_ribbon_edges`.
    let presence = [
        ribbons.iter().any(|r| r.edge == RibbonEdge::Left),
        ribbons.iter().any(|r| r.edge == RibbonEdge::Right),
        ribbons.iter().any(|r| r.edge == RibbonEdge::Top),
        ribbons.iter().any(|r| r.edge == RibbonEdge::Bottom),
    ];
    ctx.data_mut(|d| {
        d.insert_temp::<[bool; 4]>(egui::Id::new("frost_published_ribbon_edges"), presence);
    });

    // ── Resolve baseline positions ─────────────────────────────────
    // Each item has a current (ribbon, cluster_raw, slot). The
    // cluster_effective falls out of that via ribbon.mode.
    let resolved: Vec<(&'static str, RibbonCluster, u32)> =
        items.iter().map(|i| placement.resolve(i)).collect();

    // ── Compute drop target + insertion index (only while dragging)
    //
    // The pattern is exactly the old `RibbonLayout`'s: pure function
    // of (cursor, drag source, current placements). No pre-baked
    // display vector; each item's reflow is computed on the fly when
    // we paint it.
    let mut target: Option<(&'static str, RibbonCluster, u32)> = None; // (ribbon, cluster_eff, insertion)
    if let (Some(dragged_id), Some(cursor), Some(source)) = (drag.item, drag.cursor, drag.source) {
        let src_idx = items.iter().position(|i| i.id == dragged_id);
        let src_def = src_idx
            .map(|i| resolved[i].0)
            .and_then(|rid| ribbons.iter().find(|d| d.id == rid));
        if let Some(src_def) = src_def {
            // Which (ribbon, cluster) region is the cursor in?
            let mut hit: Option<(&'static str, RibbonCluster)> = None;
            'outer: for def in ribbons {
                if !accepts_drop(src_def, def) {
                    continue;
                }
                for &cluster in clusters_for_mode(def.mode) {
                    if cluster_region(def, cluster, ctx, insets_for_ribbon(ribbons, def, insets))
                        .contains(cursor)
                    {
                        hit = Some((def.id, cluster));
                        break 'outer;
                    }
                }
            }
            if let Some((tgt_rid, tgt_cluster_raw)) = hit {
                let tgt_def = ribbons
                    .iter()
                    .find(|d| d.id == tgt_rid)
                    .expect("target ribbon from same slice");
                let tgt_cluster_eff = effective_cluster(tgt_def.mode, tgt_cluster_raw);
                // Target occupant count (excluding the dragged item).
                let siblings = items
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| resolved[*i].0 == tgt_rid)
                    .filter(|(i, _)| {
                        let (_, c, _) = resolved[*i];
                        effective_cluster(tgt_def.mode, c) == tgt_cluster_eff
                    })
                    .filter(|(_, it)| it.id != dragged_id)
                    .count() as u32;
                let total_with_ghost = siblings + 1;

                // Find the slot whose centre is closest to the
                // cursor along the edge axis. That's the insertion.
                let axis_is_y = tgt_def.edge.is_vertical();
                let cursor_axis = if axis_is_y { cursor.y } else { cursor.x };
                let mut best_slot = 0u32;
                let mut best_d = f32::INFINITY;
                for slot in 0..total_with_ghost {
                    let p = place_button(
                        tgt_def,
                        tgt_cluster_eff,
                        slot,
                        total_with_ghost,
                        insets_for_ribbon(ribbons, tgt_def, insets),
                    );
                    let rect = screen_rect(ctx, p);
                    let c = if axis_is_y {
                        rect.center().y
                    } else {
                        rect.center().x
                    };
                    let d = (c - cursor_axis).abs();
                    if d < best_d {
                        best_d = d;
                        best_slot = slot;
                    }
                }
                target = Some((tgt_rid, tgt_cluster_eff, best_slot));
                let _ = source; // currently unused; retained for future no-op detection
            }
        }
    }

    // ── Pure function: each item's effective visual (cluster, slot, total)
    //
    // Mirrors the old `RibbonLayout::effective_visual` — non-dragged
    // items shift to close the source gap and open the target gap.
    let effective = |item_idx: usize| -> (&'static str, RibbonCluster, u32, u32) {
        let (rid, c, slot) = resolved[item_idx];
        let Some(def) = ribbons.iter().find(|d| d.id == rid) else {
            return (rid, c, slot, 1);
        };
        let kind = (rid, effective_cluster(def.mode, c));
        let raw_total = |rkind: (&'static str, RibbonCluster)| -> u32 {
            items
                .iter()
                .enumerate()
                .filter(|(i, _)| resolved[*i].0 == rkind.0)
                .filter(|(i, _)| {
                    let (_, c, _) = resolved[*i];
                    let Some(d) = ribbons.iter().find(|d| d.id == rkind.0) else {
                        return false;
                    };
                    effective_cluster(d.mode, c) == rkind.1
                })
                .count() as u32
        };
        // Source compaction applies as SOON as a drag is in flight —
        // the moment you grab a button its home slot vacates and
        // siblings slide up, regardless of whether the cursor is
        // over any drop zone. Target insertion stacks on top when
        // the cursor IS over a zone.
        let Some(source) = drag.source else {
            return (kind.0, kind.1, slot, raw_total(kind));
        };
        let (src_rid, src_cluster_raw, src_slot) = source;
        let Some(src_def) = ribbons.iter().find(|d| d.id == src_rid) else {
            return (kind.0, kind.1, slot, raw_total(kind));
        };
        let src_kind = (src_rid, effective_cluster(src_def.mode, src_cluster_raw));

        let mut out_slot = slot;
        let mut total_delta: i32 = 0;

        if let Some((tgt_rid, tgt_cluster_eff, insert)) = target {
            let tgt_kind = (tgt_rid, tgt_cluster_eff);
            if kind == src_kind && kind == tgt_kind {
                // Same-cluster reorder — net count unchanged.
                if src_slot < insert && slot > src_slot && slot <= insert {
                    out_slot = slot - 1;
                } else if src_slot > insert && slot >= insert && slot < src_slot {
                    out_slot = slot + 1;
                }
            } else {
                // Cross cluster.
                if kind == src_kind && slot > src_slot {
                    out_slot = slot - 1;
                }
                if kind == tgt_kind && slot >= insert {
                    out_slot = slot + 1;
                }
                if kind == src_kind {
                    total_delta = -1;
                } else if kind == tgt_kind {
                    total_delta = 1;
                }
            }
        } else {
            // No target yet — just compact the source cluster.
            if kind == src_kind && slot > src_slot {
                out_slot = slot - 1;
            }
            if kind == src_kind {
                total_delta = -1;
            }
        }

        let base = raw_total(kind) as i32 + total_delta;
        let total = base.max(1) as u32;

        (kind.0, kind.1, out_slot, total)
    };

    // ── Render pass ────────────────────────────────────────────────
    let mut click_flags: Vec<bool> = vec![false; items.len()];
    let mut drag_started_idx: Option<usize> = None;
    let mut drag_stopped_this_frame = false;
    let mut button_rects: Vec<egui::Rect> = Vec::with_capacity(items.len());

    for (idx, item) in items.iter().enumerate() {
        let (rid, cluster_eff, slot_eff, total) = effective(idx);
        let Some(def) = ribbons.iter().find(|d| d.id == rid) else {
            continue;
        };
        let is_dragging_this = drag.item == Some(item.id);
        // Panel role reads from `RibbonOpen`; Icon role reads from
        // the caller-supplied `active` closure. Both may OR with
        // each other so a caller can also tint a Panel button
        // active for reasons outside the menu-open state.
        let item_role = item.effective_role(def);
        let is_active = match item_role {
            RibbonRole::Panel => open.is_open(def.id, item.id) || active(item.id),
            RibbonRole::Icon => active(item.id),
        };

        // Resting rect (where the button would sit if released now).
        let resting_p = place_button(
            def,
            cluster_eff,
            slot_eff,
            total,
            insets_for_ribbon(ribbons, def, insets),
        );
        let resting_rect = screen_rect(ctx, resting_p);
        button_rects.push(resting_rect);

        // Paint rect — resting for most, at the cursor (as the
        // ghost) for the dragged button.
        let (paint_pos, order) = if is_dragging_this {
            let c = drag.cursor.unwrap_or_else(|| resting_rect.center());
            (
                egui::pos2(c.x - SIDE_BTN_SIZE * 0.5, c.y - SIDE_BTN_SIZE * 0.5),
                egui::Order::Tooltip,
            )
        } else {
            // `Order::Foreground` (was `Middle`): ribbon buttons
            // sit above the GAME-theme floating section icons
            // (`z::CONTAINER_FLOATING_ICON`, Middle) AND above the
            // `embed` fullscreen overlay (`Order::Foreground` too,
            // but the host paints the ribbon assembly AFTER the
            // pane loop, so registration order lands ribbons on
            // top within the same Foreground tier).
            (resting_rect.min, egui::Order::Foreground)
        };

        let area_id = egui::Id::new(("frost_assembly_btn", item.id));
        let area_response = egui::Area::new(area_id)
            .order(order)
            .fixed_pos(paint_pos)
            .interactable(true)
            .show(ctx, |ui| {
                let sense = if def.draggable {
                    egui::Sense::click_and_drag()
                } else {
                    egui::Sense::click()
                };
                let (rect, r) =
                    ui.allocate_exact_size(egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE), sense);
                paint_ribbon_button(
                    ui.painter(),
                    rect,
                    accent,
                    is_active,
                    r.hovered() || is_dragging_this,
                );
                let fg = ribbon_button_fg(
                    accent,
                    is_active || is_dragging_this,
                    r.hovered() || is_dragging_this,
                    item.glyph,
                );
                paint_ribbon_glyph(ui, rect, item.glyph, fg);
                r.on_hover_text(item.tooltip)
            });
        // Force the ribbon button's layer to the top of its Order
        // every frame. Without this, clicking inside the `embed`
        // fullscreen overlay (same `Order::Foreground`) brings the
        // overlay to the top of the order, hiding ribbon icons
        // until the next paint that re-orders them. Calling
        // `move_to_top` on every ribbon Area keeps them on top
        // even after the overlay claims focus.
        ctx.move_to_top(area_response.response.layer_id);
        let resp = area_response.inner;

        if def.draggable && resp.drag_started() {
            drag_started_idx = Some(idx);
        }
        if is_dragging_this && resp.dragged() {
            if let Some(pos) = ctx.pointer_interact_pos() {
                drag.cursor = Some(pos);
            }
        }
        if is_dragging_this && resp.drag_stopped() {
            drag_stopped_this_frame = true;
        }
        // Click fires only on genuine clicks — never the release
        // that ends a drag (drag_stopped + clicked both fire on the
        // same release).
        if resp.clicked() && drag.item.is_none() && !is_dragging_this {
            click_flags[idx] = true;
        }
    }

    // ── Insertion-slot outline (preview of where ghost lands) ──────
    //
    // A faint accent-tinted rect drawn at the target cluster's
    // insertion slot, so the user sees the landing spot separately
    // from the button they're dragging (which floats at the cursor).
    if let (Some(_dragged_id), Some((tgt_rid, tgt_cluster_eff, insert))) = (drag.item, target) {
        if let Some(tgt_def) = ribbons.iter().find(|d| d.id == tgt_rid) {
            // Recompute the target occupant count so the outline sits
            // exactly where a drop would land.
            let siblings = items
                .iter()
                .enumerate()
                .filter(|(i, _)| resolved[*i].0 == tgt_rid)
                .filter(|(i, _)| {
                    let (_, c, _) = resolved[*i];
                    effective_cluster(tgt_def.mode, c) == tgt_cluster_eff
                })
                .filter(|(_, it)| drag.item != Some(it.id))
                .count() as u32;
            let total_with_ghost = siblings + 1;
            let p = place_button(
                tgt_def,
                tgt_cluster_eff,
                insert,
                total_with_ghost,
                insets_for_ribbon(ribbons, tgt_def, insets),
            );
            let rect = screen_rect(ctx, p);
            let area_id = egui::Id::new("frost_assembly_drop_outline");
            egui::Area::new(area_id)
                .order(egui::Order::Foreground)
                .fixed_pos(rect.min)
                .interactable(false)
                .show(ctx, |ui| {
                    let (r, _) = ui.allocate_exact_size(
                        egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect(
                        r,
                        crate::style::radius_for(crate::style::RadiusRole::Section),
                        crate::style::fill_for(crate::style::FillRole::DragGhost, accent),
                        crate::style::stroke_for(crate::style::StrokeRole::DragGhost, accent),
                        egui::StrokeKind::Inside,
                    );
                });
        }
    }

    // ── Commit drag start ──────────────────────────────────────────
    if let Some(idx) = drag_started_idx {
        drag.item = Some(items[idx].id);
        drag.cursor = ctx.pointer_interact_pos();
        drag.source = Some(resolved[idx]);
    }

    // ── Commit drag release / drop ─────────────────────────────────
    if drag_stopped_this_frame {
        if let (Some(dragged_id), Some((tgt_rid, tgt_cluster_eff, insert))) = (drag.item, target) {
            if let Some(src) = drag.source {
                resolve_drop(
                    placement,
                    ribbons,
                    items,
                    dragged_id,
                    src,
                    tgt_rid,
                    tgt_cluster_eff,
                    insert,
                );
            }
        }
        drag.item = None;
        drag.cursor = None;
        drag.source = None;
    }

    // ── Empty main-bar drag detection ──────────────────────────────
    //
    // Decorationless hosts still need a native-move hit zone. The
    // first ribbon is Frost's persistent main bar; any primary press
    // inside that strip but outside visible buttons is published so
    // host crates can call their native `start_drag_move` equivalent.
    let empty_main_bar_drag_started = ribbons.first().is_some_and(|main| {
        let main_strip = ribbon_strip_rect(main, ctx, insets_for_ribbon(ribbons, main, insets));
        ctx.input(|i| {
            i.pointer.interact_pos().is_some_and(|pos| {
                i.pointer.button_pressed(egui::PointerButton::Primary)
                    && main_strip.contains(pos)
                    && !button_rects.iter().any(|rect| rect.contains(pos))
            })
        })
    });
    ctx.data_mut(|d| {
        d.insert_temp::<bool>(
            main_bar_empty_drag_started_id(),
            empty_main_bar_drag_started,
        );
    });

    // ── Click dispatch ─────────────────────────────────────────────
    let mut clicks: Vec<RibbonClick> = Vec::new();
    for (idx, &fired) in click_flags.iter().enumerate() {
        if !fired {
            continue;
        }
        let item = &items[idx];
        let rid = resolved[idx].0;
        let Some(def) = ribbons.iter().find(|d| d.id == rid) else {
            continue;
        };
        let click_role = item.effective_role(def);
        match click_role {
            RibbonRole::Panel => open.toggle(def.id, item.id),
            RibbonRole::Icon => {}
        }
        clicks.push(RibbonClick {
            item: item.id,
            ribbon: def.id,
            role: click_role,
        });
    }

    clicks
}

/// Mutate `placement` so the dragged button lands at (tgt, insert)
/// and every other item's slot shifts to close the source gap +
/// open the target gap. Finally, compact every cluster 0..n.
#[allow(clippy::too_many_arguments)]
fn resolve_drop(
    placement: &mut RibbonPlacement,
    ribbons: &[RibbonDef],
    items: &[RibbonItem],
    dragged_id: &'static str,
    source: (&'static str, RibbonCluster, u32),
    tgt_rid: &'static str,
    tgt_cluster_eff: RibbonCluster,
    insert: u32,
) {
    let (src_rid, src_cluster_raw, src_slot) = source;
    let Some(src_def) = ribbons.iter().find(|d| d.id == src_rid) else {
        return;
    };
    let src_cluster_eff = effective_cluster(src_def.mode, src_cluster_raw);

    // Resolve current positions once so we don't read our own writes.
    let now: Vec<(&'static str, (&'static str, RibbonCluster, u32))> = items
        .iter()
        .map(|it| (it.id, placement.resolve(it)))
        .collect();

    // Source compaction (close hole) — skip if same-cluster reorder,
    // that's handled by the same-kind branch below.
    let cross_cluster = (src_rid, src_cluster_eff) != (tgt_rid, tgt_cluster_eff);

    for (id, (rid, c_raw, slot)) in &now {
        if *id == dragged_id {
            continue;
        }
        let Some(def) = ribbons.iter().find(|d| d.id == *rid) else {
            continue;
        };
        let c_eff = effective_cluster(def.mode, *c_raw);
        let mut new_slot = *slot;
        if cross_cluster {
            if *rid == src_rid && c_eff == src_cluster_eff && *slot > src_slot {
                new_slot -= 1;
            }
            if *rid == tgt_rid && c_eff == tgt_cluster_eff && *slot >= insert {
                new_slot += 1;
            }
        } else {
            // Same-cluster reorder.
            if src_slot < insert && *slot > src_slot && *slot <= insert {
                new_slot -= 1;
            } else if src_slot > insert && *slot >= insert && *slot < src_slot {
                new_slot += 1;
            }
        }
        placement.overrides.insert(*id, (*rid, *c_raw, new_slot));
    }

    // Dragged button lands at (tgt_rid, tgt_cluster_eff, insert).
    // We stash the raw cluster the same as the target's (effective
    // is raw for anything other than Centered).
    placement
        .overrides
        .insert(dragged_id, (tgt_rid, tgt_cluster_eff, insert));

    // Re-compact every cluster so slots stay contiguous.
    for def in ribbons {
        for &cluster in clusters_for_mode(def.mode) {
            let c_eff = effective_cluster(def.mode, cluster);
            let mut occ: Vec<(&'static str, u32)> = items
                .iter()
                .filter_map(|it| {
                    let (r, c, s) = placement.resolve(it);
                    if r != def.id {
                        return None;
                    }
                    let d = ribbons.iter().find(|d| d.id == r)?;
                    if effective_cluster(d.mode, c) != c_eff {
                        return None;
                    }
                    Some((it.id, s))
                })
                .collect();
            occ.sort_by_key(|(_, s)| *s);
            for (n, (id, _)) in occ.into_iter().enumerate() {
                let Some(item) = items.iter().find(|i| i.id == id) else {
                    continue;
                };
                let (r, c_raw, _) = placement.resolve(item);
                placement.overrides.insert(id, (r, c_raw, n as u32));
            }
        }
    }
}

/// Build the persistent-width storage id for a `(ribbon, cluster)`.
/// Use this to scope [`crate::floating::floating_window_scoped`] so
/// each cluster's panels keep their own width.
pub fn cluster_width_scope(ribbon: &'static str, cluster: RibbonCluster) -> egui::Id {
    egui::Id::new("frost_cluster_width")
        .with(ribbon)
        .with(cluster)
}

// `floating_window_for_item` lived here in `frostcore::ribbon::assembly`,
// dispatching into `frostcore::floating`'s old pane builder. In
// `frost_core` the new pane (`crate::pane::Pane2`) replaces that path;
// callers open panes directly with `Pane2::new(...).show(ctx, body)`
// guarded by `RibbonOpen::is_open`. The helper isn't ported.

/// Convenience: find a button's definition by id.
pub fn find_item<'a>(items: &'a [RibbonItem], id: &'static str) -> Option<&'a RibbonItem> {
    items.iter().find(|i| i.id == id)
}

/// Convenience: find a ribbon's definition by id.
pub fn find_ribbon<'a>(ribbons: &'a [RibbonDef], id: &'static str) -> Option<&'a RibbonDef> {
    ribbons.iter().find(|r| r.id == id)
}

/// Panel anchor for a specific button — combines its ribbon lookup
/// with [`panel_anchor`] so callers have one call.
pub fn panel_anchor_for_item(
    ribbons: &[RibbonDef],
    items: &[RibbonItem],
    item_id: &'static str,
) -> Option<egui::Align2> {
    let item = find_item(items, item_id)?;
    let def = find_ribbon(ribbons, item.ribbon)?;
    Some(panel_anchor(def, item.cluster))
}

/// Anchor-offset vector that pushes a pane AWAY from the ribbon's
/// edge so the pane sits adjacent to (not on top of) the button
/// that opened it.
///
/// On Left / Right rails the offset slides the pane horizontally
/// off the rail (`±side_inset`) plus a small `EDGE_GAP` on the
/// opposite axis. On Top / Bottom rails the offset slides the
/// pane vertically off the rail.
///
/// Top / Bottom rails are *trimmed* by the Left / Right rail's
/// thickness (their corners belong to the side rails, see
/// [`ribbon_strip_rect`]). The pane mirrors that trim: a Top
/// rail Start-cluster pane is shifted right by `insets.left` so
/// its left edge lines up with the first button on the rail, not
/// the screen corner the LEFT rail already owns. End clusters
/// get the same shift on the opposite side; Middle clusters
/// recentre by `(insets.left - insets.right) / 2`.
///
/// `side_inset` is the rail thickness — `EDGE_GAP +
/// SIDE_BTN_SIZE + RAIL_PANEL_GAP` — so the pane lands `RAIL_PANEL_GAP`
/// below (or above / right of / left of) the button row.
pub fn panel_anchor_offset(
    def: &RibbonDef,
    cluster: RibbonCluster,
    insets: SideInsets,
) -> egui::Vec2 {
    // Match `floating::RAIL_PANEL_GAP` + the rail thickness math
    // in `floating_window_scoped`. Kept in sync by the asserts
    // there. Equals `SIDE_BTN_GAP` so panes from perpendicular
    // rails meet at the exact same corner pixel.
    const RAIL_PANEL_GAP: f32 = 4.0;
    let side_inset = EDGE_GAP + SIDE_BTN_SIZE + RAIL_PANEL_GAP;
    // Trim shifts. Start hugs the first button's coordinate (the
    // inset already encodes screen padding + perpendicular-rail
    // corner), End mirrors on the opposite side, Middle splits the
    // asymmetry between the two perpendicular rails.
    let h_start = insets.left;
    let h_end = -insets.right;
    let h_mid = (insets.left - insets.right) * 0.5;
    let v_start = insets.top;
    let v_end = -insets.bottom;
    let v_mid = (insets.top - insets.bottom) * 0.5;
    match (def.edge, cluster) {
        // LEFT rail — pane slides RIGHT off the rail.
        (RibbonEdge::Left, RibbonCluster::Start) => egui::vec2(side_inset, v_start),
        (RibbonEdge::Left, RibbonCluster::Middle) => egui::vec2(side_inset, v_mid),
        (RibbonEdge::Left, RibbonCluster::End) => egui::vec2(side_inset, v_end),
        // RIGHT rail — pane slides LEFT off the rail.
        (RibbonEdge::Right, RibbonCluster::Start) => egui::vec2(-side_inset, v_start),
        (RibbonEdge::Right, RibbonCluster::Middle) => egui::vec2(-side_inset, v_mid),
        (RibbonEdge::Right, RibbonCluster::End) => egui::vec2(-side_inset, v_end),
        // TOP rail — pane slides DOWN off the rail.
        (RibbonEdge::Top, RibbonCluster::Start) => egui::vec2(h_start, side_inset),
        (RibbonEdge::Top, RibbonCluster::Middle) => egui::vec2(h_mid, side_inset),
        (RibbonEdge::Top, RibbonCluster::End) => egui::vec2(h_end, side_inset),
        // BOTTOM rail — pane slides UP off the rail.
        (RibbonEdge::Bottom, RibbonCluster::Start) => egui::vec2(h_start, -side_inset),
        (RibbonEdge::Bottom, RibbonCluster::Middle) => egui::vec2(h_mid, -side_inset),
        (RibbonEdge::Bottom, RibbonCluster::End) => egui::vec2(h_end, -side_inset),
    }
}
