use frost_core::{
    FrostModule, FrostView, ModuleInlineCtx, ModuleResponse, ViewCtx, ViewId, WorkspaceCtx,
};

struct DualSurface;

impl FrostView for DualSurface {
    fn id(&self) -> ViewId {
        ViewId::new("dual")
    }

    fn title(&self) -> &str {
        "Dual"
    }

    fn icon(&self) -> &'static str {
        "dual"
    }

    fn show(&mut self, _ctx: &mut ViewCtx<'_>) {}
}

impl FrostModule for DualSurface {
    fn id(&self) -> egui::Id {
        egui::Id::new("dual-module")
    }

    fn title(&self) -> &str {
        "Dual"
    }

    fn icon(&self) -> &'static str {
        "dual"
    }

    fn inline(&mut self, _ui: &mut egui::Ui, _ctx: ModuleInlineCtx<'_>) -> ModuleResponse {
        ModuleResponse::none()
    }

    fn workspace(&mut self, _ws: &mut WorkspaceCtx<'_>) {}
}

fn assert_view<T: FrostView>(_value: &T) {}
fn assert_module<T: FrostModule>(_value: &T) {}

#[test]
fn one_surface_can_be_both_view_and_module() {
    let surface = DualSurface;
    assert_view(&surface);
    assert_module(&surface);
}
