use frost_core::{
    RibbonAction, RibbonActionError, RibbonActionResult, RibbonOverrideLayer, RibbonOverridePolicy,
    RibbonSlot, RibbonSlotId, RibbonSlotItem, RibbonSlotOverride, ViewId, ViewRouter,
    dispatch_ribbon_action, permanent_system_control_ribbon, permanent_view_switcher_ribbon,
    resolve_slot_item, resolve_slot_items, restore_workspace_slot_override,
};

mod support {
    use frost_core::{FrostView, ViewCtx, ViewId};

    pub struct MockView {
        id: ViewId,
        title: &'static str,
        icon: &'static str,
    }

    impl MockView {
        pub fn new(id: &'static str, title: &'static str, icon: &'static str) -> Self {
            Self {
                id: ViewId::new(id),
                title,
                icon,
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
            self.icon
        }

        fn show(&mut self, _ctx: &mut ViewCtx<'_>) {}
    }
}

fn item(id: &'static str, icon: &'static str) -> RibbonSlotItem {
    RibbonSlotItem::new(
        egui::Id::new(id),
        icon,
        id,
        id,
        RibbonAction::Command(egui::Id::new(id)),
    )
}

#[test]
fn fixed_slot_ignores_layer_overrides() {
    let slot_id = RibbonSlotId::new("system.close_or_restore");
    let slot = RibbonSlot::new(
        slot_id,
        Some(item("close", "x")),
        RibbonOverridePolicy::Fixed,
    );
    let layer = RibbonOverrideLayer::new(vec![RibbonSlotOverride::new(
        slot_id,
        item("restore", "restore"),
    )]);

    assert_eq!(resolve_slot_item(&slot, &[layer]).unwrap().icon, "x");
}

#[test]
fn layer_override_replaces_default() {
    let slot_id = RibbonSlotId::new("system.close_or_restore");
    let slot = RibbonSlot::new(
        slot_id,
        Some(item("close", "x")),
        RibbonOverridePolicy::LayerOverride,
    );
    let layer = RibbonOverrideLayer::new(vec![RibbonSlotOverride::new(
        slot_id,
        item("restore", "restore"),
    )]);

    assert_eq!(resolve_slot_item(&slot, &[layer]).unwrap().icon, "restore");
}

#[test]
fn deeper_workspace_override_beats_view_override() {
    let slot_id = RibbonSlotId::new("system.close_or_restore");
    let slot = RibbonSlot::new(
        slot_id,
        Some(item("close", "x")),
        RibbonOverridePolicy::LayerOverride,
    );
    let view_layer = RibbonOverrideLayer::new(vec![RibbonSlotOverride::new(
        slot_id,
        RibbonSlotItem::new(
            egui::Id::new("view-settings"),
            "settings",
            "settings",
            "settings",
            RibbonAction::SwitchView(ViewId::new("settings")),
        ),
    )]);
    let l1_layer = RibbonOverrideLayer::new(vec![RibbonSlotOverride::new(
        slot_id,
        RibbonSlotItem::new(
            egui::Id::new("restore"),
            "restore",
            "restore",
            "restore",
            RibbonAction::PopWorkspace,
        ),
    )]);

    let resolved = resolve_slot_item(&slot, &[view_layer, l1_layer]).unwrap();
    assert_eq!(resolved.icon, "restore");
    assert_eq!(resolved.action, RibbonAction::PopWorkspace);
}

#[test]
fn fallback_returns_permanent_default_when_no_override_exists() {
    let slot = RibbonSlot::new(
        RibbonSlotId::new("global.status"),
        Some(item("status", "circle")),
        RibbonOverridePolicy::LayerOverride,
    );

    assert_eq!(resolve_slot_item(&slot, &[]).unwrap().icon, "circle");
}

#[test]
fn append_policy_keeps_default_and_adds_layer_items() {
    let slot_id = RibbonSlotId::new("global.tools");
    let slot = RibbonSlot::new(
        slot_id,
        Some(item("base", "home")),
        RibbonOverridePolicy::LayerAppend,
    );
    let view_layer =
        RibbonOverrideLayer::new(vec![RibbonSlotOverride::new(slot_id, item("view", "eye"))]);
    let l1_layer =
        RibbonOverrideLayer::new(vec![RibbonSlotOverride::new(slot_id, item("l1", "brush"))]);

    let icons: Vec<&'static str> = resolve_slot_items(&slot, &[view_layer, l1_layer])
        .into_iter()
        .map(|item| item.icon)
        .collect();
    assert_eq!(icons, vec!["home", "eye", "brush"]);
}

#[test]
fn permanent_view_switcher_generates_switch_view_items() {
    let mut router = ViewRouter::new(support::MockView::new("bevy", "Bevy", "cube"));
    router.register(support::MockView::new("graph", "Graph", "node_tree"));

    let ribbon = permanent_view_switcher_ribbon(router.entries());
    assert_eq!(ribbon.slots.len(), 2);
    let graph_item = ribbon.slots[1].default_item.as_ref().unwrap();
    assert_eq!(graph_item.icon, "node_tree");
    assert_eq!(
        graph_item.action,
        RibbonAction::SwitchView(ViewId::new("graph"))
    );
}

#[test]
fn permanent_system_control_slot_can_resolve_to_restore_override() {
    let ribbon = permanent_system_control_ribbon();
    let slot = &ribbon.slots[0];
    assert_eq!(
        resolve_slot_item(slot, &[]).unwrap().action,
        RibbonAction::CloseApp
    );

    let layer = RibbonOverrideLayer::new(vec![restore_workspace_slot_override()]);
    assert_eq!(
        resolve_slot_item(slot, &[layer]).unwrap().action,
        RibbonAction::PopWorkspace
    );
}

#[test]
fn dispatch_switch_view_and_workspace_actions() {
    let mut router = ViewRouter::new(support::MockView::new("bevy", "Bevy", "cube"));
    let graph = router.register(support::MockView::new("graph", "Graph", "node_tree"));

    assert_eq!(
        dispatch_ribbon_action(RibbonAction::SwitchView(graph), &mut router),
        Ok(RibbonActionResult::SwitchedView(graph))
    );
    assert_eq!(router.active(), Ok(graph));

    let module_id = egui::Id::new("image-module");
    assert_eq!(
        dispatch_ribbon_action(RibbonAction::PushModuleWorkspace(module_id), &mut router),
        Ok(RibbonActionResult::PushedModuleWorkspace(module_id))
    );
    assert_eq!(router.active_workspace().unwrap().depth(), 1);

    assert_eq!(
        dispatch_ribbon_action(RibbonAction::CloseApp, &mut router),
        Err(RibbonActionError::AppWindowControlsDenied)
    );

    assert_eq!(
        dispatch_ribbon_action(RibbonAction::PopWorkspace, &mut router),
        Ok(RibbonActionResult::PoppedWorkspace)
    );
    assert_eq!(router.active_workspace().unwrap().depth(), 0);
}
