//! # Ribbons — edge-anchored button strips.
//!
//! Two kinds live here:
//!
//! * [`SideRibbon`] / [`BarRibbon`] — *static* rows of buttons with
//!   no drag, no state, no persistence. Reach for these when you
//!   just need an activity-bar-style strip.
//! * [`RibbonLayout`] + [`SideActive`] — *drag-aware* layout. Each
//!   button is identified by a stable string id; users can
//!   rearrange buttons within a ribbon and (for side rails) move
//!   them between Left and Right.
//!
//! Add [`RibbonPlugin`] once at app start-up to install the
//! resources + the ghost paint system. `FrostPlugin` already adds it
//! transitively, so there's nothing extra to do when you've
//! installed the full frost stack.

pub mod active;
pub mod assembly;
pub mod declare;
pub mod ghost;
pub mod kinds;
pub mod layout;
mod paint;
pub mod static_ribbon;

pub use active::SideActive;
pub use assembly::{
    draw_assembly, find_item, find_ribbon, floating_window_for_item, panel_anchor,
    panel_anchor_for_item, RibbonClick, RibbonCluster, RibbonDef, RibbonDrag, RibbonEdge,
    RibbonItem, RibbonMode, RibbonOpen, RibbonPlacement, RibbonRole, RibbonWidth,
};
pub use declare::{draw_ribbon_buttons, RibbonButton};
pub use ghost::{paint_drop_ghost_system, RibbonGhostSet};
pub use kinds::{Bar, RibbonConstraint, RibbonKind, Side};
pub use layout::RibbonLayout;
pub use static_ribbon::{BarRibbon, SideRibbon};

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

use crate::style;

/// Installs everything ribbon-related: the [`RibbonLayout`] and
/// [`SideActive`] resources, plus the ghost drop-preview system.
/// Has a soft dep on [`crate::style`]'s `AccentColor`, so add
/// [`crate::ThemePlugin`] (or [`crate::FrostPlugin`]) as well.
pub struct RibbonPlugin;

impl Plugin for RibbonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RibbonLayout>()
            .init_resource::<SideActive>()
            .init_resource::<RibbonOpen>()
            .init_resource::<RibbonWidth>()
            .init_resource::<RibbonPlacement>()
            .init_resource::<RibbonDrag>()
            .configure_sets(
                EguiPrimaryContextPass,
                RibbonGhostSet.after(style::apply_theme),
            )
            .add_systems(
                EguiPrimaryContextPass,
                paint_drop_ghost_system.in_set(RibbonGhostSet),
            );
    }
}
