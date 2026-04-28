//! Anchor → screen position for [`super::Pane2`]. Uses `egui::Area`'s
//! `anchor(Align2, offset)` API so the Area auto-positions itself
//! based on its content size — the pane's main axis grows with body
//! content while the pinned corner stays put.
//!
//! For Start/End zones the corner closest to the rail stays pinned
//! and the opposite edge moves as content grows. For Middle zones the
//! pane's centre sits at the rail's mid-point and grows symmetrically.

use egui::{vec2, Align2, Vec2};

use super::anchor::{PaneAnchor, RailZone};
use super::RAIL_INSET;

/// Pick the `Align2` + offset that pins the pane to its rail-
/// adjacent corner. egui::Area::anchor reads this to position the
/// Area so the pinned corner stays put as content size changes.
pub(crate) fn anchor_align(anchor: PaneAnchor) -> (Align2, Vec2) {
    let i = RAIL_INSET;
    match anchor {
        PaneAnchor::LeftRail(RailZone::Start)   => (Align2::LEFT_TOP,    vec2( i,  i)),
        PaneAnchor::LeftRail(RailZone::Middle)  => (Align2::LEFT_CENTER, vec2( i,  0.0)),
        PaneAnchor::LeftRail(RailZone::End)     => (Align2::LEFT_BOTTOM, vec2( i, -i)),
        PaneAnchor::RightRail(RailZone::Start)  => (Align2::RIGHT_TOP,    vec2(-i,  i)),
        PaneAnchor::RightRail(RailZone::Middle) => (Align2::RIGHT_CENTER, vec2(-i,  0.0)),
        PaneAnchor::RightRail(RailZone::End)    => (Align2::RIGHT_BOTTOM, vec2(-i, -i)),
        PaneAnchor::TopRail(RailZone::Start)    => (Align2::LEFT_TOP,   vec2( i,  i)),
        PaneAnchor::TopRail(RailZone::Middle)   => (Align2::CENTER_TOP, vec2( 0.0,  i)),
        PaneAnchor::TopRail(RailZone::End)      => (Align2::RIGHT_TOP,  vec2(-i,  i)),
        PaneAnchor::BottomRail(RailZone::Start) => (Align2::LEFT_BOTTOM,   vec2( i, -i)),
        PaneAnchor::BottomRail(RailZone::Middle)=> (Align2::CENTER_BOTTOM, vec2( 0.0, -i)),
        PaneAnchor::BottomRail(RailZone::End)   => (Align2::RIGHT_BOTTOM,  vec2(-i, -i)),
    }
}
