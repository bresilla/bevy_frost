use egui::Id;

use super::{ModuleInlineCtx, ModuleResponse, WorkspaceCtx};

/// A recursive Frost module.
///
/// Inline mode is rendered inside a pod. Workspace mode is rendered
/// when the module owns the active L1+ workspace level.
pub trait FrostModule {
    fn id(&self) -> Id;
    fn title(&self) -> &str;
    fn icon(&self) -> &'static str;

    fn inline(&mut self, ui: &mut egui::Ui, ctx: ModuleInlineCtx<'_>) -> ModuleResponse;

    fn workspace(&mut self, _ws: &mut WorkspaceCtx<'_>) {}
}
