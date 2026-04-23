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

use bevy::prelude::*;
use bevy::platform::collections::HashMap;
use bevy_egui::egui;

use super::paint::{paint_ribbon_button, EDGE_GAP, SIDE_BTN_GAP, SIDE_BTN_SIZE};
use crate::style::{TEXT_PRIMARY, TEXT_SECONDARY};

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

/// Declaration of one button that lives inside a ribbon.
#[derive(Clone, Debug)]
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
    /// Single-glyph label.
    pub glyph: &'static str,
    /// Hover tooltip.
    pub tooltip: &'static str,
    /// If set, this icon-ribbon button pops a nested ribbon on press.
    /// Phase-1 stub — carried on the struct so callers can already
    /// wire the data; renderer lands with phase 2.
    pub child_ribbon: Option<&'static str>,
}

// ─── State resources ────────────────────────────────────────────────

/// Per-ribbon exclusive slot — the id of the button whose panel is
/// currently open on that ribbon, if any. Key is the ribbon id.
#[derive(Resource, Default, Debug, Clone)]
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
}

/// Panel widths keyed by `(ribbon_id, cluster)`. Widths persist
/// across frames so a cluster's panels remember the user's drag.
#[derive(Resource, Default, Debug, Clone)]
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
#[derive(Resource, Default, Debug, Clone)]
pub struct RibbonPlacement {
    pub overrides: HashMap<&'static str, (&'static str, RibbonCluster, u32)>,
}

impl RibbonPlacement {
    /// Resolved position for `item`, folding in any user drag.
    pub fn resolve(
        &self,
        item: &RibbonItem,
    ) -> (&'static str, RibbonCluster, u32) {
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
#[derive(Resource, Default, Debug, Clone)]
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
/// Pixels reserved along each edge by the side rails, used to inset
/// the top / bottom ribbons so the corners don't overlap. Vertical
/// rails are considered "ownership priority" — they claim full
/// screen height first; horizontal bars render *between* them.
#[derive(Clone, Copy, Debug, Default)]
pub struct SideInsets {
    pub left: f32,
    pub right: f32,
}

fn compute_side_insets(ribbons: &[RibbonDef]) -> SideInsets {
    let thickness = EDGE_GAP * 2.0 + SIDE_BTN_SIZE;
    SideInsets {
        left: if ribbons.iter().any(|r| r.edge == RibbonEdge::Left) {
            thickness
        } else {
            0.0
        },
        right: if ribbons.iter().any(|r| r.edge == RibbonEdge::Right) {
            thickness
        } else {
            0.0
        },
    }
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
        (RibbonEdge::Left, RibbonCluster::Start) => ButtonPlacement {
            anchor: egui::Align2::LEFT_TOP,
            offset: egui::vec2(EDGE_GAP, EDGE_GAP + s * step),
        },
        (RibbonEdge::Left, RibbonCluster::Middle) => ButtonPlacement {
            anchor: egui::Align2::LEFT_CENTER,
            offset: egui::vec2(EDGE_GAP, centred_offset_v()),
        },
        (RibbonEdge::Left, RibbonCluster::End) => ButtonPlacement {
            anchor: egui::Align2::LEFT_BOTTOM,
            offset: egui::vec2(EDGE_GAP, -EDGE_GAP - s * step),
        },
        (RibbonEdge::Right, RibbonCluster::Start) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_TOP,
            offset: egui::vec2(-EDGE_GAP, EDGE_GAP + s * step),
        },
        (RibbonEdge::Right, RibbonCluster::Middle) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_CENTER,
            offset: egui::vec2(-EDGE_GAP, centred_offset_v()),
        },
        (RibbonEdge::Right, RibbonCluster::End) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_BOTTOM,
            offset: egui::vec2(-EDGE_GAP, -EDGE_GAP - s * step),
        },

        // ── Horizontal bars (Top / Bottom) ────────────────────────
        //
        // Offsets along the X axis include the side-rail insets so
        // Top/Bottom ribbons stop at the vertical rails instead of
        // overlapping into them.
        (RibbonEdge::Top, RibbonCluster::Start) => ButtonPlacement {
            anchor: egui::Align2::LEFT_TOP,
            offset: egui::vec2(insets.left + EDGE_GAP + s * step, EDGE_GAP),
        },
        (RibbonEdge::Top, RibbonCluster::Middle) => ButtonPlacement {
            anchor: egui::Align2::CENTER_TOP,
            offset: egui::vec2(centred_offset_h(), EDGE_GAP),
        },
        (RibbonEdge::Top, RibbonCluster::End) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_TOP,
            offset: egui::vec2(-insets.right - EDGE_GAP - s * step, EDGE_GAP),
        },
        (RibbonEdge::Bottom, RibbonCluster::Start) => ButtonPlacement {
            anchor: egui::Align2::LEFT_BOTTOM,
            offset: egui::vec2(insets.left + EDGE_GAP + s * step, -EDGE_GAP),
        },
        (RibbonEdge::Bottom, RibbonCluster::Middle) => ButtonPlacement {
            anchor: egui::Align2::CENTER_BOTTOM,
            offset: egui::vec2(centred_offset_h(), -EDGE_GAP),
        },
        (RibbonEdge::Bottom, RibbonCluster::End) => ButtonPlacement {
            anchor: egui::Align2::RIGHT_BOTTOM,
            offset: egui::vec2(-insets.right - EDGE_GAP - s * step, -EDGE_GAP),
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

/// Check whether `source` ribbon is allowed to drop buttons into
/// `target` ribbon — always true within the same ribbon, otherwise
/// requires `target.accepts` to contain the source id or `"*"`.
fn accepts_drop(source: &RibbonDef, target: &RibbonDef) -> bool {
    if source.id == target.id {
        return true;
    }
    target.accepts.iter().any(|&id| id == source.id || id == "*")
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
        RibbonEdge::Left => egui::Rect::from_min_max(
            screen.min,
            egui::pos2(screen.min.x + thickness, screen.max.y),
        ),
        RibbonEdge::Right => egui::Rect::from_min_max(
            egui::pos2(screen.max.x - thickness, screen.min.y),
            screen.max,
        ),
        // Horizontal bars are trimmed by the side-rail insets so the
        // corners don't overlap.
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
                    RibbonCluster::Start => egui::Rect::from_min_max(
                        strip.min,
                        egui::pos2(strip.max.x, t1),
                    ),
                    RibbonCluster::Middle => egui::Rect::from_min_max(
                        egui::pos2(strip.min.x, t1),
                        egui::pos2(strip.max.x, t2),
                    ),
                    RibbonCluster::End => egui::Rect::from_min_max(
                        egui::pos2(strip.min.x, t2),
                        strip.max,
                    ),
                }
            } else {
                let w = strip.width() / 3.0;
                let t1 = strip.min.x + w;
                let t2 = strip.min.x + w * 2.0;
                match cluster {
                    RibbonCluster::Start => egui::Rect::from_min_max(
                        strip.min,
                        egui::pos2(t1, strip.max.y),
                    ),
                    RibbonCluster::Middle => egui::Rect::from_min_max(
                        egui::pos2(t1, strip.min.y),
                        egui::pos2(t2, strip.max.y),
                    ),
                    RibbonCluster::End => egui::Rect::from_min_max(
                        egui::pos2(t2, strip.min.y),
                        strip.max,
                    ),
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
) -> Vec<RibbonClick> {
    let insets = compute_side_insets(ribbons);

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
                    if cluster_region(def, cluster, ctx, insets).contains(cursor) {
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
                    let p = place_button(tgt_def, tgt_cluster_eff, slot, total_with_ghost, insets);
                    let rect = screen_rect(ctx, p);
                    let c = if axis_is_y { rect.center().y } else { rect.center().x };
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

    for (idx, item) in items.iter().enumerate() {
        let (rid, cluster_eff, slot_eff, total) = effective(idx);
        let Some(def) = ribbons.iter().find(|d| d.id == rid) else {
            continue;
        };
        let is_dragging_this = drag.item == Some(item.id);
        let is_active = match def.role {
            RibbonRole::Panel => open.is_open(def.id, item.id),
            RibbonRole::Icon => false,
        };

        // Resting rect (where the button would sit if released now).
        let resting_p = place_button(def, cluster_eff, slot_eff, total, insets);
        let resting_rect = screen_rect(ctx, resting_p);

        // Paint rect — resting for most, at the cursor (as the
        // ghost) for the dragged button.
        let (paint_pos, order) = if is_dragging_this {
            let c = drag.cursor.unwrap_or_else(|| resting_rect.center());
            (
                egui::pos2(c.x - SIDE_BTN_SIZE * 0.5, c.y - SIDE_BTN_SIZE * 0.5),
                egui::Order::Tooltip,
            )
        } else {
            (resting_rect.min, egui::Order::Middle)
        };

        let area_id = egui::Id::new(("frost_assembly_btn", item.id));
        let resp = egui::Area::new(area_id)
            .order(order)
            .fixed_pos(paint_pos)
            .interactable(true)
            .show(ctx, |ui| {
                let sense = if def.draggable {
                    egui::Sense::click_and_drag()
                } else {
                    egui::Sense::click()
                };
                let (rect, r) = ui.allocate_exact_size(
                    egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
                    sense,
                );
                paint_ribbon_button(
                    ui.painter(),
                    rect,
                    accent,
                    is_active,
                    r.hovered() || is_dragging_this,
                );
                let fg = if is_active || is_dragging_this {
                    TEXT_PRIMARY
                } else {
                    TEXT_SECONDARY
                };
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    item.glyph,
                    egui::FontId::new(14.0, egui::FontFamily::Monospace),
                    fg,
                );
                r.on_hover_text(item.tooltip)
            })
            .inner;

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
    if let (Some(_dragged_id), Some((tgt_rid, tgt_cluster_eff, insert))) =
        (drag.item, target)
    {
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
            let p = place_button(tgt_def, tgt_cluster_eff, insert, total_with_ghost, insets);
            let rect = screen_rect(ctx, p);
            let area_id = egui::Id::new("frost_assembly_drop_outline");
            egui::Area::new(area_id)
                .order(egui::Order::Middle)
                .fixed_pos(rect.min)
                .interactable(false)
                .show(ctx, |ui| {
                    let (r, _) = ui.allocate_exact_size(
                        egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect(
                        r,
                        egui::CornerRadius::same(6),
                        egui::Color32::from_rgba_unmultiplied(
                            accent.r(),
                            accent.g(),
                            accent.b(),
                            28,
                        ),
                        egui::Stroke::new(1.5, accent),
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
        if let (Some(dragged_id), Some((tgt_rid, tgt_cluster_eff, insert))) =
            (drag.item, target)
        {
            if let Some(src) = drag.source {
                resolve_drop(placement, ribbons, items, dragged_id, src, tgt_rid, tgt_cluster_eff, insert);
            }
        }
        drag.item = None;
        drag.cursor = None;
        drag.source = None;
    }

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
        match def.role {
            RibbonRole::Panel => open.toggle(def.id, item.id),
            RibbonRole::Icon => {}
        }
        clicks.push(RibbonClick {
            item: item.id,
            ribbon: def.id,
            role: def.role,
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

/// One-shot floating panel for a declared item — derives its anchor
/// + width scope from the item's resolved `(ribbon, cluster)`.
/// Width is per-cluster, so Start and End clusters on the same
/// ribbon remember independent widths.
pub fn floating_window_for_item(
    ctx: &egui::Context,
    ribbons: &[RibbonDef],
    items: &[RibbonItem],
    placement: &RibbonPlacement,
    item_id: &'static str,
    title: &str,
    size: egui::Vec2,
    open: &mut bool,
    accent: egui::Color32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let Some(item) = find_item(items, item_id) else {
        return;
    };
    let (rid, cluster, _) = placement.resolve(item);
    let Some(def) = find_ribbon(ribbons, rid) else {
        return;
    };
    let anchor = panel_anchor(def, cluster);
    let scope = cluster_width_scope(def.id, effective_cluster(def.mode, cluster));
    crate::floating::floating_window_scoped(
        ctx,
        item_id,
        title,
        anchor,
        size,
        open,
        accent,
        scope,
        add_contents,
    );
}

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
