//! Anchor → screen-position math for [`super::Pane2`]. Uniform
//! `RAIL_INSET = 46` on every edge — same recipe the original
//! `frostcore::floating::floating_window_scoped` uses (4 px gap
//! between pane and rail button strip on every side).
//!
//! Earlier revisions had per-anchor "far" pushes on the bottom/
//! right edges to fix what looked like dimension bugs. The actual
//! cause was a flex intrinsic-size pass painting strips at the
//! wrong rect (now fixed in `super::Pane2::lay_out_flex` by
//! allocating an exact size instead of `available_size_before_wrap`).
//! Once the paint was clean, the per-anchor pushes were no longer
//! needed and got removed.
//!
//! Generic axis math dispatches by `RailZone` to compute Start /
//! Middle / End placement along each rail's perpendicular axis.

use egui::{pos2, Rect, Vec2};

use super::anchor::{PaneAnchor, RailZone};
use super::RAIL_INSET;

/// Compute the top-left screen position for a pane of size `pane`
/// anchored at `anchor` inside `screen`.
pub(crate) fn compute_pane_pos(
    anchor: PaneAnchor,
    screen: Rect,
    pane: Vec2,
) -> egui::Pos2 {
    let inset = RAIL_INSET;
    let x_min = screen.min.x + inset;
    let y_min = screen.min.y + inset;
    let x_max = screen.max.x - inset;
    let y_max = screen.max.y - inset;

    // Side-rail (Left/Right) panes pin x to one edge and place y by
    // zone; horizontal-rail (Top/Bottom) panes pin y and place x by
    // zone.
    let x = match anchor {
        PaneAnchor::LeftRail(_) => x_min,
        PaneAnchor::RightRail(_) => x_max - pane.x,
        PaneAnchor::TopRail(z) | PaneAnchor::BottomRail(z) => match z {
            RailZone::Start  => x_min,
            RailZone::Middle => (x_min + x_max - pane.x) * 0.5,
            RailZone::End    => x_max - pane.x,
        },
    };
    let y = match anchor {
        PaneAnchor::TopRail(_) => y_min,
        PaneAnchor::BottomRail(_) => y_max - pane.y,
        PaneAnchor::LeftRail(z) | PaneAnchor::RightRail(z) => match z {
            RailZone::Start  => y_min,
            RailZone::Middle => (y_min + y_max - pane.y) * 0.5,
            RailZone::End    => y_max - pane.y,
        },
    };
    pos2(x, y)
}
