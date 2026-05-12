use egui::Color32;

use crate::workspace::WorkspaceStack;

/// Rendering context for the active L0 view.
///
/// This starts deliberately small. Typed helpers for panes,
/// full-canvas surfaces, and view-local ribbons will layer on top
/// after the router and ribbon slot model are stable.
pub struct ViewCtx<'a> {
    pub egui_ctx: &'a egui::Context,
    pub workspace: &'a mut WorkspaceStack,
    pub accent: Color32,
}
