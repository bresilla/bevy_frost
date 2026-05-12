use frost_core::{FrostView, ViewCtx, ViewId, ViewRouter, ViewRouterError};

struct MockView {
    id: ViewId,
    title: &'static str,
}

impl MockView {
    fn new(id: &'static str, title: &'static str) -> Self {
        Self {
            id: ViewId::new(id),
            title,
        }
    }
}

impl FrostView for MockView {
    fn id(&self) -> ViewId {
        self.id
    }

    fn title(&self) -> &str {
        self.title
    }

    fn icon(&self) -> &'static str {
        "square"
    }

    fn show(&mut self, _ctx: &mut ViewCtx<'_>) {}
}

#[test]
fn router_starts_with_first_registered_view() {
    let router = ViewRouter::new(MockView::new("bevy", "Bevy"));

    assert_eq!(router.active(), Ok(ViewId::new("bevy")));
    assert_eq!(router.active_entry().unwrap().title, "Bevy");
    assert_eq!(router.active_workspace().unwrap().depth(), 0);
}

#[test]
fn switching_preserves_each_views_workspace_stack() {
    let mut router = ViewRouter::new(MockView::new("bevy", "Bevy"));
    let graph = router.register(MockView::new("graph", "Graph"));
    let bevy = ViewId::new("bevy");

    router
        .active_workspace_mut()
        .unwrap()
        .push_module(egui::Id::new("inline_graph"));
    assert_eq!(router.active_workspace().unwrap().depth(), 1);

    router.set_active(graph).unwrap();
    assert_eq!(router.active_workspace().unwrap().depth(), 0);
    router
        .active_workspace_mut()
        .unwrap()
        .push_module(egui::Id::new("image"));
    assert_eq!(router.active_workspace().unwrap().depth(), 1);

    router.set_active(bevy).unwrap();
    assert_eq!(router.active_workspace().unwrap().depth(), 1);
    router.set_active(graph).unwrap();
    assert_eq!(router.active_workspace().unwrap().depth(), 1);
}

#[test]
fn unknown_view_switch_returns_typed_error() {
    let mut router = ViewRouter::new(MockView::new("bevy", "Bevy"));
    let missing = ViewId::new("missing");

    assert_eq!(
        router.set_active(missing),
        Err(ViewRouterError::UnknownView(missing))
    );
}
