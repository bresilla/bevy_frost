use egui::{Id, Rect, pos2};
use frost_core::{
    RibbonEdge, ShelfContainer, ShelfDef, ShelfEdge, ShelfEdgeError, ShelfState, layout_shelves,
    style,
};

#[test]
fn shelf_edge_rejects_top() {
    assert_eq!(
        ShelfEdge::try_from(RibbonEdge::Top),
        Err(ShelfEdgeError::TopShelfForbidden)
    );
    assert_eq!(ShelfEdge::try_from(RibbonEdge::Left), Ok(ShelfEdge::Left));
    assert_eq!(ShelfEdge::try_from(RibbonEdge::Right), Ok(ShelfEdge::Right));
    assert_eq!(
        ShelfEdge::try_from(RibbonEdge::Bottom),
        Ok(ShelfEdge::Bottom)
    );
}

#[test]
fn shelves_reserve_viewport_space() {
    let theme = *style::theme().shelf();
    let mut state = ShelfState::default();
    let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));
    let shelves = vec![
        ShelfDef::new("left", ShelfEdge::Left, egui::Color32::WHITE).default_size(200.0),
        ShelfDef::new("right", ShelfEdge::Right, egui::Color32::WHITE).default_size(180.0),
        ShelfDef::new("bottom", ShelfEdge::Bottom, egui::Color32::WHITE).default_size(160.0),
    ];

    let layout = layout_shelves(available, &shelves, &mut state, &theme);

    assert_eq!(layout.left.unwrap().width(), 200.0);
    assert_eq!(layout.right.unwrap().width(), 180.0);
    assert_eq!(layout.bottom.unwrap().height(), 160.0);
    assert_eq!(layout.viewport.min, pos2(200.0, 0.0));
    assert_eq!(layout.viewport.max, pos2(820.0, 640.0));
}

#[test]
fn shelf_state_persists_size_and_active_container() {
    let shelf_id = Id::new("shelf");
    let container_id = Id::new("container");
    let mut state = ShelfState::default();

    assert_eq!(state.size(shelf_id), None);
    state.set_size(shelf_id, 333.0);
    assert_eq!(state.size(shelf_id), Some(333.0));

    assert_eq!(state.active_container(shelf_id), None);
    state.set_active_container(shelf_id, container_id);
    assert_eq!(state.active_container(shelf_id), Some(container_id));
}

#[test]
fn shelf_container_api_is_typed_tabbed_only() {
    let _container = ShelfContainer::tabbed(Id::new("tabbed"), "Inspector", "settings", vec![]);
}
