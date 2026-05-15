//! Persistent docked Shelves.
//!
//! A Shelf is structural chrome: it reserves space on the left,
//! right, or bottom side of the workspace and hosts typed tabbed
//! containers. It is deliberately not a ribbon-opened floating
//! [`crate::pane::Pane`].

use std::collections::{HashMap, HashSet};

use egui::{Color32, Id, Pos2, Rect, Sense, Stroke, UiBuilder, Vec2, pos2, vec2};

use crate::container::Tab;
use crate::pane::{self, PaneAnchor, RailZone, active_pane_key};
use crate::ribbon::RibbonEdge;
use crate::style::{self, ShelfTheme};

/// Allowed dock edges for persistent Shelves.
///
/// There is intentionally no `Top` variant: top-level chrome is
/// owned by the persistent main bar/ribbon policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShelfEdge {
    Left,
    Right,
    Bottom,
}

impl ShelfEdge {
    #[must_use]
    pub fn is_side(self) -> bool {
        matches!(self, ShelfEdge::Left | ShelfEdge::Right)
    }

    #[must_use]
    pub fn container_anchor(self) -> PaneAnchor {
        match self {
            // Side shelves are docked vertical panes, but their
            // tabbed containers should expose tabs on the side, not
            // across the top.
            ShelfEdge::Left | ShelfEdge::Right => PaneAnchor::TopRail(RailZone::Middle),
            // Bottom shelves should expose each container's tabs
            // along the top edge of the docked pane.
            ShelfEdge::Bottom => PaneAnchor::LeftRail(RailZone::Middle),
        }
    }
}

/// Error returned when adapting a generic screen edge into a Shelf
/// edge. `Top` is rejected at the API boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShelfEdgeError {
    TopShelfForbidden,
}

impl TryFrom<RibbonEdge> for ShelfEdge {
    type Error = ShelfEdgeError;

    fn try_from(edge: RibbonEdge) -> Result<Self, Self::Error> {
        match edge {
            RibbonEdge::Left => Ok(ShelfEdge::Left),
            RibbonEdge::Right => Ok(ShelfEdge::Right),
            RibbonEdge::Bottom => Ok(ShelfEdge::Bottom),
            RibbonEdge::Top => Err(ShelfEdgeError::TopShelfForbidden),
        }
    }
}

/// A Shelf-hosted tabbed container. Public constructors only create
/// typed tabbed containers, so consumers cannot smuggle arbitrary
/// egui closures into Shelf content.
pub struct ShelfContainer<'a> {
    spec: crate::pane::ContainerSpec<'a>,
}

impl<'a> ShelfContainer<'a> {
    #[must_use]
    pub fn tabbed(
        id: impl Into<Id>,
        title: impl Into<String>,
        icon: &'static str,
        tabs: Vec<Tab>,
    ) -> Self {
        Self {
            spec: crate::pane::ContainerSpec::tabbed(id, title, icon, tabs),
        }
    }
}

/// Declarative Shelf definition for one workspace level.
pub struct ShelfDef<'a> {
    pub id: Id,
    pub edge: ShelfEdge,
    pub accent: Color32,
    pub containers: Vec<ShelfContainer<'a>>,
    pub default_size: Option<f32>,
    pub min_size: Option<f32>,
    pub max_size: Option<f32>,
    pub movable: bool,
}

impl<'a> ShelfDef<'a> {
    #[must_use]
    pub fn new(id: impl Into<Id>, edge: ShelfEdge, accent: Color32) -> Self {
        Self {
            id: id.into(),
            edge,
            accent,
            containers: Vec::new(),
            default_size: None,
            min_size: None,
            max_size: None,
            movable: false,
        }
    }

    #[must_use]
    pub fn default_size(mut self, size: f32) -> Self {
        self.default_size = Some(size);
        self
    }

    #[must_use]
    pub fn size_bounds(mut self, min: f32, max: f32) -> Self {
        self.min_size = Some(min);
        self.max_size = Some(max);
        self
    }

    #[must_use]
    pub fn movable(mut self) -> Self {
        self.movable = true;
        self
    }

    #[must_use]
    pub fn with_movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    #[must_use]
    pub fn container(mut self, container: ShelfContainer<'a>) -> Self {
        self.containers.push(container);
        self
    }

    #[must_use]
    pub fn containers(mut self, containers: impl IntoIterator<Item = ShelfContainer<'a>>) -> Self {
        self.containers.extend(containers);
        self
    }

    fn default_extent_for(&self, edge: ShelfEdge, theme: &ShelfTheme) -> f32 {
        let fallback = if edge.is_side() {
            theme.side_default_size
        } else {
            theme.bottom_default_size
        };
        sanitize_extent(self.default_size.unwrap_or(fallback), fallback.max(0.0))
    }

    fn min_extent(&self, theme: &ShelfTheme) -> f32 {
        self.extent_bounds(theme).0
    }

    fn max_extent(&self, theme: &ShelfTheme) -> f32 {
        self.extent_bounds(theme).1
    }

    fn extent_bounds(&self, theme: &ShelfTheme) -> (f32, f32) {
        normalize_extent_bounds(
            self.min_size.unwrap_or(theme.min_size),
            self.max_size.unwrap_or(theme.max_size),
            theme,
        )
    }
}

fn normalize_extent_bounds(min: f32, max: f32, theme: &ShelfTheme) -> (f32, f32) {
    let fallback_min = theme.min_size.max(0.0);
    let fallback_max = theme.max_size.max(fallback_min);
    let min = sanitize_extent(min, fallback_min);
    let max = sanitize_extent(max, fallback_max);
    if min <= max { (min, max) } else { (max, min) }
}

fn sanitize_extent(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ShelfDragState {
    shelf_id: Id,
    source_edge: ShelfEdge,
    cursor: Pos2,
    target_edge: Option<ShelfEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ShelfResizeStart {
    size: f32,
    pointer: Pos2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ShelfContainerMoveState {
    container_id: Id,
    source_shelf: Id,
    source_pane: Id,
    source_edge: ShelfEdge,
    cursor: Pos2,
    target_edge: Option<ShelfEdge>,
    target_shelf: Option<Id>,
    target_pane: Option<Id>,
    target_slot: Option<usize>,
    container_size: Vec2,
}

#[derive(Debug, Clone, Copy)]
struct ShelfContainerMoveUpdate {
    container_id: Id,
    source_shelf: Id,
    source_pane: Id,
    source_edge: ShelfEdge,
    cursor: Pos2,
    target_edge: Option<ShelfEdge>,
    container_size: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ShelfPaneInfo {
    shelf_id: Id,
    pane_id: Id,
    edge: ShelfEdge,
    horizontal_stack: bool,
    content_rect: Rect,
    screen_rect: Rect,
    screen_offset: Vec2,
    accent: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShelfContainerLocation {
    shelf_id: Option<Id>,
    edge: ShelfEdge,
}

fn detached_shelf_id(source_shelf: Id, container_id: Id) -> Id {
    source_shelf.with("frost_detached_shelf").with(container_id)
}

/// Persistent Shelf UI state: user sizes and per-Shelf active group.
#[derive(Debug, Default)]
pub struct ShelfState {
    sizes: HashMap<Id, f32>,
    resize_starts: HashMap<Id, ShelfResizeStart>,
    edge_overrides: HashMap<Id, ShelfEdge>,
    container_locations: HashMap<Id, ShelfContainerLocation>,
    active_containers: HashMap<Id, Id>,
    drag: Option<ShelfDragState>,
    container_move: Option<ShelfContainerMoveState>,
}

impl ShelfState {
    fn size(&self, key: Id) -> Option<f32> {
        self.sizes.get(&key).copied()
    }

    fn set_size(&mut self, key: Id, size: f32) {
        if size.is_finite() {
            self.sizes.insert(key, size.max(0.0));
        } else {
            self.sizes.remove(&key);
        }
    }

    /// Read the user's persisted size for a shelf on a concrete edge.
    ///
    /// Shelf sizes are intentionally edge-scoped: a shelf moved from
    /// left to bottom needs a different dimension axis, and moving it
    /// back should restore the side width instead of reusing the
    /// bottom height.
    #[must_use]
    pub fn edge_size(&self, shelf_id: Id, edge: ShelfEdge) -> Option<f32> {
        self.size(shelf_id.with(edge))
    }

    /// Persist a user size for a shelf on a concrete edge.
    pub fn set_edge_size(&mut self, shelf_id: Id, edge: ShelfEdge, size: f32) {
        self.set_size(shelf_id.with(edge), size);
    }

    #[must_use]
    pub fn edge(&self, shelf_id: Id, default: ShelfEdge) -> ShelfEdge {
        self.edge_overrides
            .get(&shelf_id)
            .copied()
            .unwrap_or(default)
    }

    pub fn set_edge(&mut self, shelf_id: Id, edge: ShelfEdge) {
        self.edge_overrides.insert(shelf_id, edge);
    }

    pub fn clear_edge_override(&mut self, shelf_id: Id) {
        self.edge_overrides.remove(&shelf_id);
    }

    #[must_use]
    pub fn container_edge(&self, container_id: Id, default: ShelfEdge) -> ShelfEdge {
        self.container_locations
            .get(&container_id)
            .map(|location| location.edge)
            .unwrap_or(default)
    }

    pub fn set_container_edge(&mut self, container_id: Id, edge: ShelfEdge) {
        self.container_locations.insert(
            container_id,
            ShelfContainerLocation {
                shelf_id: None,
                edge,
            },
        );
    }

    pub fn clear_container_edge_override(&mut self, container_id: Id) {
        self.container_locations.remove(&container_id);
    }

    fn container_location(
        &self,
        container_id: Id,
        default_edge: ShelfEdge,
    ) -> ShelfContainerLocation {
        self.container_locations
            .get(&container_id)
            .copied()
            .unwrap_or(ShelfContainerLocation {
                shelf_id: None,
                edge: default_edge,
            })
    }

    fn set_container_location(&mut self, container_id: Id, shelf_id: Option<Id>, edge: ShelfEdge) {
        self.container_locations
            .insert(container_id, ShelfContainerLocation { shelf_id, edge });
    }

    #[must_use]
    pub fn active_container(&self, shelf_id: Id) -> Option<Id> {
        self.active_containers.get(&shelf_id).copied()
    }

    pub fn set_active_container(&mut self, shelf_id: Id, container_id: Id) {
        self.active_containers.insert(shelf_id, container_id);
    }

    fn clear_active_container(&mut self, shelf_id: Id) {
        self.active_containers.remove(&shelf_id);
    }

    fn active_container_for_group(&self, group_id: Id) -> Option<Id> {
        self.active_containers.get(&group_id).copied()
    }

    fn set_active_container_for_group(&mut self, group_id: Id, container_id: Id) {
        self.active_containers.insert(group_id, container_id);
    }

    fn clear_active_container_for_group(&mut self, group_id: Id) {
        self.active_containers.remove(&group_id);
    }

    fn extent_for_key(
        &mut self,
        size_key: Id,
        shelf: &ShelfDef<'_>,
        edge: ShelfEdge,
        theme: &ShelfTheme,
    ) -> f32 {
        let (min, max) = shelf.extent_bounds(theme);
        let default = shelf.default_extent_for(edge, theme).clamp(min, max);
        let value = self.sizes.entry(size_key).or_insert(default);
        *value = sanitize_extent(*value, default).clamp(min, max);
        *value
    }

    fn begin_drag(&mut self, shelf_id: Id, source_edge: ShelfEdge, cursor: Pos2) {
        self.drag = Some(ShelfDragState {
            shelf_id,
            source_edge,
            cursor,
            target_edge: None,
        });
    }

    fn update_drag(&mut self, cursor: Pos2, target_edge: Option<ShelfEdge>) {
        if let Some(drag) = &mut self.drag {
            drag.cursor = cursor;
            drag.target_edge = target_edge;
        }
    }

    fn finish_drag(&mut self) {
        if let Some(drag) = self.drag.take()
            && let Some(target) = drag
                .target_edge
                .filter(|target| *target != drag.source_edge)
        {
            let source_group_key = shelf_active_container_key_for(drag.shelf_id, drag.source_edge);
            let target_group_key = shelf_active_container_key_for(drag.shelf_id, target);
            if let Some(active) = self.active_containers.remove(&source_group_key) {
                self.active_containers.insert(target_group_key, active);
            }
            if let Some(size) = self
                .sizes
                .get(&drag.shelf_id.with(drag.source_edge))
                .copied()
                .filter(|_| drag.source_edge.is_side() == target.is_side())
            {
                self.sizes.insert(drag.shelf_id.with(target), size);
            }
            self.resize_starts
                .remove(&drag.shelf_id.with(drag.source_edge));
            self.resize_starts.remove(&drag.shelf_id.with(target));
            for location in self.container_locations.values_mut().filter(|location| {
                location.shelf_id == Some(drag.shelf_id) && location.edge == drag.source_edge
            }) {
                location.edge = target;
            }
            self.set_edge(drag.shelf_id, target);
        }
    }

    fn cancel_drag(&mut self) {
        self.drag = None;
    }

    fn update_container_move(&mut self, update: ShelfContainerMoveUpdate) {
        let previous_slot = self
            .container_move
            .filter(|drag| {
                drag.container_id == update.container_id && drag.target_edge == update.target_edge
            })
            .and_then(|drag| {
                drag.target_pane
                    .zip(drag.target_slot)
                    .zip(drag.target_shelf)
            })
            .map(|((pane, slot), shelf)| (pane, slot, shelf));
        self.container_move = Some(ShelfContainerMoveState {
            container_id: update.container_id,
            source_shelf: update.source_shelf,
            source_pane: update.source_pane,
            source_edge: update.source_edge,
            cursor: update.cursor,
            target_edge: update.target_edge,
            target_shelf: previous_slot.map(|(_, _, shelf)| shelf),
            target_pane: previous_slot.map(|(pane, _, _)| pane),
            target_slot: previous_slot.map(|(_, slot, _)| slot),
            container_size: update.container_size,
        });
    }

    fn update_container_move_target_slot(
        &mut self,
        target_shelf: Id,
        target_pane: Id,
        target_slot: usize,
        target_size: Vec2,
    ) {
        if let Some(drag) = &mut self.container_move {
            drag.target_shelf = Some(target_shelf);
            drag.target_pane = Some(target_pane);
            drag.target_slot = Some(target_slot);
            drag.container_size = target_size;
        }
    }

    fn clear_container_move_target_slot(&mut self) {
        if let Some(drag) = &mut self.container_move {
            drag.target_shelf = None;
            drag.target_pane = None;
            drag.target_slot = None;
        }
    }

    fn clear_container_move(&mut self) {
        self.container_move = None;
    }
}

/// Output of Shelf layout reservation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShelfLayout {
    pub viewport: Rect,
    pub left: Option<Rect>,
    pub right: Option<Rect>,
    pub bottom: Option<Rect>,
}

impl ShelfLayout {
    #[must_use]
    pub fn rect_for(self, edge: ShelfEdge) -> Option<Rect> {
        match edge {
            ShelfEdge::Left => self.left,
            ShelfEdge::Right => self.right,
            ShelfEdge::Bottom => self.bottom,
        }
    }

    #[must_use]
    pub fn available(self) -> Rect {
        let mut rect = self.viewport;
        for shelf in [self.left, self.right, self.bottom].into_iter().flatten() {
            rect.min.x = rect.min.x.min(shelf.min.x);
            rect.min.y = rect.min.y.min(shelf.min.y);
            rect.max.x = rect.max.x.max(shelf.max.x);
            rect.max.y = rect.max.y.max(shelf.max.y);
        }
        rect
    }
}

/// Reserve structural Shelf space and return the remaining viewport.
pub fn layout_shelves(
    available: Rect,
    shelves: &[ShelfDef<'_>],
    state: &mut ShelfState,
    theme: &ShelfTheme,
) -> ShelfLayout {
    assert_unique_shelf_ids(shelves);
    let mut viewport = available;
    let mut left = None;
    let mut right = None;
    let mut bottom = None;

    for entry in shelf_layout_edges(shelves, state) {
        let shelf = &shelves[entry.base_idx];
        let edge = entry.edge;
        let extent = state.extent_for_key(entry.shelf_id.with(edge), shelf, edge, theme);
        match edge {
            ShelfEdge::Left => {
                let extent = extent.min(viewport.width().max(0.0));
                let rect =
                    Rect::from_min_max(viewport.min, pos2(viewport.min.x + extent, viewport.max.y));
                viewport.min.x = (viewport.min.x + extent).min(viewport.max.x);
                left = Some(rect);
            }
            ShelfEdge::Right => {
                let extent = extent.min(viewport.width().max(0.0));
                let rect =
                    Rect::from_min_max(pos2(viewport.max.x - extent, viewport.min.y), viewport.max);
                viewport.max.x = (viewport.max.x - extent).max(viewport.min.x);
                right = Some(rect);
            }
            ShelfEdge::Bottom => {
                let extent = extent.min(viewport.height().max(0.0));
                let rect =
                    Rect::from_min_max(pos2(viewport.min.x, viewport.max.y - extent), viewport.max);
                viewport.max.y = (viewport.max.y - extent).max(viewport.min.y);
                bottom = Some(rect);
            }
        }
    }

    ShelfLayout {
        viewport,
        left,
        right,
        bottom,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ShelfLayoutEntry {
    base_idx: usize,
    shelf_id: Id,
    edge: ShelfEdge,
}

fn shelf_layout_edges(shelves: &[ShelfDef<'_>], state: &ShelfState) -> Vec<ShelfLayoutEntry> {
    let mut out = Vec::new();
    let resolved_edges = resolved_shelf_edges(shelves, state);
    let moved_shelf_owners = declared_moved_shelf_owners(shelves, state);
    let shelf_indices: HashMap<Id, usize> = shelves
        .iter()
        .enumerate()
        .map(|(idx, shelf)| (shelf.id, idx))
        .collect();
    for (idx, shelf) in shelves.iter().enumerate() {
        let default_edge = resolved_edges.get(&shelf.id).copied().unwrap_or(shelf.edge);
        if shelf.containers.is_empty() {
            if !moved_shelf_owners.contains(&shelf.id) {
                push_unique_edge(&mut out, idx, shelf.id, default_edge);
            }
            continue;
        }
        for container in &shelf.containers {
            let location = state.container_location(container.spec.container_id(), default_edge);
            let (shelf_idx, shelf_id) = resolve_target_layout_shelf(
                shelves,
                &shelf_indices,
                &resolved_edges,
                idx,
                shelf.id,
                location,
            );
            push_unique_edge(&mut out, shelf_idx, shelf_id, location.edge);
        }
    }
    out.sort_by_key(|entry| shelf_reservation_order(entry.edge));
    out
}

fn resolve_target_layout_shelf(
    shelves: &[ShelfDef<'_>],
    shelf_indices: &HashMap<Id, usize>,
    resolved_edges: &HashMap<Id, ShelfEdge>,
    source_idx: usize,
    source_shelf: Id,
    location: ShelfContainerLocation,
) -> (usize, Id) {
    if let Some(target_shelf) = location.shelf_id {
        if let Some(idx) = shelf_indices.get(&target_shelf).copied() {
            return (idx, target_shelf);
        }
        if let Some(idx) = shelf_index_for_edge(shelves, resolved_edges, location.edge) {
            return (idx, shelves[idx].id);
        }
        return (source_idx, target_shelf);
    }
    if let Some(idx) = shelf_index_for_edge(shelves, resolved_edges, location.edge) {
        return (idx, shelves[idx].id);
    }
    (source_idx, source_shelf)
}

fn resolved_shelf_edges(shelves: &[ShelfDef<'_>], state: &ShelfState) -> HashMap<Id, ShelfEdge> {
    let mut out = HashMap::with_capacity(shelves.len());
    let mut occupied = HashSet::with_capacity(shelves.len());

    for shelf in shelves {
        if !state.edge_overrides.contains_key(&shelf.id) && occupied.insert(shelf.edge) {
            out.insert(shelf.id, shelf.edge);
        }
    }

    for shelf in shelves {
        if out.contains_key(&shelf.id) {
            continue;
        }
        let desired = state.edge(shelf.id, shelf.edge);
        let edge = if occupied.insert(desired) {
            desired
        } else if occupied.insert(shelf.edge) {
            shelf.edge
        } else {
            desired
        };
        out.insert(shelf.id, edge);
    }

    out
}

fn declared_moved_shelf_owners(shelves: &[ShelfDef<'_>], state: &ShelfState) -> HashSet<Id> {
    shelves
        .iter()
        .flat_map(|shelf| shelf.containers.iter())
        .filter_map(|container| {
            state
                .container_locations
                .get(&container.spec.container_id())
                .and_then(|location| location.shelf_id)
        })
        .collect()
}

fn shelf_index_for_edge(
    shelves: &[ShelfDef<'_>],
    resolved_edges: &HashMap<Id, ShelfEdge>,
    edge: ShelfEdge,
) -> Option<usize> {
    shelves
        .iter()
        .position(|shelf| resolved_edges.get(&shelf.id).copied().unwrap_or(shelf.edge) == edge)
}

fn shelf_reservation_order(edge: ShelfEdge) -> u8 {
    match edge {
        ShelfEdge::Left => 0,
        ShelfEdge::Right => 1,
        ShelfEdge::Bottom => 2,
    }
}

fn push_unique_edge(
    out: &mut Vec<ShelfLayoutEntry>,
    base_idx: usize,
    shelf_id: Id,
    edge: ShelfEdge,
) {
    if !out.iter().any(|existing| existing.edge == edge) {
        out.push(ShelfLayoutEntry {
            base_idx,
            shelf_id,
            edge,
        });
    }
}

/// Paint all Shelves and their typed tabbed containers.
pub fn show_shelves<'a>(
    ctx: &egui::Context,
    layout: ShelfLayout,
    shelves: Vec<ShelfDef<'a>>,
    state: &mut ShelfState,
) {
    assert_unique_shelf_ids(&shelves);
    publish_shelf_layout(ctx, layout);
    clear_published_shelf_pane_infos(ctx);
    let theme = style::theme();
    let shelf_theme = *theme.shelf();
    let available = layout.available();
    let mut shelves = split_shelf_render_groups(shelves, state);
    let mut tab_scope = pane::TabRoutingScope::new();
    for shelf in &mut shelves {
        for container in &mut shelf.containers {
            tab_scope.absorb_spec(&mut container.spec);
        }
    }
    let tab_routing_id = shelf_tab_routing_id();

    for shelf in shelves {
        let Some(rect) = layout.rect_for(shelf.edge) else {
            continue;
        };
        let render_id = shelf_render_id(&shelf);
        let pane_id = shelf_pane_id(&shelf);
        let shelf_id = shelf.id;
        let shelf_edge = shelf.edge;
        let shelf_movable = shelf.movable;

        let area = egui::Area::new(render_id.with("frost_shelf_area"))
            .order(egui::Order::Middle)
            .fixed_pos(rect.min)
            .interactable(true);

        area.show(ctx, |ui| {
            let shelf_rect = Rect::from_min_size(ui.min_rect().min, rect.size());
            let screen_offset = rect.min - shelf_rect.min;
            ui.set_min_size(rect.size());
            let move_response = ui.interact(
                shelf_rect,
                render_id.with("background_move"),
                Sense::click_and_drag(),
            );
            paint_shelf_background(ui, shelf_rect, shelf.accent, &shelf_theme);
            let resize_response =
                resize_shelf(ui, &shelf, render_id, state, &shelf_theme, shelf_rect);

            let content_rect = shelf_rect.shrink(shelf_theme.padding);
            if resize_response.drag_started() || resize_response.dragged() {
                state.cancel_drag();
            }

            render_shelf_body(ShelfBodyInput {
                ui,
                content_rect,
                shelf_rect,
                screen_offset,
                layout,
                shelf,
                state,
                tab_routing_id,
                tab_scope: &mut tab_scope,
            });

            let pointer_on_resize = resize_response.interact_pointer_pos().is_some_and(|pos| {
                resize_handle_rect(shelf_edge, shelf_rect, &shelf_theme).contains(pos)
            });
            if shelf_movable
                && !resize_response.drag_started()
                && !resize_response.dragged()
                && !pointer_on_resize
            {
                handle_shelf_move_drag(ShelfMoveDragInput {
                    ctx: ui.ctx(),
                    shelf_id,
                    shelf_edge,
                    pane_id,
                    state,
                    layout,
                    available,
                    shelf_rect,
                    response: &move_response,
                });
            }
        });
    }

    update_container_move_target_from_published(ctx, state);
    finish_container_move_if_released(ctx, state);
    paint_shelf_move_ghost(ctx, layout, state, &shelf_theme);
    paint_container_move_ghost(ctx, layout, state, &shelf_theme);
}

fn assert_unique_shelf_ids(shelves: &[ShelfDef<'_>]) {
    let mut seen_shelves = HashSet::with_capacity(shelves.len());
    assert!(
        shelves.iter().all(|shelf| seen_shelves.insert(shelf.id)),
        "shelves require unique shelf ids"
    );
    let container_count = shelves.iter().map(|shelf| shelf.containers.len()).sum();
    let mut seen_containers = HashSet::with_capacity(container_count);
    assert!(
        shelves
            .iter()
            .flat_map(|shelf| shelf.containers.iter())
            .all(|container| seen_containers.insert(container.spec.container_id())),
        "shelf containers require unique container ids"
    );
}

fn split_shelf_render_groups<'a>(
    shelves: Vec<ShelfDef<'a>>,
    state: &ShelfState,
) -> Vec<ShelfDef<'a>> {
    let mut groups = Vec::new();
    let moved_shelf_owners = declared_moved_shelf_owners(&shelves, state);
    let resolved_edges = resolved_shelf_edges(&shelves, state);
    let bases: Vec<ShelfRenderBase> = shelves
        .iter()
        .map(|shelf| ShelfRenderBase {
            id: shelf.id,
            edge: resolved_edges.get(&shelf.id).copied().unwrap_or(shelf.edge),
            accent: shelf.accent,
            default_size: shelf.default_size,
            min_size: shelf.min_size,
            max_size: shelf.max_size,
            movable: shelf.movable,
        })
        .collect();
    for mut shelf in shelves {
        let default_edge = resolved_edges.get(&shelf.id).copied().unwrap_or(shelf.edge);
        shelf.edge = default_edge;
        if shelf.containers.is_empty() {
            if !moved_shelf_owners.contains(&shelf.id) {
                push_shelf_render_group(&mut groups, shelf, default_edge);
            }
            continue;
        }
        let base = ShelfRenderBase {
            id: shelf.id,
            edge: default_edge,
            accent: shelf.accent,
            default_size: shelf.default_size,
            min_size: shelf.min_size,
            max_size: shelf.max_size,
            movable: shelf.movable,
        };
        for container in shelf.containers {
            let location = state.container_location(container.spec.container_id(), default_edge);
            let target_base = resolve_target_render_base(&bases, base, location);
            push_container_render_group(&mut groups, target_base, location.edge, container);
        }
    }
    groups
}

#[derive(Debug, Clone, Copy)]
struct ShelfRenderBase {
    id: Id,
    edge: ShelfEdge,
    accent: Color32,
    default_size: Option<f32>,
    min_size: Option<f32>,
    max_size: Option<f32>,
    movable: bool,
}

fn resolve_target_render_base(
    bases: &[ShelfRenderBase],
    source_base: ShelfRenderBase,
    location: ShelfContainerLocation,
) -> ShelfRenderBase {
    if let Some(target_shelf) = location.shelf_id {
        if let Some(base) = bases.iter().find(|base| base.id == target_shelf).copied() {
            return base;
        }
        if let Some(base) = bases
            .iter()
            .find(|base| base.edge == location.edge)
            .copied()
        {
            return base;
        }
        return ShelfRenderBase {
            id: target_shelf,
            edge: location.edge,
            ..source_base
        };
    }
    bases
        .iter()
        .find(|base| base.edge == location.edge)
        .copied()
        .unwrap_or(source_base)
}

fn push_shelf_render_group<'a>(
    groups: &mut Vec<ShelfDef<'a>>,
    mut shelf: ShelfDef<'a>,
    edge: ShelfEdge,
) {
    shelf.edge = edge;
    if groups.iter().any(|group| group.edge == edge) {
        return;
    }
    groups.push(shelf);
}

fn push_container_render_group<'a>(
    groups: &mut Vec<ShelfDef<'a>>,
    base: ShelfRenderBase,
    edge: ShelfEdge,
    container: ShelfContainer<'a>,
) {
    if let Some(group) = groups.iter_mut().find(|group| group.edge == edge) {
        group.containers.push(container);
        return;
    }
    groups.push(ShelfDef {
        id: base.id,
        edge,
        accent: base.accent,
        containers: vec![container],
        default_size: base.default_size,
        min_size: base.min_size,
        max_size: base.max_size,
        movable: base.movable,
    });
}

fn shelf_render_id(shelf: &ShelfDef<'_>) -> Id {
    shelf_render_key(shelf.id, shelf.edge)
}

fn shelf_render_key(shelf_id: Id, edge: ShelfEdge) -> Id {
    shelf_id.with(edge)
}

fn shelf_pane_id(shelf: &ShelfDef<'_>) -> Id {
    shelf_render_id(shelf).with("shelf_pane_scope")
}

fn shelf_tab_routing_id() -> Id {
    Id::new("frost_shelf_tab_routing_scope")
}

fn shelf_active_container_key(shelf: &ShelfDef<'_>) -> Id {
    shelf_active_container_key_for(shelf.id, shelf.edge)
}

fn shelf_active_container_key_for(shelf_id: Id, edge: ShelfEdge) -> Id {
    shelf_render_key(shelf_id, edge).with("active_container")
}

struct ShelfBodyInput<'ui, 'state, 'scope, 'a> {
    ui: &'ui mut egui::Ui,
    content_rect: Rect,
    shelf_rect: Rect,
    screen_offset: Vec2,
    layout: ShelfLayout,
    shelf: ShelfDef<'a>,
    state: &'state mut ShelfState,
    tab_routing_id: Id,
    tab_scope: &'scope mut pane::TabRoutingScope,
}

fn render_shelf_body(input: ShelfBodyInput<'_, '_, '_, '_>) {
    let ShelfBodyInput {
        ui,
        content_rect,
        shelf_rect,
        screen_offset,
        layout,
        shelf,
        state,
        tab_routing_id,
        tab_scope,
    } = input;
    let pane_id = shelf_pane_id(&shelf);
    let anchor = shelf.edge.container_anchor();
    // Container stack axis mirrors `Pane::lay_out_flex`: vertical-
    // strip title sides stack containers horizontally, horizontal-
    // strip title sides stack them vertically. The previous
    // implementation hard-coded `top_down` for every Shelf edge, so
    // the Bottom shelf stacked containers vertically when they
    // should flow horizontally — and the drag ghost-gap allocated
    // along the wrong axis as a result.
    let horizontal_stack = !anchor.title_side().is_horizontal_strip();
    let stack_layout = if horizontal_stack {
        egui::Layout::left_to_right(egui::Align::Min)
    } else {
        egui::Layout::top_down(egui::Align::Min)
    };

    ui.ctx().data_mut(|d| {
        d.insert_temp(active_pane_key(), pane_id);
        d.insert_temp(pane_id.with("frost_pane_open_elapsed"), 99.0_f32);
        d.insert_temp(pane_id.with("frost_pane_section_idx"), 0_u32);
    });
    pane::clear_container_min_widths(ui.ctx(), pane_id);

    // Body viewport — same role as the `ui` `Pane::lay_out_flex`
    // hands to the body closure. All drag plumbing (cache writes,
    // trailing ghost gap, finalize, preview) runs on THIS ui so the
    // recorded rects, the ghost slot, and the cursor/release event
    // all share one coordinate space.
    let mut viewport = ui.new_child(UiBuilder::new().max_rect(content_rect).layout(stack_layout));
    // Zero item spacing so the ghost gap sits flush against
    // neighbouring containers (matches `Pane::lay_out_flex`).
    viewport.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

    pane::begin_drag_frame(viewport.ctx(), pane_id);
    pane::clear_container_dot_rects(viewport.ctx(), pane_id);
    clear_external_container_gap(viewport.ctx(), pane_id);
    let pre_body_drag = pane::drag_state(viewport.ctx(), pane_id);
    if let (Some(item), Some(pos)) = (pre_body_drag.item, viewport.ctx().pointer_interact_pos()) {
        pane::set_drag(
            viewport.ctx(),
            pane_id,
            pane::DragState {
                item: Some(item),
                cursor: Some(pos),
            },
        );
    }
    pane::tab_drag::begin_frame(viewport.ctx(), pane_id);

    let screen_shelf_rect = shelf_rect.translate(screen_offset);
    let pointer_cursor = viewport
        .ctx()
        .pointer_interact_pos()
        .or_else(|| viewport.ctx().pointer_latest_pos());
    let suppress_source_container_gap = should_suppress_source_container_gap(
        pre_body_drag,
        state.container_move,
        pane_id,
        shelf.edge,
        screen_shelf_rect,
        layout,
        pointer_cursor,
    );
    pane::set_ghost_gap_suppressed(viewport.ctx(), pane_id, suppress_source_container_gap);
    let external_container_gap = should_render_external_container_gap(
        pre_body_drag,
        state.container_move,
        shelf.edge,
        screen_shelf_rect,
        pointer_cursor,
    )
    .then_some(())
    .and(state.container_move);
    let saved_target_snapshot = if let Some(drag) = external_container_gap {
        let mut synthetic_snapshot = pane::snapshot(viewport.ctx(), pane_id);
        synthetic_snapshot.retain(|entry| entry.id != drag.container_id);
        let size = container_move_ghost_size_for_edge(
            viewport.ctx(),
            drag.container_id,
            shelf.edge,
            content_rect,
        );
        synthetic_snapshot.push(pane::RectEntry {
            id: drag.container_id,
            rect: Rect::from_min_size(content_rect.min, size),
            frame: None,
        });
        pane::set_snapshot(viewport.ctx(), pane_id, synthetic_snapshot);
        pane::set_drag(
            viewport.ctx(),
            pane_id,
            pane::DragState {
                item: Some(drag.container_id),
                cursor: Some(pointer_cursor.unwrap_or(drag.cursor)),
            },
        );
        mark_external_container_gap(viewport.ctx(), pane_id);
        Some(pre_body_drag)
    } else {
        None
    };

    let active_key = shelf_active_container_key(&shelf);
    let specs: Vec<_> = shelf
        .containers
        .into_iter()
        .map(|container| container.spec)
        .collect();
    let declared_order: Vec<Id> = specs.iter().map(|spec| spec.container_id()).collect();
    let responses = crate::pane::render_containers_with_tab_scope(
        &mut viewport,
        pane_id,
        tab_routing_id,
        anchor,
        shelf.accent,
        specs,
        tab_scope,
    );

    let effective_active = resolve_visible_active_container(
        viewport.ctx(),
        pane_id,
        state.active_container_for_group(active_key),
        &declared_order,
        |id| responses.contains_key(&id),
    );
    if let Some(container_id) = effective_active {
        state.set_active_container_for_group(active_key, container_id);
    }
    if let Some(container_id) = effective_active {
        state.set_active_container(shelf.id, container_id);
    } else {
        state.clear_active_container(shelf.id);
        state.clear_active_container_for_group(active_key);
    }

    // ── Trailing ghost gap ──
    //
    // Same logic as `Pane::lay_out_flex`: when the cursor's slot is
    // past the last rendered container, paint the gap inline at the
    // end of the viewport so the trailing drop position is visible.
    let drag_state = pane::drag_state(viewport.ctx(), pane_id);
    if let Some(dragged_id) = drag_state.item
        && !pane::ghost_gap_suppressed(viewport.ctx(), pane_id)
    {
        let snap = pane::target_cache(viewport.ctx(), pane_id);
        let total = pane::current_cache(viewport.ctx(), pane_id).len();
        let cursor = viewport.ctx().pointer_interact_pos().or(drag_state.cursor);
        if let Some(c) = cursor {
            let cursor_axis = if horizontal_stack { c.x } else { c.y };
            let target_idx = pane::compute_target(&snap, dragged_id, cursor_axis, horizontal_stack);
            if target_idx >= total
                && let Some(entry) = pane::dragged_entry(&snap, dragged_id)
            {
                let entry = source_shelf_gap_entry(
                    viewport.ctx(),
                    dragged_id,
                    shelf.edge,
                    content_rect,
                    entry,
                );
                pane::paint_ghost_gap_entry_inline(
                    &mut viewport,
                    entry,
                    shelf.accent,
                    horizontal_stack,
                );
            }
        }
    }

    pane::finalize_snapshot(viewport.ctx(), pane_id);
    publish_shelf_pane_info(
        viewport.ctx(),
        ShelfPaneInfo {
            shelf_id: shelf.id,
            pane_id,
            edge: shelf.edge,
            horizontal_stack,
            content_rect,
            screen_rect: shelf_rect.translate(screen_offset),
            screen_offset,
            accent: shelf.accent,
        },
    );
    update_container_move_target_slot(
        &mut viewport,
        shelf.id,
        pane_id,
        shelf.edge,
        horizontal_stack,
        content_rect,
        state,
    );

    let external_gap_drag = saved_target_snapshot.is_some();
    if let Some(saved_drag) = saved_target_snapshot {
        if saved_drag.item.is_some() {
            pane::set_drag(viewport.ctx(), pane_id, saved_drag);
        } else {
            pane::clear_drag(viewport.ctx(), pane_id);
        }
    }
    if external_gap_drag {
        return;
    }

    if let Some(dragged_id) = drag_state.item {
        let snap = pane::target_cache(viewport.ctx(), pane_id);
        let cursor = viewport.ctx().pointer_interact_pos().or(drag_state.cursor);
        if let Some(c) = cursor {
            if let Some(entry) = pane::dragged_entry(&snap, dragged_id) {
                let screen_shelf_rect = shelf_rect.translate(screen_offset);
                let target_edge =
                    container_move_target_for_cursor(c, screen_shelf_rect, layout, shelf.edge);
                let target_size = target_edge
                    .map(|edge| {
                        container_move_ghost_size_for_edge(
                            viewport.ctx(),
                            dragged_id,
                            edge,
                            content_rect,
                        )
                    })
                    .unwrap_or_else(|| entry.rect.size());
                state.update_container_move(ShelfContainerMoveUpdate {
                    container_id: dragged_id,
                    source_shelf: shelf.id,
                    source_pane: pane_id,
                    source_edge: shelf.edge,
                    cursor: c,
                    target_edge,
                    container_size: target_size,
                });
                update_container_move_target_from_published(viewport.ctx(), state);
            }
            if should_paint_source_container_preview(
                drag_state,
                state.container_move,
                pane_id,
                shelf.edge,
            ) {
                pane::paint_drag_preview(
                    viewport.ctx(),
                    pane_id,
                    &snap,
                    dragged_id,
                    c,
                    shelf.accent,
                );
            }
            viewport.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        if viewport.ctx().input(|i| i.pointer.any_released()) {
            if let Some(drag) = state
                .container_move
                .filter(|drag| drag.container_id == dragged_id)
            {
                if drag.target_edge.is_some() {
                    return;
                }
                let screen_shelf_rect = shelf_rect.translate(screen_offset);
                if should_cancel_no_target_container_release(cursor, screen_shelf_rect) {
                    pane::clear_drag(viewport.ctx(), pane_id);
                    state.clear_container_move();
                    return;
                }
            }
            if let Some(c) = cursor {
                let cursor_axis = if horizontal_stack { c.x } else { c.y };
                commit_shelf_container_reorder(
                    viewport.ctx(),
                    pane_id,
                    dragged_id,
                    cursor_axis,
                    horizontal_stack,
                );
            }
            pane::clear_drag(viewport.ctx(), pane_id);
            state.clear_container_move();
        }
    }

    // ── Tab drag: preview + commit-on-release (Shelf scope) ──
    //
    // `render_containers` runs through the same tab-drag plumbing as
    // a normal Pane, so the drag STARTS work in a Shelf. Without
    // this block the pointer-release event has nowhere to commit /
    // clear, leaving the dragged tab stuck to the cursor.
    if let Some(tab_drag_state) = pane::tab_drag::drag_state(viewport.ctx(), pane_id) {
        let cursor = viewport
            .ctx()
            .pointer_latest_pos()
            .or(tab_drag_state.cursor);
        if let Some(c) = cursor {
            pane::tab_drag::set_drag(
                viewport.ctx(),
                pane_id,
                pane::tab_drag::TabDragState {
                    cursor: Some(c),
                    ..tab_drag_state
                },
            );
            pane::tab_drag::paint_drag_preview(
                viewport.ctx(),
                pane_id,
                egui::vec2(28.0, 28.0),
                c,
                shelf.accent,
                "",
                tab_drag_state.icon,
            );
            viewport.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        if viewport.ctx().input(|i| i.pointer.any_released()) {
            if let Some(c) = cursor
                && let Some((tgt_cid, slot)) = pane::tab_drag::find_drop_target_for_drag(
                    viewport.ctx(),
                    pane_id,
                    c,
                    tab_drag_state,
                )
            {
                pane::tab_drag::commit_drop(
                    viewport.ctx(),
                    tab_routing_id,
                    tab_drag_state.tab_id,
                    tab_drag_state.source_container,
                    tgt_cid,
                    slot,
                );
            }
            pane::tab_drag::clear_drag(viewport.ctx(), pane_id);
        }
    }
}

fn resolve_visible_active_container(
    ctx: &egui::Context,
    pane_id: Id,
    active: Option<Id>,
    declared_order: &[Id],
    is_visible: impl Fn(Id) -> bool,
) -> Option<Id> {
    if let Some(active) = active.filter(|active| is_visible(*active)) {
        return Some(active);
    }
    shelf_display_order(ctx, pane_id, declared_order.iter()).find(|id| is_visible(*id))
}

/// Publish the post-Shelf viewport as the chrome bounds for floating
/// ribbons/panes. Call this before drawing ribbons when Shelves are
/// present; [`show_shelves`] does it automatically.
pub fn publish_shelf_layout(ctx: &egui::Context, layout: ShelfLayout) {
    ctx.data_mut(|d| {
        d.insert_temp(crate::ribbon::chrome::chrome_bounds_key(), layout.viewport);
    });
}

fn shelf_display_order<'a>(
    ctx: &egui::Context,
    pane_id: Id,
    containers: impl Iterator<Item = &'a Id>,
) -> impl Iterator<Item = Id> {
    let defaults: Vec<Id> = containers.copied().collect();
    pane::section_order_for(ctx, pane_id, &defaults).into_iter()
}

fn commit_shelf_container_reorder(
    ctx: &egui::Context,
    pane_id: Id,
    dragged_id: Id,
    cursor_axis: f32,
    horizontal_stack: bool,
) {
    let cache = shelf_target_cache(ctx, pane_id);
    let target_idx = pane::compute_target(&cache, dragged_id, cursor_axis, horizontal_stack);
    let defaults: Vec<Id> = cache.iter().map(|e| e.id).collect();
    let mut order = pane::section_order_for(ctx, pane_id, &defaults);
    order.retain(|cid| *cid != dragged_id);
    let clamped = target_idx.min(order.len());
    order.insert(clamped, dragged_id);
    pane::set_section_order(ctx, pane_id, order);
}

fn paint_shelf_background(ui: &mut egui::Ui, rect: Rect, accent: Color32, theme: &ShelfTheme) {
    let active = style::theme();
    let fill = style::glass_fill(active.bg_panel, accent, theme.background_alpha);
    let stroke = Stroke::new(theme.border_width, style::widget_border(accent));
    ui.painter().rect_filled(rect, 0.0, fill);
    if theme.border_width > 0.0 {
        ui.painter()
            .rect_stroke(rect, 0.0, stroke, egui::StrokeKind::Inside);
    }
}

fn resize_shelf(
    ui: &mut egui::Ui,
    shelf: &ShelfDef<'_>,
    render_id: Id,
    state: &mut ShelfState,
    theme: &ShelfTheme,
    rect: Rect,
) -> egui::Response {
    let handle = resize_handle_rect(shelf.edge, rect, theme);
    let size_key = shelf.id.with(shelf.edge);
    let resp = ui.interact(handle, render_id.with("resize"), Sense::drag());
    if resp.drag_started() {
        let cur = state
            .edge_size(shelf.id, shelf.edge)
            .unwrap_or_else(|| shelf.default_extent_for(shelf.edge, theme));
        if let Some(pointer) = resp.interact_pointer_pos() {
            state
                .resize_starts
                .insert(size_key, ShelfResizeStart { size: cur, pointer });
        }
    }
    if let Some(start) = state.resize_starts.get(&size_key).copied() {
        let pointer_down = ui.ctx().input(|i| i.pointer.primary_down());
        let pointer = ui
            .ctx()
            .pointer_interact_pos()
            .or_else(|| resp.interact_pointer_pos());
        if pointer_down {
            let pointer = pointer.unwrap_or(start.pointer);
            let delta = pointer - start.pointer;
            let next = resized_shelf_extent(
                shelf.edge,
                start.size,
                delta,
                shelf.min_extent(theme),
                shelf.max_extent(theme),
            );
            state.set_edge_size(shelf.id, shelf.edge, next);
            ui.ctx().request_repaint();
        } else {
            state.resize_starts.remove(&size_key);
        }
    } else if resp.dragged() {
        let start = state
            .edge_size(shelf.id, shelf.edge)
            .unwrap_or_else(|| shelf.default_extent_for(shelf.edge, theme));
        let next = resized_shelf_extent(
            shelf.edge,
            start,
            resp.drag_delta(),
            shelf.min_extent(theme),
            shelf.max_extent(theme),
        );
        state.set_edge_size(shelf.id, shelf.edge, next);
        ui.ctx().request_repaint();
    }
    if resp.drag_stopped() {
        state.resize_starts.remove(&size_key);
    }
    resp
}

fn resized_shelf_extent(edge: ShelfEdge, start: f32, delta: Vec2, min: f32, max: f32) -> f32 {
    let raw_delta = match edge {
        ShelfEdge::Left => delta.x,
        ShelfEdge::Right => -delta.x,
        ShelfEdge::Bottom => -delta.y,
    };
    (start + raw_delta).clamp(min, max)
}

fn update_container_move_target_slot(
    viewport: &mut egui::Ui,
    shelf_id: Id,
    pane_id: Id,
    shelf_edge: ShelfEdge,
    horizontal_stack: bool,
    content_rect: Rect,
    state: &mut ShelfState,
) {
    let Some(drag) = state.container_move else {
        return;
    };
    if drag.target_edge != Some(shelf_edge) {
        return;
    }
    let snap = shelf_target_cache(viewport.ctx(), pane_id);
    let cursor = viewport
        .ctx()
        .pointer_interact_pos()
        .or_else(|| viewport.ctx().pointer_latest_pos())
        .unwrap_or(drag.cursor);
    let cursor_axis = if horizontal_stack { cursor.x } else { cursor.y };
    let target_slot = pane::compute_target(&snap, drag.container_id, cursor_axis, horizontal_stack);
    let target_size = container_move_ghost_size_for_edge(
        viewport.ctx(),
        drag.container_id,
        shelf_edge,
        content_rect,
    );
    state.update_container_move_target_slot(shelf_id, pane_id, target_slot, target_size);
}

fn shelf_pane_info_key(edge: ShelfEdge) -> Id {
    Id::new("frost_shelf_pane_info").with(edge)
}

fn publish_shelf_pane_info(ctx: &egui::Context, info: ShelfPaneInfo) {
    ctx.data_mut(|d| d.insert_temp(shelf_pane_info_key(info.edge), info));
}

fn shelf_pane_info(ctx: &egui::Context, edge: ShelfEdge) -> Option<ShelfPaneInfo> {
    ctx.data(|d| d.get_temp(shelf_pane_info_key(edge)))
}

fn clear_published_shelf_pane_infos(ctx: &egui::Context) {
    ctx.data_mut(|d| {
        for edge in [ShelfEdge::Left, ShelfEdge::Right, ShelfEdge::Bottom] {
            d.remove::<ShelfPaneInfo>(shelf_pane_info_key(edge));
        }
    });
}

fn external_container_gap_key(pane_id: Id) -> Id {
    pane_id.with("frost_shelf_external_container_gap")
}

fn mark_external_container_gap(ctx: &egui::Context, pane_id: Id) {
    ctx.data_mut(|d| d.insert_temp(external_container_gap_key(pane_id), true));
}

fn clear_external_container_gap(ctx: &egui::Context, pane_id: Id) {
    ctx.data_mut(|d| {
        d.remove::<bool>(external_container_gap_key(pane_id));
    });
}

fn external_container_gap_was_painted(ctx: &egui::Context, pane_id: Id) -> bool {
    ctx.data(|d| {
        d.get_temp::<bool>(external_container_gap_key(pane_id))
            .unwrap_or(false)
    })
}

fn update_container_move_target_from_published(ctx: &egui::Context, state: &mut ShelfState) {
    let Some(drag) = state.container_move else {
        return;
    };
    let Some(target_edge) = drag.target_edge else {
        return;
    };
    let Some(info) = shelf_pane_info(ctx, target_edge) else {
        state.clear_container_move_target_slot();
        return;
    };
    let cursor = ctx
        .pointer_interact_pos()
        .or_else(|| ctx.pointer_latest_pos())
        .unwrap_or(drag.cursor);
    if !info.screen_rect.expand(24.0).contains(cursor) {
        state.clear_container_move_target_slot();
        return;
    }
    let snap = shelf_target_cache(ctx, info.pane_id);
    let cursor_axis = if info.horizontal_stack {
        cursor.x
    } else {
        cursor.y
    };
    let target_slot =
        pane::compute_target(&snap, drag.container_id, cursor_axis, info.horizontal_stack);
    let target_size =
        container_move_ghost_size_for_edge(ctx, drag.container_id, info.edge, info.content_rect);
    state.update_container_move_target_slot(info.shelf_id, info.pane_id, target_slot, target_size);
}

fn should_suppress_source_container_gap(
    source_pane_drag: pane::DragState,
    container_move: Option<ShelfContainerMoveState>,
    pane_id: Id,
    shelf_edge: ShelfEdge,
    screen_shelf_rect: Rect,
    layout: ShelfLayout,
    pointer_cursor: Option<Pos2>,
) -> bool {
    let Some(dragged_id) = source_pane_drag.item else {
        return false;
    };

    if let Some(cursor) = pointer_cursor.or(source_pane_drag.cursor) {
        return container_move_target_for_cursor(cursor, screen_shelf_rect, layout, shelf_edge)
            .is_some();
    }

    container_move.is_some_and(|drag| {
        drag.container_id == dragged_id
            && drag.source_pane == pane_id
            && drag.source_edge == shelf_edge
            && drag.target_edge.is_some_and(|target| target != shelf_edge)
    })
}

fn should_paint_source_container_preview(
    source_pane_drag: pane::DragState,
    container_move: Option<ShelfContainerMoveState>,
    pane_id: Id,
    shelf_edge: ShelfEdge,
) -> bool {
    let Some(dragged_id) = source_pane_drag.item else {
        return false;
    };
    !container_move.is_some_and(|drag| {
        drag.container_id == dragged_id
            && drag.source_pane == pane_id
            && drag.source_edge == shelf_edge
            && drag.target_edge.is_some_and(|target| target != shelf_edge)
    })
}

fn should_render_external_container_gap(
    source_pane_drag: pane::DragState,
    container_move: Option<ShelfContainerMoveState>,
    shelf_edge: ShelfEdge,
    screen_shelf_rect: Rect,
    pointer_cursor: Option<Pos2>,
) -> bool {
    source_pane_drag.item.is_none()
        && container_move.is_some_and(|drag| {
            let cursor = pointer_cursor.unwrap_or(drag.cursor);
            // There is only one render group per edge. If the
            // container is hovering an already-rendered target edge,
            // synthesize the inline gap in that current edge group
            // even when `drag.target_shelf` still contains a stale
            // owner id from a previous frame. The published-pane
            // pass refreshes the real target owner before commit.
            drag.target_edge == Some(shelf_edge)
                && drag.source_edge != shelf_edge
                && screen_shelf_rect.expand(24.0).contains(cursor)
        })
}

fn source_shelf_gap_entry(
    ctx: &egui::Context,
    dragged_id: Id,
    shelf_edge: ShelfEdge,
    content_rect: Rect,
    entry: pane::RectEntry,
) -> pane::RectEntry {
    if content_rect.contains(entry.rect.center()) {
        return entry;
    }

    let size = container_move_ghost_size_for_edge(ctx, dragged_id, shelf_edge, content_rect)
        .min(content_rect.size());
    pane::RectEntry {
        rect: Rect::from_min_size(content_rect.min, size),
        ..entry
    }
}

fn finish_container_move_if_released(ctx: &egui::Context, state: &mut ShelfState) {
    if !ctx.input(|i| i.pointer.any_released()) {
        return;
    }
    let Some(drag) = state.container_move else {
        return;
    };
    commit_container_move(ctx, state, drag);
}

fn commit_container_move(
    ctx: &egui::Context,
    state: &mut ShelfState,
    drag: ShelfContainerMoveState,
) {
    let Some(target) = drag.target_edge else {
        pane::clear_drag(ctx, drag.source_pane);
        state.clear_container_move();
        return;
    };

    let target_shelf = drag.target_shelf.unwrap_or_else(|| {
        state
            .container_locations
            .get(&drag.container_id)
            .and_then(|location| location.shelf_id)
            .unwrap_or_else(|| detached_shelf_id(drag.source_shelf, drag.container_id))
    });
    state.set_container_location(drag.container_id, Some(target_shelf), target);
    let target_group_key = shelf_active_container_key_for(target_shelf, target);
    let source_group_key = shelf_active_container_key_for(drag.source_shelf, drag.source_edge);
    state.set_active_container(target_shelf, drag.container_id);
    state.set_active_container_for_group(target_group_key, drag.container_id);
    if target_shelf != drag.source_shelf
        && state.active_container(drag.source_shelf) == Some(drag.container_id)
    {
        state.active_containers.remove(&drag.source_shelf);
    }
    if target_group_key != source_group_key
        && state.active_container_for_group(source_group_key) == Some(drag.container_id)
    {
        state.active_containers.remove(&source_group_key);
    }
    if let (Some(target_pane), Some(target_slot)) = (drag.target_pane, drag.target_slot) {
        let defaults: Vec<Id> = shelf_target_cache(ctx, target_pane)
            .iter()
            .map(|entry| entry.id)
            .collect();
        let mut order = pane::section_order_for(ctx, target_pane, &defaults);
        order.retain(|cid| *cid != drag.container_id);
        let clamped = target_slot.min(order.len());
        order.insert(clamped, drag.container_id);
        pane::set_section_order(ctx, target_pane, order);
        pane::clear_drag(ctx, target_pane);
    }
    pane::clear_drag(ctx, drag.source_pane);
    state.clear_container_move();
}

fn should_cancel_no_target_container_release(cursor: Option<Pos2>, shelf_rect: Rect) -> bool {
    cursor.is_some_and(|pos| !shelf_rect.expand(24.0).contains(pos))
}

fn container_slot_ghost_rect_in(
    fallback_rect: Option<Rect>,
    snap: &[pane::RectEntry],
    drag: ShelfContainerMoveState,
    slot: usize,
    horizontal_stack: bool,
) -> Option<Rect> {
    let size = drag.container_size;
    let others: Vec<&pane::RectEntry> = snap
        .iter()
        .filter(|entry| entry.id != drag.container_id)
        .collect();
    if let Some(next) = others.get(slot) {
        let pos = pos2(next.rect.left(), next.rect.top());
        return Some(container_slot_ghost_rect_from_pos(
            fallback_rect,
            pos,
            size,
            horizontal_stack,
        ));
    }
    if let Some(last) = others.last() {
        let pos = if horizontal_stack {
            pos2(last.rect.right(), last.rect.top())
        } else {
            pos2(last.rect.left(), last.rect.bottom())
        };
        return Some(container_slot_ghost_rect_from_pos(
            fallback_rect,
            pos,
            size,
            horizontal_stack,
        ));
    }
    fallback_rect.map(|rect| Rect::from_min_size(rect.min, size.min(rect.size())))
}

fn container_slot_ghost_rect_from_pos(
    fallback_rect: Option<Rect>,
    pos: Pos2,
    size: Vec2,
    horizontal_stack: bool,
) -> Rect {
    let Some(bounds) = fallback_rect else {
        return Rect::from_min_size(pos, size);
    };
    let (clamped_size, min) = if horizontal_stack {
        let clamped_size = vec2(size.x, size.y.min(bounds.height()));
        let min = pos2(
            pos.x,
            pos.y.clamp(
                bounds.top(),
                (bounds.bottom() - clamped_size.y).max(bounds.top()),
            ),
        );
        (clamped_size, min)
    } else {
        let clamped_size = vec2(size.x.min(bounds.width()), size.y);
        let min = pos2(
            pos.x.clamp(
                bounds.left(),
                (bounds.right() - clamped_size.x).max(bounds.left()),
            ),
            pos.y,
        );
        (clamped_size, min)
    };
    Rect::from_min_size(min, clamped_size)
}

fn resize_handle_rect(edge: ShelfEdge, rect: Rect, theme: &ShelfTheme) -> Rect {
    let thickness = theme.resize_handle_thickness;
    match edge {
        ShelfEdge::Left => Rect::from_min_size(
            pos2(rect.max.x - thickness, rect.min.y),
            vec2(thickness, rect.height()),
        ),
        ShelfEdge::Right => Rect::from_min_size(rect.min, vec2(thickness, rect.height())),
        ShelfEdge::Bottom => Rect::from_min_size(rect.min, vec2(rect.width(), thickness)),
    }
}

struct ShelfMoveDragInput<'a, 'state> {
    ctx: &'a egui::Context,
    shelf_id: Id,
    shelf_edge: ShelfEdge,
    pane_id: Id,
    state: &'state mut ShelfState,
    layout: ShelfLayout,
    available: Rect,
    shelf_rect: Rect,
    response: &'a egui::Response,
}

fn handle_shelf_move_drag(input: ShelfMoveDragInput<'_, '_>) {
    let ShelfMoveDragInput {
        ctx,
        shelf_id,
        shelf_edge,
        pane_id,
        state,
        layout,
        available,
        shelf_rect,
        response,
    } = input;
    let _ = shelf_rect;
    if response.drag_started()
        && let Some(cursor) = response.interact_pointer_pos()
        && !pointer_over_shelf_container(ctx, pane_id, cursor)
        && !pane::pointer_over_container_dots(ctx, pane_id, cursor)
    {
        state.begin_drag(shelf_id, shelf_edge, cursor);
    }

    let dragging_this = state.drag.is_some_and(|drag| drag.shelf_id == shelf_id);
    if dragging_this {
        if let Some(cursor) = ctx
            .pointer_interact_pos()
            .or(response.interact_pointer_pos())
        {
            let occupied = occupied_edges_for_layout(layout, Some(shelf_edge));
            let target = shelf_move_target(cursor, available, occupied, shelf_edge);
            state.update_drag(cursor, target);
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            ctx.request_repaint();
        }
        if ctx.input(|i| i.pointer.any_released()) {
            state.finish_drag();
        }
    }
    if state.drag.is_some() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.cancel_drag();
    }
}

fn pointer_over_shelf_container(ctx: &egui::Context, pane_id: Id, pos: Pos2) -> bool {
    pane::snapshot(ctx, pane_id)
        .iter()
        .any(|entry| entry.frame.unwrap_or(entry.rect).contains(pos))
}

#[derive(Clone, Copy, Default)]
struct ShelfOccupied {
    left: bool,
    right: bool,
    bottom: bool,
}

impl ShelfOccupied {
    fn has(self, edge: ShelfEdge) -> bool {
        match edge {
            ShelfEdge::Left => self.left,
            ShelfEdge::Right => self.right,
            ShelfEdge::Bottom => self.bottom,
        }
    }
}

fn occupied_edges_for_layout(layout: ShelfLayout, exclude: Option<ShelfEdge>) -> ShelfOccupied {
    ShelfOccupied {
        left: layout.left.is_some() && exclude != Some(ShelfEdge::Left),
        right: layout.right.is_some() && exclude != Some(ShelfEdge::Right),
        bottom: layout.bottom.is_some() && exclude != Some(ShelfEdge::Bottom),
    }
}

fn shelf_move_target(
    cursor: Pos2,
    available: Rect,
    occupied: ShelfOccupied,
    source: ShelfEdge,
) -> Option<ShelfEdge> {
    if !available.contains(cursor) {
        return None;
    }
    let distances = [
        (ShelfEdge::Left, (cursor.x - available.left()).abs()),
        (ShelfEdge::Right, (available.right() - cursor.x).abs()),
        (ShelfEdge::Bottom, (available.bottom() - cursor.y).abs()),
    ];
    let edge_band = (available.width().min(available.height()) * 0.28).max(96.0);
    distances
        .into_iter()
        .filter(|(edge, dist)| *edge != source && !occupied.has(*edge) && *dist <= edge_band)
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(edge, _)| edge)
}

#[cfg(test)]
fn container_move_target(cursor: Pos2, available: Rect, source: ShelfEdge) -> Option<ShelfEdge> {
    if !available.contains(cursor) {
        return None;
    }
    let distances = [
        (ShelfEdge::Left, (cursor.x - available.left()).abs()),
        (ShelfEdge::Right, (available.right() - cursor.x).abs()),
        (ShelfEdge::Bottom, (available.bottom() - cursor.y).abs()),
    ];
    let edge_band = (available.width().min(available.height()) * 0.28).max(96.0);
    distances
        .into_iter()
        .filter(|(edge, dist)| *edge != source && *dist <= edge_band)
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(edge, _)| edge)
}

fn container_move_target_for_cursor(
    cursor: Pos2,
    screen_shelf_rect: Rect,
    layout: ShelfLayout,
    source: ShelfEdge,
) -> Option<ShelfEdge> {
    if screen_shelf_rect.expand(24.0).contains(cursor) {
        return None;
    }

    for edge in [ShelfEdge::Left, ShelfEdge::Right, ShelfEdge::Bottom] {
        if edge == source {
            continue;
        }
        if let Some(rect) = layout.rect_for(edge)
            && rect.expand(24.0).contains(cursor)
        {
            return Some(edge);
        }
    }

    container_move_empty_edge_target(cursor, layout, source)
}

fn container_move_empty_edge_target(
    cursor: Pos2,
    layout: ShelfLayout,
    source: ShelfEdge,
) -> Option<ShelfEdge> {
    let available = layout.available();
    if !available.contains(cursor) {
        return None;
    }
    let distances = [
        (ShelfEdge::Left, (cursor.x - available.left()).abs()),
        (ShelfEdge::Right, (available.right() - cursor.x).abs()),
        (ShelfEdge::Bottom, (available.bottom() - cursor.y).abs()),
    ];
    let edge_band = (available.width().min(available.height()) * 0.12).clamp(48.0, 96.0);
    distances
        .into_iter()
        .filter(|(edge, dist)| {
            *edge != source && layout.rect_for(*edge).is_none() && *dist <= edge_band
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(edge, _)| edge)
}

fn paint_shelf_move_ghost(
    ctx: &egui::Context,
    layout: ShelfLayout,
    state: &ShelfState,
    theme: &ShelfTheme,
) {
    let Some(drag) = state.drag else {
        return;
    };
    let Some(target) = drag.target_edge else {
        return;
    };
    let Some(rect) = shelf_drop_rect(layout, drag.source_edge, target, theme) else {
        return;
    };

    egui::Area::new(egui::Id::new("frost_shelf_move_ghost"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .interactable(false)
        .show(ctx, |ui| {
            let (local, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
            ui.painter().rect(
                local,
                0.0,
                style::fill_for(style::FillRole::DragGhost, style::active_accent()),
                style::stroke_for(style::StrokeRole::DragGhost, style::active_accent()),
                egui::StrokeKind::Inside,
            );
        });
}

fn paint_container_move_ghost(
    ctx: &egui::Context,
    layout: ShelfLayout,
    state: &ShelfState,
    theme: &ShelfTheme,
) {
    let Some(drag) = state.container_move else {
        return;
    };
    let Some(target) = drag.target_edge else {
        return;
    };
    if let Some((rect, accent)) = existing_shelf_container_slot_ghost(ctx, target, drag) {
        egui::Area::new(egui::Id::new("frost_shelf_existing_container_slot_ghost"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .interactable(false)
            .show(ctx, |ui| {
                let (local, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
                ui.painter().rect(
                    local,
                    egui::CornerRadius::same(style::theme().radius_md),
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 72),
                    Stroke::new(1.5, accent),
                    egui::StrokeKind::Inside,
                );
            });
        return;
    }
    let Some(shelf_rect) = container_drop_rect(layout, drag.source_edge, target, theme) else {
        return;
    };
    let accent = style::active_accent();
    egui::Area::new(egui::Id::new("frost_shelf_container_move_ghost"))
        .order(egui::Order::Foreground)
        .fixed_pos(shelf_rect.min)
        .interactable(false)
        .show(ctx, |ui| {
            let (shelf_local, _) = ui.allocate_exact_size(shelf_rect.size(), Sense::hover());
            ui.painter().rect(
                shelf_local,
                0.0,
                style::fill_for(style::FillRole::DragGhost, accent),
                style::stroke_for(style::StrokeRole::DragGhost, accent),
                egui::StrokeKind::Inside,
            );

            let container_rect =
                new_shelf_container_ghost_rect(ctx, drag.container_id, target, shelf_local);
            ui.painter().rect(
                container_rect,
                egui::CornerRadius::same(style::theme().radius_md),
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 72),
                Stroke::new(1.5, accent),
                egui::StrokeKind::Inside,
            );
        });
}

fn existing_shelf_container_slot_ghost(
    ctx: &egui::Context,
    target: ShelfEdge,
    drag: ShelfContainerMoveState,
) -> Option<(Rect, Color32)> {
    let target_pane = drag.target_pane?;
    let target_slot = drag.target_slot?;
    if external_container_gap_was_painted(ctx, target_pane) {
        return None;
    }
    let info = shelf_pane_info(ctx, target)?;
    if target_pane != info.pane_id {
        return None;
    }
    let snap = shelf_target_cache(ctx, target_pane);
    container_slot_ghost_rect_in(
        Some(info.content_rect),
        &snap,
        drag,
        target_slot,
        info.horizontal_stack,
    )
    .map(|rect| (rect.translate(info.screen_offset), info.accent))
}

fn shelf_target_cache(ctx: &egui::Context, pane_id: Id) -> Vec<pane::RectEntry> {
    pane::target_cache(ctx, pane_id)
}

fn new_shelf_container_ghost_rect(
    ctx: &egui::Context,
    container_id: Id,
    target: ShelfEdge,
    shelf_rect: Rect,
) -> Rect {
    let content_rect = shelf_rect.shrink(style::theme().shelf().padding);
    let container_size =
        container_move_ghost_size_for_edge(ctx, container_id, target, content_rect)
            .min(content_rect.size());
    Rect::from_min_size(content_rect.min, container_size)
}

fn container_move_ghost_size_for_edge(
    ctx: &egui::Context,
    container_id: Id,
    edge: ShelfEdge,
    content_rect: Rect,
) -> Vec2 {
    let anchor = edge.container_anchor();
    let horizontal_stack = !anchor.title_side().is_horizontal_strip();
    let pane_horizontal_strip = anchor.title_side().is_horizontal_strip();
    let flow = crate::container::container_flow(ctx, container_id, pane_horizontal_strip);
    let title = style::theme().container().title_zone_thickness;
    if horizontal_stack {
        vec2(flow + title, content_rect.height())
    } else {
        vec2(content_rect.width(), flow + title)
    }
}

fn shelf_drop_rect(
    layout: ShelfLayout,
    source: ShelfEdge,
    target: ShelfEdge,
    theme: &ShelfTheme,
) -> Option<Rect> {
    let occupied = occupied_edges_for_layout(layout, Some(source));
    if occupied.has(target) {
        return None;
    }
    Some(drop_rect_for_occupied_edges(
        layout,
        target,
        drop_extent_for(layout, source, target, theme),
        occupied,
    ))
}

fn container_drop_rect(
    layout: ShelfLayout,
    source: ShelfEdge,
    target: ShelfEdge,
    theme: &ShelfTheme,
) -> Option<Rect> {
    layout.rect_for(target).or_else(|| {
        let occupied = occupied_edges_for_layout(layout, None);
        if occupied.has(target) {
            return None;
        }
        Some(drop_rect_for_occupied_edges(
            layout,
            target,
            drop_extent_for(layout, source, target, theme),
            occupied,
        ))
    })
}

fn drop_extent_for(
    layout: ShelfLayout,
    source: ShelfEdge,
    target: ShelfEdge,
    theme: &ShelfTheme,
) -> f32 {
    if source.is_side() == target.is_side() {
        source_extent_for(layout, source, theme)
    } else if target.is_side() {
        theme.side_default_size
    } else {
        theme.bottom_default_size
    }
}

fn source_extent_for(layout: ShelfLayout, source: ShelfEdge, theme: &ShelfTheme) -> f32 {
    match source {
        ShelfEdge::Left => layout
            .left
            .map(|rect| rect.width())
            .unwrap_or(theme.side_default_size),
        ShelfEdge::Right => layout
            .right
            .map(|rect| rect.width())
            .unwrap_or(theme.side_default_size),
        ShelfEdge::Bottom => layout
            .bottom
            .map(|rect| rect.height())
            .unwrap_or(theme.bottom_default_size),
    }
}

fn drop_rect_for_occupied_edges(
    layout: ShelfLayout,
    target: ShelfEdge,
    extent: f32,
    occupied: ShelfOccupied,
) -> Rect {
    let available = layout.available();
    let left_w = if occupied.left {
        layout.left.map_or(0.0, |rect| rect.width())
    } else {
        0.0
    };
    let right_w = if occupied.right {
        layout.right.map_or(0.0, |rect| rect.width())
    } else {
        0.0
    };

    match target {
        ShelfEdge::Left => Rect::from_min_max(
            available.min,
            pos2(
                (available.left() + extent).min(available.right()),
                available.bottom(),
            ),
        ),
        ShelfEdge::Right => Rect::from_min_max(
            pos2(
                (available.right() - extent).max(available.left()),
                available.top(),
            ),
            pos2(available.right(), available.bottom()),
        ),
        ShelfEdge::Bottom => Rect::from_min_max(
            pos2(available.left() + left_w, available.bottom() - extent),
            pos2(available.right() - right_w, available.bottom()),
        ),
    }
}

/// Shelf-reserved insets, useful for ribbon/pane placement code.
#[must_use]
pub fn shelf_insets(layout: ShelfLayout) -> Vec2 {
    vec2(
        layout.left.map_or(0.0, |r| r.width()) + layout.right.map_or(0.0, |r| r.width()),
        layout.bottom.map_or(0.0, |r| r.height()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tabs() -> Vec<Tab> {
        vec![Tab::new("test.tab", "Tab", "box")]
    }

    #[test]
    fn container_move_preserves_existing_shelf_slot_while_drag_continues() {
        let mut state = ShelfState::default();
        let container_id = Id::new("dragged");
        let pane_id = Id::new("target-pane");

        state.update_container_move(ShelfContainerMoveUpdate {
            container_id,
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Left,
            cursor: pos2(10.0, 10.0),
            target_edge: Some(ShelfEdge::Right),
            container_size: vec2(120.0, 240.0),
        });
        state.update_container_move_target_slot(
            Id::new("target-shelf"),
            pane_id,
            2,
            vec2(120.0, 240.0),
        );

        state.update_container_move(ShelfContainerMoveUpdate {
            container_id,
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Left,
            cursor: pos2(20.0, 20.0),
            target_edge: Some(ShelfEdge::Right),
            container_size: vec2(120.0, 240.0),
        });

        let drag = state
            .container_move
            .expect("container drag should continue");
        assert_eq!(drag.target_pane, Some(pane_id));
        assert_eq!(drag.target_slot, Some(2));
    }

    #[test]
    fn container_move_target_slot_adopts_target_shelf_size() {
        let mut state = ShelfState::default();
        let container_id = Id::new("dragged");
        let pane_id = Id::new("target-pane");

        state.update_container_move(ShelfContainerMoveUpdate {
            container_id,
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Left,
            cursor: pos2(10.0, 10.0),
            target_edge: Some(ShelfEdge::Bottom),
            container_size: vec2(120.0, 240.0),
        });
        state.update_container_move_target_slot(
            Id::new("target-shelf"),
            pane_id,
            1,
            vec2(360.0, 96.0),
        );

        let drag = state
            .container_move
            .expect("container drag should be tracked");
        assert_eq!(drag.target_slot, Some(1));
        assert_eq!(drag.container_size, vec2(360.0, 96.0));
    }

    #[test]
    fn external_container_gap_flag_is_frame_local() {
        let ctx = egui::Context::default();
        let pane_id = Id::new("target-pane");

        mark_external_container_gap(&ctx, pane_id);
        assert!(external_container_gap_was_painted(&ctx, pane_id));

        clear_external_container_gap(&ctx, pane_id);
        assert!(!external_container_gap_was_painted(&ctx, pane_id));
    }

    #[test]
    fn published_shelf_pane_info_is_cleared_before_shelf_render() {
        let ctx = egui::Context::default();
        let info = ShelfPaneInfo {
            shelf_id: Id::new("stale-shelf"),
            pane_id: Id::new("stale-pane"),
            edge: ShelfEdge::Left,
            horizontal_stack: false,
            content_rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0)),
            screen_rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0)),
            screen_offset: Vec2::ZERO,
            accent: Color32::WHITE,
        };

        publish_shelf_pane_info(&ctx, info);
        assert_eq!(shelf_pane_info(&ctx, ShelfEdge::Left), Some(info));

        clear_published_shelf_pane_infos(&ctx);
        assert_eq!(shelf_pane_info(&ctx, ShelfEdge::Left), None);
    }

    #[test]
    fn publish_shelf_layout_sets_chrome_bounds_to_reserved_viewport() {
        let ctx = egui::Context::default();
        let viewport = Rect::from_min_max(pos2(200.0, 0.0), pos2(900.0, 640.0));

        publish_shelf_layout(
            &ctx,
            ShelfLayout {
                viewport,
                left: Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 640.0))),
                right: None,
                bottom: Some(Rect::from_min_max(pos2(200.0, 640.0), pos2(900.0, 800.0))),
            },
        );

        let chrome = ctx
            .data(|d| d.get_temp::<Rect>(crate::ribbon::chrome::chrome_bounds_key()))
            .expect("shelf layout should publish ribbon chrome bounds");
        assert_eq!(chrome, viewport);
    }

    #[test]
    fn show_shelves_sets_public_active_container_for_default_visible_container() {
        let ctx = egui::Context::default();
        let shelf_id = Id::new("active-shelf");
        let container_id = Id::new("visible-container");
        let theme = *style::theme().shelf();
        let available = Rect::from_min_size(pos2(0.0, 0.0), vec2(640.0, 480.0));
        let shelves = vec![
            ShelfDef::new(shelf_id, ShelfEdge::Left, Color32::WHITE)
                .default_size(220.0)
                .container(ShelfContainer::tabbed(
                    container_id,
                    "Visible",
                    "box",
                    test_tabs(),
                )),
        ];
        let mut state = ShelfState::default();
        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(available),
            ..Default::default()
        });
        show_shelves(&ctx, layout, shelves, &mut state);
        let _ = ctx.end_pass();

        assert_eq!(
            state.active_container(shelf_id),
            Some(container_id),
            "the public shelf state should mirror the visible default active container"
        );
    }

    #[test]
    fn show_shelves_repairs_stale_public_active_container_from_rendered_group() {
        let ctx = egui::Context::default();
        let shelf_id = Id::new("active-shelf");
        let visible_container = Id::new("visible-container");
        let stale_container = Id::new("removed-container");
        let edge = ShelfEdge::Left;
        let theme = *style::theme().shelf();
        let available = Rect::from_min_size(pos2(0.0, 0.0), vec2(640.0, 480.0));
        let shelves = vec![
            ShelfDef::new(shelf_id, edge, Color32::WHITE)
                .default_size(220.0)
                .container(ShelfContainer::tabbed(
                    visible_container,
                    "Visible",
                    "box",
                    test_tabs(),
                )),
        ];
        let mut state = ShelfState::default();
        state.set_active_container(shelf_id, stale_container);
        state.set_active_container_for_group(
            shelf_active_container_key_for(shelf_id, edge),
            visible_container,
        );
        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(available),
            ..Default::default()
        });
        show_shelves(&ctx, layout, shelves, &mut state);
        let _ = ctx.end_pass();

        assert_eq!(
            state.active_container(shelf_id),
            Some(visible_container),
            "public shelf active state should be repaired from the visible rendered group"
        );
    }

    #[test]
    fn show_shelves_clears_active_container_when_no_container_is_visible() {
        let ctx = egui::Context::default();
        let shelf_id = Id::new("empty-shelf");
        let stale_container = Id::new("removed-container");
        let edge = ShelfEdge::Left;
        let theme = *style::theme().shelf();
        let available = Rect::from_min_size(pos2(0.0, 0.0), vec2(640.0, 480.0));
        let shelves = vec![ShelfDef::new(shelf_id, edge, Color32::WHITE).default_size(220.0)];
        let mut state = ShelfState::default();
        state.set_active_container(shelf_id, stale_container);
        state.set_active_container_for_group(
            shelf_active_container_key_for(shelf_id, edge),
            stale_container,
        );
        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(available),
            ..Default::default()
        });
        show_shelves(&ctx, layout, shelves, &mut state);
        let _ = ctx.end_pass();

        assert_eq!(
            state.active_container(shelf_id),
            None,
            "empty shelves must not keep stale public active-container state"
        );
        assert_eq!(
            state.active_container_for_group(shelf_active_container_key_for(shelf_id, edge)),
            None,
            "empty rendered shelf groups must not keep stale active-container state"
        );
    }

    #[test]
    fn commit_container_move_inserts_into_target_pane_order() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("target-pane");
        let source_pane = Id::new("source-pane");
        let target_shelf = Id::new("target-shelf");
        let source_shelf = Id::new("source-shelf");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let mut state = ShelfState::default();
        state.set_active_container(source_shelf, dragged);
        state.set_active_container_for_group(
            shelf_active_container_key_for(source_shelf, ShelfEdge::Left),
            dragged,
        );
        pane::set_drag(
            &ctx,
            target_pane,
            pane::DragState {
                item: Some(dragged),
                cursor: Some(pos2(0.0, 0.0)),
            },
        );
        pane::set_drag(
            &ctx,
            source_pane,
            pane::DragState {
                item: Some(dragged),
                cursor: Some(pos2(0.0, 0.0)),
            },
        );
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(0.0, 90.0), vec2(100.0, 80.0)),
                    frame: None,
                },
            ],
        );

        commit_container_move(
            &ctx,
            &mut state,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf,
                source_pane,
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Right),
                target_shelf: Some(target_shelf),
                target_pane: Some(target_pane),
                target_slot: Some(1),
                container_size: vec2(100.0, 80.0),
            },
        );

        assert_eq!(
            state.container_edge(dragged, ShelfEdge::Left),
            ShelfEdge::Right
        );
        assert_eq!(
            state.container_location(dragged, ShelfEdge::Left),
            ShelfContainerLocation {
                shelf_id: Some(target_shelf),
                edge: ShelfEdge::Right,
            },
            "committing into an existing shelf should adopt that shelf owner, not only the edge"
        );
        assert_eq!(
            state.active_container(target_shelf),
            Some(dragged),
            "the receiving shelf should select the container that was just moved into it"
        );
        assert_eq!(
            state.active_container_for_group(shelf_active_container_key_for(
                target_shelf,
                ShelfEdge::Right
            )),
            Some(dragged),
            "the receiving rendered shelf group should select the moved container immediately"
        );
        assert_eq!(
            state.active_container(source_shelf),
            None,
            "the source shelf must not keep a moved-away container as its public active container"
        );
        assert_eq!(
            state.active_container_for_group(shelf_active_container_key_for(
                source_shelf,
                ShelfEdge::Left
            )),
            None,
            "the source rendered shelf group must not keep a moved-away container selected"
        );
        assert_eq!(
            pane::section_order_for(&ctx, target_pane, &[first, dragged, second]),
            vec![first, dragged, second]
        );
        assert!(
            pane::drag_state(&ctx, target_pane).item.is_none(),
            "committing into a target shelf should clear target-pane drag state so the ghost cannot stick"
        );
        assert!(
            pane::drag_state(&ctx, source_pane).item.is_none(),
            "committing into a target shelf should also clear source-pane drag state"
        );
    }

    #[test]
    fn commit_container_move_inserts_into_bottom_target_order() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("bottom-target-pane");
        let target_shelf = Id::new("bottom-target-shelf");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let third = Id::new("third");
        let mut state = ShelfState::default();
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(120.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(140.0, 0.0), vec2(120.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: third,
                    rect: Rect::from_min_size(pos2(280.0, 0.0), vec2(120.0, 80.0)),
                    frame: None,
                },
            ],
        );

        commit_container_move(
            &ctx,
            &mut state,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Bottom),
                target_shelf: Some(target_shelf),
                target_pane: Some(target_pane),
                target_slot: Some(2),
                container_size: vec2(120.0, 80.0),
            },
        );

        assert_eq!(
            state.container_location(dragged, ShelfEdge::Left),
            ShelfContainerLocation {
                shelf_id: Some(target_shelf),
                edge: ShelfEdge::Bottom,
            }
        );
        assert_eq!(
            pane::section_order_for(&ctx, target_pane, &[first, second, dragged, third]),
            vec![first, second, dragged, third]
        );
    }

    #[test]
    fn commit_container_move_clamps_oversized_target_slot_to_end() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("target-pane");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let mut state = ShelfState::default();
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(0.0, 90.0), vec2(100.0, 80.0)),
                    frame: None,
                },
            ],
        );

        commit_container_move(
            &ctx,
            &mut state,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Right),
                target_shelf: Some(Id::new("target-shelf")),
                target_pane: Some(target_pane),
                target_slot: Some(usize::MAX),
                container_size: vec2(100.0, 80.0),
            },
        );

        assert_eq!(
            pane::section_order_for(&ctx, target_pane, &[first, second, dragged]),
            vec![first, second, dragged],
            "a stale/oversized slot should clamp to the end instead of corrupting order"
        );
    }

    #[test]
    fn commit_container_move_deduplicates_existing_target_order_entry() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("target-pane");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let mut state = ShelfState::default();
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: dragged,
                    rect: Rect::from_min_size(pos2(0.0, 90.0), vec2(100.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(0.0, 180.0), vec2(100.0, 80.0)),
                    frame: None,
                },
            ],
        );
        pane::set_section_order(&ctx, target_pane, vec![first, dragged, second]);

        commit_container_move(
            &ctx,
            &mut state,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Right),
                target_shelf: Some(Id::new("target-shelf")),
                target_pane: Some(target_pane),
                target_slot: Some(0),
                container_size: vec2(100.0, 80.0),
            },
        );

        assert_eq!(
            pane::section_order_for(&ctx, target_pane, &[dragged, first, second]),
            vec![dragged, first, second],
            "moving an already-known container should reposition it without duplicating the id"
        );
    }

    #[test]
    fn commit_container_move_uses_live_target_cache_for_trailing_slot() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("target-pane");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let third = Id::new("third");
        let mut state = ShelfState::default();
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(0.0, 90.0), vec2(100.0, 80.0)),
                    frame: None,
                },
            ],
        );
        pane::push_rect(
            &ctx,
            target_pane,
            first,
            Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0)),
        );
        pane::push_rect(
            &ctx,
            target_pane,
            second,
            Rect::from_min_size(pos2(0.0, 90.0), vec2(100.0, 80.0)),
        );
        pane::push_rect(
            &ctx,
            target_pane,
            third,
            Rect::from_min_size(pos2(0.0, 180.0), vec2(100.0, 80.0)),
        );

        commit_container_move(
            &ctx,
            &mut state,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Right),
                target_shelf: Some(Id::new("target-shelf")),
                target_pane: Some(target_pane),
                target_slot: Some(3),
                container_size: vec2(100.0, 80.0),
            },
        );

        assert_eq!(
            pane::section_order_for(&ctx, target_pane, &[first, second, third, dragged]),
            vec![first, second, third, dragged],
            "committing after the last live-rendered container must not fall back to a stale shorter snapshot"
        );
    }

    #[test]
    fn same_shelf_reorder_uses_live_cache_for_trailing_slot() {
        let ctx = egui::Context::default();
        let pane_id = Id::new("shelf-pane");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let third = Id::new("third");
        pane::set_snapshot(
            &ctx,
            pane_id,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(0.0, 90.0), vec2(100.0, 80.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: dragged,
                    rect: Rect::from_min_size(pos2(0.0, 180.0), vec2(100.0, 80.0)),
                    frame: None,
                },
            ],
        );
        pane::set_section_order(&ctx, pane_id, vec![first, second, dragged]);
        pane::push_rect(
            &ctx,
            pane_id,
            first,
            Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0)),
        );
        pane::push_rect(
            &ctx,
            pane_id,
            second,
            Rect::from_min_size(pos2(0.0, 90.0), vec2(100.0, 80.0)),
        );
        pane::push_rect(
            &ctx,
            pane_id,
            third,
            Rect::from_min_size(pos2(0.0, 180.0), vec2(100.0, 80.0)),
        );

        commit_shelf_container_reorder(&ctx, pane_id, dragged, 260.0, false);

        assert_eq!(
            pane::section_order_for(&ctx, pane_id, &[first, second, third, dragged]),
            vec![first, second, third, dragged],
            "same-shelf reorder commit must not drop live-rendered containers that were absent from the stale snapshot"
        );
    }

    #[test]
    fn commit_adopted_container_to_new_edge_keeps_current_shelf_owner() {
        let ctx = egui::Context::default();
        let adopted_shelf = Id::new("adopted-shelf");
        let original_shelf = Id::new("original-shelf");
        let dragged = Id::new("dragged");
        let mut state = ShelfState::default();
        state.set_container_location(dragged, Some(adopted_shelf), ShelfEdge::Right);

        commit_container_move(
            &ctx,
            &mut state,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: adopted_shelf,
                source_pane: Id::new("adopted-pane"),
                source_edge: ShelfEdge::Right,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Bottom),
                target_shelf: None,
                target_pane: None,
                target_slot: None,
                container_size: vec2(120.0, 80.0),
            },
        );

        assert_eq!(
            state.container_location(dragged, ShelfEdge::Left),
            ShelfContainerLocation {
                shelf_id: Some(adopted_shelf),
                edge: ShelfEdge::Bottom,
            },
            "moving an already-adopted container to a new edge should keep the shelf it was dragged from"
        );

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(original_shelf, ShelfEdge::Left, Color32::WHITE)
                    .container(ShelfContainer::tabbed(dragged, "Moved", "box", test_tabs())),
                ShelfDef::new(adopted_shelf, ShelfEdge::Right, Color32::WHITE),
            ],
            &state,
        );

        assert!(groups.iter().any(|group| {
            group.id == adopted_shelf
                && group.edge == ShelfEdge::Bottom
                && group
                    .containers
                    .iter()
                    .any(|container| container.spec.container_id() == dragged)
        }));
    }

    #[test]
    fn missing_published_target_clears_stale_container_move_slot() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("old-target-pane");
        let mut state = ShelfState {
            container_move: Some(ShelfContainerMoveState {
                container_id: Id::new("dragged"),
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Right),
                target_shelf: Some(Id::new("old-target-shelf")),
                target_pane: Some(target_pane),
                target_slot: Some(2),
                container_size: vec2(100.0, 80.0),
            }),
            ..Default::default()
        };

        clear_published_shelf_pane_infos(&ctx);
        update_container_move_target_from_published(&ctx, &mut state);

        let drag = state
            .container_move
            .expect("container move should remain active");
        assert_eq!(drag.target_edge, Some(ShelfEdge::Right));
        assert_eq!(drag.target_shelf, None);
        assert_eq!(drag.target_pane, None);
        assert_eq!(drag.target_slot, None);
    }

    #[test]
    fn published_target_clears_slot_when_cursor_left_target_shelf_rect() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("old-target-pane");
        let mut state = ShelfState {
            container_move: Some(ShelfContainerMoveState {
                container_id: Id::new("dragged"),
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Right,
                cursor: pos2(300.0, 200.0),
                target_edge: Some(ShelfEdge::Left),
                target_shelf: Some(Id::new("old-target-shelf")),
                target_pane: Some(target_pane),
                target_slot: Some(2),
                container_size: vec2(100.0, 80.0),
            }),
            ..Default::default()
        };
        publish_shelf_pane_info(
            &ctx,
            ShelfPaneInfo {
                shelf_id: Id::new("left-shelf"),
                pane_id: target_pane,
                edge: ShelfEdge::Left,
                horizontal_stack: false,
                content_rect: Rect::from_min_size(pos2(8.0, 8.0), vec2(120.0, 400.0)),
                screen_rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(140.0, 420.0)),
                screen_offset: Vec2::ZERO,
                accent: Color32::WHITE,
            },
        );

        update_container_move_target_from_published(&ctx, &mut state);

        let drag = state
            .container_move
            .expect("container move should remain active");
        assert_eq!(drag.target_edge, Some(ShelfEdge::Left));
        assert_eq!(
            drag.target_pane, None,
            "stale target shelf slots must be cleared when the cursor is in the canvas, not over the shelf"
        );
        assert_eq!(drag.target_slot, None);
    }

    #[test]
    fn published_existing_shelf_target_tracks_middle_container_slot() {
        let ctx = egui::Context::default();
        let target_shelf = Id::new("target-shelf");
        let target_pane = Id::new("target-pane");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let third = Id::new("third");
        let mut state = ShelfState {
            container_move: Some(ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(120.0, 150.0),
                target_edge: Some(ShelfEdge::Right),
                target_shelf: None,
                target_pane: None,
                target_slot: None,
                container_size: vec2(80.0, 120.0),
            }),
            ..Default::default()
        };
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(100.0, 20.0), vec2(80.0, 100.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(100.0, 140.0), vec2(80.0, 100.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: third,
                    rect: Rect::from_min_size(pos2(100.0, 260.0), vec2(80.0, 100.0)),
                    frame: None,
                },
            ],
        );
        publish_shelf_pane_info(
            &ctx,
            ShelfPaneInfo {
                shelf_id: target_shelf,
                pane_id: target_pane,
                edge: ShelfEdge::Right,
                horizontal_stack: false,
                content_rect: Rect::from_min_size(pos2(96.0, 16.0), vec2(120.0, 400.0)),
                screen_rect: Rect::from_min_size(pos2(96.0, 16.0), vec2(120.0, 400.0)),
                screen_offset: Vec2::ZERO,
                accent: Color32::WHITE,
            },
        );

        update_container_move_target_from_published(&ctx, &mut state);

        let drag = state
            .container_move
            .expect("container move should keep tracking target shelf");
        assert_eq!(drag.target_shelf, Some(target_shelf));
        assert_eq!(drag.target_pane, Some(target_pane));
        assert_eq!(
            drag.target_slot,
            Some(1),
            "cursor between first and second target containers should place the ghost in the middle"
        );
    }

    #[test]
    fn published_existing_shelf_target_prefers_live_rendered_container_positions() {
        let ctx = egui::Context::default();
        let target_shelf = Id::new("target-shelf");
        let target_pane = Id::new("target-pane");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let third = Id::new("third");
        let mut state = ShelfState {
            container_move: Some(ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Right,
                cursor: pos2(120.0, 250.0),
                target_edge: Some(ShelfEdge::Left),
                target_shelf: None,
                target_pane: None,
                target_slot: None,
                container_size: vec2(80.0, 120.0),
            }),
            ..Default::default()
        };
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(100.0, 20.0), vec2(80.0, 100.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(100.0, 140.0), vec2(80.0, 100.0)),
                    frame: None,
                },
            ],
        );
        pane::push_rect(
            &ctx,
            target_pane,
            first,
            Rect::from_min_size(pos2(100.0, 20.0), vec2(80.0, 100.0)),
        );
        pane::push_rect(
            &ctx,
            target_pane,
            second,
            Rect::from_min_size(pos2(100.0, 140.0), vec2(80.0, 100.0)),
        );
        pane::push_rect(
            &ctx,
            target_pane,
            third,
            Rect::from_min_size(pos2(100.0, 260.0), vec2(80.0, 100.0)),
        );
        publish_shelf_pane_info(
            &ctx,
            ShelfPaneInfo {
                shelf_id: target_shelf,
                pane_id: target_pane,
                edge: ShelfEdge::Left,
                horizontal_stack: false,
                content_rect: Rect::from_min_size(pos2(96.0, 16.0), vec2(120.0, 400.0)),
                screen_rect: Rect::from_min_size(pos2(96.0, 16.0), vec2(120.0, 400.0)),
                screen_offset: Vec2::ZERO,
                accent: Color32::WHITE,
            },
        );

        update_container_move_target_from_published(&ctx, &mut state);

        let drag = state
            .container_move
            .expect("container move should keep tracking the live target shelf");
        assert_eq!(
            drag.target_slot,
            Some(2),
            "existing-shelf drops must use this frame's live rendered positions, not a stale two-container snapshot"
        );
        let (rect, _) = existing_shelf_container_slot_ghost(&ctx, ShelfEdge::Left, drag)
            .expect("existing shelf slot ghost should use the live target slot");
        assert_eq!(
            rect.min,
            pos2(96.0, 260.0),
            "the foreground ghost should keep the target slot's main-axis position while filling the shelf cross-axis"
        );
    }

    #[test]
    fn existing_shelf_slot_ghost_translates_local_shelf_rects_to_screen_space() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("target-pane");
        let dragged = Id::new("dragged");
        let target = Id::new("target");
        let screen_offset = vec2(480.0, 360.0);
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![pane::RectEntry {
                id: target,
                rect: Rect::from_min_size(pos2(96.0, 140.0), vec2(80.0, 100.0)),
                frame: None,
            }],
        );
        publish_shelf_pane_info(
            &ctx,
            ShelfPaneInfo {
                shelf_id: Id::new("target-shelf"),
                pane_id: target_pane,
                edge: ShelfEdge::Right,
                horizontal_stack: false,
                content_rect: Rect::from_min_size(pos2(80.0, 16.0), vec2(120.0, 260.0)),
                screen_rect: Rect::from_min_size(pos2(560.0, 376.0), vec2(120.0, 260.0)),
                screen_offset,
                accent: Color32::WHITE,
            },
        );

        let (rect, _) = existing_shelf_container_slot_ghost(
            &ctx,
            ShelfEdge::Right,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Right),
                target_shelf: Some(Id::new("target-shelf")),
                target_pane: Some(target_pane),
                target_slot: Some(0),
                container_size: vec2(80.0, 100.0),
            },
        )
        .expect("existing shelf slot ghost should be computed");

        assert_eq!(
            rect.min,
            pos2(576.0, 500.0),
            "foreground ghost areas are positioned in screen space, so local shelf geometry must be translated by the shelf area's screen offset"
        );
    }

    #[test]
    fn container_move_target_stays_in_source_shelf_when_cursor_is_inside_screen_shelf_rect() {
        let available = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let layout = ShelfLayout {
            viewport: available,
            left: None,
            right: None,
            bottom: None,
        };
        let local_shelf = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 600.0));
        let screen_shelf = local_shelf.translate(vec2(500.0, 0.0));
        let cursor = pos2(690.0, 120.0);

        assert_eq!(
            container_move_target(cursor, available, ShelfEdge::Left),
            Some(ShelfEdge::Right),
            "without source-shelf containment, this cursor is close enough to the right edge to start an external move"
        );
        assert_eq!(
            container_move_target_for_cursor(cursor, screen_shelf, layout, ShelfEdge::Left),
            None,
            "starting/holding a container drag inside its current shelf must keep the ghost in that shelf even when the shelf UI uses local coordinates"
        );
    }

    #[test]
    fn container_move_target_does_not_snap_to_existing_left_shelf_from_canvas_band() {
        let layout = ShelfLayout {
            viewport: Rect::from_min_max(pos2(120.0, 0.0), pos2(800.0, 600.0)),
            left: Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(120.0, 600.0))),
            right: None,
            bottom: None,
        };
        let source_shelf = Rect::from_min_max(pos2(680.0, 0.0), pos2(800.0, 600.0));
        let cursor = pos2(165.0, 300.0);

        assert_eq!(
            container_move_target(cursor, layout.available(), ShelfEdge::Right),
            Some(ShelfEdge::Left),
            "the old broad edge-band logic snapped to the existing left shelf even from the canvas"
        );
        assert_eq!(
            container_move_target_for_cursor(cursor, source_shelf, layout, ShelfEdge::Right),
            None,
            "existing shelves should only be targeted when the cursor is actually over that shelf, not merely near the window edge"
        );
    }

    #[test]
    fn shelf_target_cache_prefers_live_rects_but_keeps_dragged_geometry() {
        let ctx = egui::Context::default();
        let pane_id = Id::new("target-pane");
        let dragged = Id::new("dragged");
        let live = Id::new("live");
        let stale = Id::new("stale");
        let dragged_rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(90.0, 60.0));
        pane::set_snapshot(
            &ctx,
            pane_id,
            vec![
                pane::RectEntry {
                    id: dragged,
                    rect: dragged_rect,
                    frame: Some(dragged_rect),
                },
                pane::RectEntry {
                    id: stale,
                    rect: Rect::from_min_size(pos2(100.0, 100.0), vec2(20.0, 20.0)),
                    frame: None,
                },
            ],
        );
        pane::set_drag(
            &ctx,
            pane_id,
            pane::DragState {
                item: Some(dragged),
                cursor: Some(pos2(30.0, 40.0)),
            },
        );
        pane::begin_drag_frame(&ctx, pane_id);
        pane::push_rect(
            &ctx,
            pane_id,
            live,
            Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 10.0)),
        );

        let cache = shelf_target_cache(&ctx, pane_id);

        assert!(cache.iter().any(|entry| entry.id == live));
        assert!(
            cache
                .iter()
                .any(|entry| entry.id == dragged && entry.rect == dragged_rect),
            "shelf drag previews still need the dragged container's carried full geometry"
        );
        assert!(
            !cache.iter().any(|entry| entry.id == stale),
            "shelf targeting must not resurrect removed containers from stale snapshots"
        );
    }

    #[test]
    fn existing_shelf_slot_foreground_ghost_is_suppressed_when_inline_gap_was_marked() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("target-pane");
        let dragged = Id::new("dragged");
        let accent = Color32::from_rgb(10, 140, 220);
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: Id::new("first"),
                    rect: Rect::from_min_size(pos2(100.0, 20.0), vec2(80.0, 100.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: Id::new("second"),
                    rect: Rect::from_min_size(pos2(100.0, 140.0), vec2(80.0, 100.0)),
                    frame: None,
                },
            ],
        );
        publish_shelf_pane_info(
            &ctx,
            ShelfPaneInfo {
                shelf_id: Id::new("target-shelf"),
                pane_id: target_pane,
                edge: ShelfEdge::Right,
                horizontal_stack: false,
                content_rect: Rect::from_min_size(pos2(96.0, 16.0), vec2(120.0, 260.0)),
                screen_rect: Rect::from_min_size(pos2(96.0, 16.0), vec2(120.0, 260.0)),
                screen_offset: Vec2::ZERO,
                accent,
            },
        );
        mark_external_container_gap(&ctx, target_pane);

        let ghost = existing_shelf_container_slot_ghost(
            &ctx,
            ShelfEdge::Right,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(120.0, 150.0),
                target_edge: Some(ShelfEdge::Right),
                target_shelf: Some(Id::new("target-shelf")),
                target_pane: Some(target_pane),
                target_slot: Some(1),
                container_size: vec2(80.0, 100.0),
            },
        );

        assert!(
            ghost.is_none(),
            "when the target shelf already painted the inline layout gap, do not paint a second foreground destination ghost"
        );
    }

    #[test]
    fn published_bottom_shelf_target_tracks_horizontal_middle_slot() {
        let ctx = egui::Context::default();
        let target_pane = Id::new("bottom-target-pane");
        let dragged = Id::new("dragged");
        let first = Id::new("first");
        let second = Id::new("second");
        let third = Id::new("third");
        let mut state = ShelfState {
            container_move: Some(ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(300.0, 520.0),
                target_edge: Some(ShelfEdge::Bottom),
                target_shelf: None,
                target_pane: None,
                target_slot: None,
                container_size: vec2(160.0, 120.0),
            }),
            ..Default::default()
        };
        pane::set_snapshot(
            &ctx,
            target_pane,
            vec![
                pane::RectEntry {
                    id: first,
                    rect: Rect::from_min_size(pos2(24.0, 500.0), vec2(160.0, 120.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: second,
                    rect: Rect::from_min_size(pos2(204.0, 500.0), vec2(160.0, 120.0)),
                    frame: None,
                },
                pane::RectEntry {
                    id: third,
                    rect: Rect::from_min_size(pos2(384.0, 500.0), vec2(160.0, 120.0)),
                    frame: None,
                },
            ],
        );
        publish_shelf_pane_info(
            &ctx,
            ShelfPaneInfo {
                shelf_id: Id::new("bottom-shelf"),
                pane_id: target_pane,
                edge: ShelfEdge::Bottom,
                horizontal_stack: true,
                content_rect: Rect::from_min_size(pos2(16.0, 492.0), vec2(720.0, 144.0)),
                screen_rect: Rect::from_min_size(pos2(16.0, 492.0), vec2(720.0, 144.0)),
                screen_offset: Vec2::ZERO,
                accent: Color32::WHITE,
            },
        );

        update_container_move_target_from_published(&ctx, &mut state);

        let drag = state
            .container_move
            .expect("bottom shelf container move should stay active");
        assert_eq!(
            drag.target_slot,
            Some(2),
            "bottom shelf containers flow horizontally, so x-position should choose the middle slot"
        );
        let (rect, _) = existing_shelf_container_slot_ghost(&ctx, ShelfEdge::Bottom, drag)
            .expect("bottom shelf slot ghost should be computed");
        assert_eq!(rect.min, pos2(384.0, 492.0));
        assert_eq!(rect.height(), 144.0);
    }

    #[test]
    fn external_container_gap_ignores_stale_target_shelf_owner() {
        let drag = ShelfContainerMoveState {
            container_id: Id::new("dragged"),
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Left,
            cursor: pos2(0.0, 0.0),
            target_edge: Some(ShelfEdge::Right),
            target_shelf: Some(Id::new("stale-target-owner")),
            target_pane: Some(Id::new("stale-target-pane")),
            target_slot: Some(4),
            container_size: vec2(100.0, 80.0),
        };

        assert!(should_render_external_container_gap(
            pane::DragState::default(),
            Some(drag),
            ShelfEdge::Right,
            Rect::from_min_size(pos2(-10.0, -10.0), vec2(40.0, 40.0)),
            None,
        ));
        assert!(!should_render_external_container_gap(
            pane::DragState::default(),
            Some(drag),
            ShelfEdge::Right,
            Rect::from_min_size(pos2(100.0, 100.0), vec2(40.0, 40.0)),
            None,
        ));
    }

    #[test]
    fn external_container_gap_uses_current_pointer_not_stale_drag_cursor() {
        let drag = ShelfContainerMoveState {
            container_id: Id::new("dragged"),
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Right,
            cursor: pos2(40.0, 100.0),
            target_edge: Some(ShelfEdge::Left),
            target_shelf: Some(Id::new("left-shelf")),
            target_pane: Some(Id::new("left-pane")),
            target_slot: Some(0),
            container_size: vec2(100.0, 80.0),
        };
        let left_shelf_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(140.0, 600.0));

        assert!(
            should_render_external_container_gap(
                pane::DragState::default(),
                Some(drag),
                ShelfEdge::Left,
                left_shelf_rect,
                None,
            ),
            "without a live pointer, the helper falls back to the stored drag cursor"
        );
        assert!(
            !should_render_external_container_gap(
                pane::DragState::default(),
                Some(drag),
                ShelfEdge::Left,
                left_shelf_rect,
                Some(pos2(260.0, 300.0)),
            ),
            "a current pointer in the canvas must suppress a stale left-shelf gap"
        );
    }

    #[test]
    fn source_container_gap_is_suppressed_during_cross_shelf_drag() {
        let dragged = Id::new("dragged");
        let source_pane = Id::new("right-pane");
        let layout = ShelfLayout {
            viewport: Rect::from_min_max(pos2(140.0, 0.0), pos2(680.0, 520.0)),
            left: Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(140.0, 520.0))),
            right: Some(Rect::from_min_max(pos2(680.0, 0.0), pos2(800.0, 520.0))),
            bottom: None,
        };
        let right_shelf_rect = layout.right.expect("right shelf");

        assert!(should_suppress_source_container_gap(
            pane::DragState {
                item: Some(dragged),
                cursor: Some(pos2(720.0, 200.0)),
            },
            None,
            source_pane,
            ShelfEdge::Right,
            right_shelf_rect,
            layout,
            Some(pos2(60.0, 200.0)),
        ));
        assert!(
            !should_suppress_source_container_gap(
                pane::DragState {
                    item: Some(dragged),
                    cursor: Some(pos2(720.0, 200.0)),
                },
                None,
                source_pane,
                ShelfEdge::Right,
                right_shelf_rect,
                layout,
                Some(pos2(720.0, 200.0)),
            ),
            "normal same-shelf reorder still keeps the inline gap"
        );
    }

    #[test]
    fn source_container_preview_is_suppressed_during_cross_shelf_drag() {
        let dragged = Id::new("dragged");
        let source_pane = Id::new("right-pane");
        let source_shelf = Id::new("right-shelf");
        let drag_state = pane::DragState {
            item: Some(dragged),
            cursor: Some(pos2(720.0, 200.0)),
        };

        assert!(!should_paint_source_container_preview(
            drag_state,
            Some(ShelfContainerMoveState {
                container_id: dragged,
                source_shelf,
                source_pane,
                source_edge: ShelfEdge::Right,
                cursor: pos2(60.0, 200.0),
                target_edge: Some(ShelfEdge::Left),
                target_shelf: Some(Id::new("left-shelf")),
                target_pane: Some(Id::new("left-pane")),
                target_slot: Some(0),
                container_size: vec2(100.0, 80.0),
            }),
            source_pane,
            ShelfEdge::Right,
        ));
        assert!(
            should_paint_source_container_preview(
                drag_state,
                Some(ShelfContainerMoveState {
                    container_id: dragged,
                    source_shelf,
                    source_pane,
                    source_edge: ShelfEdge::Right,
                    cursor: pos2(720.0, 200.0),
                    target_edge: None,
                    target_shelf: None,
                    target_pane: None,
                    target_slot: None,
                    container_size: vec2(100.0, 80.0),
                }),
                source_pane,
                ShelfEdge::Right,
            ),
            "same-source dragging/reordering can still paint the normal held-container preview"
        );
    }

    #[test]
    fn source_shelf_gap_entry_reanchors_stale_right_shelf_rect() {
        let ctx = egui::Context::default();
        let dragged = Id::new("dragged");
        let right_content = Rect::from_min_size(pos2(1660.0, 80.0), vec2(280.0, 820.0));
        let stale_left_rect = pane::RectEntry {
            id: dragged,
            rect: Rect::from_min_size(pos2(16.0, 280.0), vec2(280.0, 160.0)),
            frame: None,
        };

        let fixed = source_shelf_gap_entry(
            &ctx,
            dragged,
            ShelfEdge::Right,
            right_content,
            stale_left_rect,
        );

        assert_eq!(fixed.rect.min, right_content.min);
        assert_eq!(fixed.rect.width(), right_content.width());
        assert!(right_content.contains(fixed.rect.center()));
    }

    #[test]
    fn source_shelf_gap_entry_reanchors_stale_bottom_shelf_rect() {
        let ctx = egui::Context::default();
        let dragged = Id::new("dragged");
        let bottom_content = Rect::from_min_size(pos2(280.0, 910.0), vec2(1320.0, 220.0));
        let stale_left_rect = pane::RectEntry {
            id: dragged,
            rect: Rect::from_min_size(pos2(16.0, 280.0), vec2(280.0, 160.0)),
            frame: None,
        };

        let fixed = source_shelf_gap_entry(
            &ctx,
            dragged,
            ShelfEdge::Bottom,
            bottom_content,
            stale_left_rect,
        );

        assert_eq!(fixed.rect.min, bottom_content.min);
        assert_eq!(fixed.rect.height(), bottom_content.height());
        assert!(bottom_content.contains(fixed.rect.center()));
    }

    #[test]
    fn external_container_gap_does_not_render_in_source_pane_or_source_edge() {
        let drag = ShelfContainerMoveState {
            container_id: Id::new("dragged"),
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Left,
            cursor: pos2(0.0, 0.0),
            target_edge: Some(ShelfEdge::Right),
            target_shelf: None,
            target_pane: None,
            target_slot: None,
            container_size: vec2(100.0, 80.0),
        };

        assert!(!should_render_external_container_gap(
            pane::DragState {
                item: Some(Id::new("dragged")),
                cursor: Some(pos2(0.0, 0.0)),
            },
            Some(drag),
            ShelfEdge::Right,
            Rect::from_min_size(pos2(-10.0, -10.0), vec2(40.0, 40.0)),
            None,
        ));
        assert!(!should_render_external_container_gap(
            pane::DragState::default(),
            Some(drag),
            ShelfEdge::Left,
            Rect::from_min_size(pos2(-10.0, -10.0), vec2(40.0, 40.0)),
            None,
        ));
    }

    #[test]
    fn commit_container_move_to_new_shelf_creates_detached_shelf_owner() {
        let ctx = egui::Context::default();
        let dragged = Id::new("dragged");
        let source_shelf = Id::new("source-shelf");
        let detached_shelf = detached_shelf_id(source_shelf, dragged);
        let mut state = ShelfState::default();

        commit_container_move(
            &ctx,
            &mut state,
            ShelfContainerMoveState {
                container_id: dragged,
                source_shelf,
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Bottom),
                target_shelf: None,
                target_pane: None,
                target_slot: None,
                container_size: vec2(100.0, 80.0),
            },
        );

        assert_eq!(
            state.container_location(dragged, ShelfEdge::Left),
            ShelfContainerLocation {
                shelf_id: Some(detached_shelf),
                edge: ShelfEdge::Bottom,
            }
        );
    }

    #[test]
    fn commit_container_move_without_target_clears_stale_drag_state() {
        let ctx = egui::Context::default();
        let dragged = Id::new("dragged");
        let source_pane = Id::new("source-pane");
        let mut state = ShelfState {
            container_move: Some(ShelfContainerMoveState {
                container_id: dragged,
                source_shelf: Id::new("source-shelf"),
                source_pane,
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: None,
                target_shelf: None,
                target_pane: None,
                target_slot: None,
                container_size: vec2(100.0, 80.0),
            }),
            ..Default::default()
        };
        pane::set_drag(
            &ctx,
            source_pane,
            pane::DragState {
                item: Some(dragged),
                cursor: Some(pos2(0.0, 0.0)),
            },
        );

        let drag = state
            .container_move
            .expect("stale container move should be present");
        commit_container_move(&ctx, &mut state, drag);

        assert!(state.container_move.is_none());
        assert!(pane::drag_state(&ctx, source_pane).item.is_none());
        assert_eq!(
            state.container_edge(dragged, ShelfEdge::Left),
            ShelfEdge::Left
        );
    }

    #[test]
    fn no_target_container_release_cancels_only_when_outside_source_shelf() {
        let shelf_rect = Rect::from_min_size(pos2(100.0, 100.0), vec2(240.0, 400.0));

        assert!(!should_cancel_no_target_container_release(
            Some(pos2(120.0, 120.0)),
            shelf_rect
        ));
        assert!(!should_cancel_no_target_container_release(
            Some(pos2(90.0, 120.0)),
            shelf_rect
        ));
        assert!(should_cancel_no_target_container_release(
            Some(pos2(10.0, 120.0)),
            shelf_rect
        ));
    }

    #[test]
    fn container_move_target_allows_existing_shelf_edges_but_rejects_source_edge() {
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));

        assert_eq!(
            container_move_target(pos2(990.0, 400.0), available, ShelfEdge::Left),
            Some(ShelfEdge::Right),
            "containers may target an occupied/existing shelf edge for insertion"
        );
        assert_eq!(
            container_move_target(pos2(10.0, 400.0), available, ShelfEdge::Left),
            None,
            "moving out of a shelf should not create a cross-shelf move back into the source edge"
        );
    }

    #[test]
    fn container_move_target_prefers_nearest_valid_edge() {
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));

        assert_eq!(
            container_move_target(pos2(930.0, 770.0), available, ShelfEdge::Left),
            Some(ShelfEdge::Bottom),
            "near a corner, the closest edge should own the target ghost"
        );
    }

    #[test]
    fn layout_repairs_corrupted_persisted_shelf_size() {
        let theme = *style::theme().shelf();
        let shelf_id = Id::new("left-shelf");
        let mut state = ShelfState::default();
        state.sizes.insert(shelf_id.with(ShelfEdge::Left), f32::NAN);
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));
        let shelves =
            vec![ShelfDef::new(shelf_id, ShelfEdge::Left, Color32::WHITE).default_size(240.0)];

        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        assert_eq!(layout.left.unwrap().width(), 240.0);
        assert_eq!(state.edge_size(shelf_id, ShelfEdge::Left), Some(240.0));
    }

    #[test]
    fn container_slot_ghost_uses_existing_shelf_insertion_slot() {
        let drag = ShelfContainerMoveState {
            container_id: Id::new("dragged"),
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Left,
            cursor: pos2(0.0, 0.0),
            target_edge: Some(ShelfEdge::Right),
            target_shelf: None,
            target_pane: None,
            target_slot: None,
            container_size: vec2(80.0, 120.0),
        };
        let snap = [
            pane::RectEntry {
                id: Id::new("first"),
                rect: Rect::from_min_size(pos2(100.0, 20.0), vec2(80.0, 120.0)),
                frame: None,
            },
            pane::RectEntry {
                id: Id::new("second"),
                rect: Rect::from_min_size(pos2(100.0, 160.0), vec2(80.0, 120.0)),
                frame: None,
            },
        ];

        let before_first = container_slot_ghost_rect_in(None, &snap, drag, 0, false)
            .expect("slot before first should have a ghost");
        let middle = container_slot_ghost_rect_in(None, &snap, drag, 1, false)
            .expect("slot between containers should have a ghost");
        let after_last = container_slot_ghost_rect_in(None, &snap, drag, 2, false)
            .expect("slot after last should have a ghost");

        assert_eq!(before_first.min, pos2(100.0, 20.0));
        assert_eq!(middle.min, pos2(100.0, 160.0));
        assert_eq!(after_last.min, pos2(100.0, 280.0));
    }

    #[test]
    fn container_slot_ghost_uses_horizontal_insertion_slot_for_bottom_shelf() {
        let drag = ShelfContainerMoveState {
            container_id: Id::new("dragged"),
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Left,
            cursor: pos2(0.0, 0.0),
            target_edge: Some(ShelfEdge::Bottom),
            target_shelf: None,
            target_pane: None,
            target_slot: None,
            container_size: vec2(180.0, 96.0),
        };
        let snap = [
            pane::RectEntry {
                id: Id::new("first"),
                rect: Rect::from_min_size(pos2(30.0, 400.0), vec2(180.0, 96.0)),
                frame: None,
            },
            pane::RectEntry {
                id: Id::new("second"),
                rect: Rect::from_min_size(pos2(230.0, 400.0), vec2(180.0, 96.0)),
                frame: None,
            },
        ];

        let before_first = container_slot_ghost_rect_in(None, &snap, drag, 0, true)
            .expect("slot before first should have a horizontal ghost");
        let middle = container_slot_ghost_rect_in(None, &snap, drag, 1, true)
            .expect("slot between containers should have a horizontal ghost");
        let after_last = container_slot_ghost_rect_in(None, &snap, drag, 2, true)
            .expect("slot after last should have a horizontal ghost");

        assert_eq!(before_first.min, pos2(30.0, 400.0));
        assert_eq!(middle.min, pos2(230.0, 400.0));
        assert_eq!(after_last.min, pos2(410.0, 400.0));
    }

    #[test]
    fn existing_shelf_container_ghost_preserves_slot_main_axis() {
        let drag = ShelfContainerMoveState {
            container_id: Id::new("dragged"),
            source_shelf: Id::new("source-shelf"),
            source_pane: Id::new("source-pane"),
            source_edge: ShelfEdge::Left,
            cursor: pos2(0.0, 0.0),
            target_edge: Some(ShelfEdge::Right),
            target_shelf: None,
            target_pane: None,
            target_slot: None,
            container_size: vec2(80.0, 120.0),
        };
        let content_rect = Rect::from_min_size(pos2(100.0, 20.0), vec2(100.0, 260.0));
        let snap = [
            pane::RectEntry {
                id: Id::new("first"),
                rect: Rect::from_min_size(pos2(100.0, 20.0), vec2(80.0, 120.0)),
                frame: None,
            },
            pane::RectEntry {
                id: Id::new("second"),
                rect: Rect::from_min_size(pos2(100.0, 160.0), vec2(80.0, 120.0)),
                frame: None,
            },
        ];

        let after_last = container_slot_ghost_rect_in(Some(content_rect), &snap, drag, 2, false)
            .expect("slot after last should keep the actual insertion position");

        assert_eq!(after_last.min, pos2(100.0, 280.0));
        assert!(content_rect.contains(after_last.min));
        assert_eq!(
            after_last.height(),
            120.0,
            "slot ghosts keep the dragged container's main-axis size instead of snapping upward"
        );
    }

    #[test]
    fn new_shelf_container_ghost_uses_target_shelf_content_rect() {
        let ctx = egui::Context::default();
        let container_id = Id::new("dragged");
        let shelf_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 180.0));
        let content_rect = shelf_rect.shrink(style::theme().shelf().padding);

        let ghost =
            new_shelf_container_ghost_rect(&ctx, container_id, ShelfEdge::Bottom, shelf_rect);

        assert_eq!(ghost.min, content_rect.min);
        assert_eq!(ghost.height(), content_rect.height());
        assert!(ghost.width() < content_rect.width());
        assert!(content_rect.contains(ghost.max));
    }

    #[test]
    fn new_side_shelf_container_ghost_uses_target_shelf_width() {
        let ctx = egui::Context::default();
        let container_id = Id::new("dragged");
        let shelf_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 700.0));
        let content_rect = shelf_rect.shrink(style::theme().shelf().padding);

        let ghost =
            new_shelf_container_ghost_rect(&ctx, container_id, ShelfEdge::Right, shelf_rect);

        assert_eq!(ghost.min, content_rect.min);
        assert_eq!(ghost.width(), content_rect.width());
        assert!(ghost.height() < content_rect.height());
        assert!(content_rect.contains(ghost.max));
    }

    #[test]
    fn shelf_move_ghost_to_bottom_respects_occupied_side_shelf() {
        let theme = *style::theme().shelf();
        let layout = ShelfLayout {
            viewport: Rect::from_min_max(pos2(240.0, 0.0), pos2(780.0, 800.0)),
            left: Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(240.0, 800.0))),
            right: Some(Rect::from_min_max(pos2(780.0, 0.0), pos2(1000.0, 800.0))),
            bottom: None,
        };

        let ghost = shelf_drop_rect(layout, ShelfEdge::Left, ShelfEdge::Bottom, &theme)
            .expect("bottom is the only free target edge");

        assert_eq!(ghost.left(), 0.0);
        assert_eq!(ghost.right(), 780.0);
        assert_eq!(ghost.bottom(), 800.0);
        assert_eq!(
            ghost.height(),
            theme.bottom_default_size,
            "cross-axis shelf moves should preview the target bottom height, not the source side width"
        );
    }

    #[test]
    fn shelf_move_ghost_to_side_keeps_full_height_when_bottom_is_occupied() {
        let theme = *style::theme().shelf();
        let layout = ShelfLayout {
            viewport: Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 620.0)),
            left: None,
            right: None,
            bottom: Some(Rect::from_min_max(pos2(0.0, 620.0), pos2(1000.0, 800.0))),
        };

        let ghost = shelf_drop_rect(layout, ShelfEdge::Left, ShelfEdge::Right, &theme)
            .expect("right edge is free");

        assert_eq!(
            ghost,
            Rect::from_min_max(pos2(700.0, 0.0), pos2(1000.0, 800.0)),
            "side shelves reserve before bottom shelves, so side drop ghosts must show full height"
        );
    }

    #[test]
    fn container_new_bottom_shelf_ghost_respects_source_side_shelf() {
        let theme = *style::theme().shelf();
        let layout = ShelfLayout {
            viewport: Rect::from_min_max(pos2(240.0, 0.0), pos2(1000.0, 800.0)),
            left: Some(Rect::from_min_max(pos2(0.0, 0.0), pos2(240.0, 800.0))),
            right: None,
            bottom: None,
        };

        let ghost = container_drop_rect(layout, ShelfEdge::Left, ShelfEdge::Bottom, &theme)
            .expect("bottom edge is free");

        assert_eq!(
            ghost,
            Rect::from_min_max(
                pos2(240.0, 800.0 - theme.bottom_default_size),
                pos2(1000.0, 800.0)
            ),
            "moving one container out of a side shelf leaves that source shelf in place, but the new bottom shelf ghost uses the target bottom height"
        );
    }

    #[test]
    fn shelf_move_target_rejects_fully_occupied_edges() {
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));
        let occupied = ShelfOccupied {
            left: false,
            right: true,
            bottom: true,
        };

        let source_band_cursor = pos2(12.0, 790.0);
        let target = shelf_move_target(source_band_cursor, available, occupied, ShelfEdge::Left);

        assert_eq!(
            target, None,
            "when the source edge is excluded and every other edge is occupied, no move target is valid"
        );
    }

    #[test]
    fn finishing_shelf_move_without_target_preserves_original_edge() {
        let shelf_id = Id::new("movable-shelf");
        let mut state = ShelfState::default();

        state.begin_drag(shelf_id, ShelfEdge::Left, pos2(10.0, 10.0));
        state.update_drag(pos2(400.0, 400.0), None);
        state.finish_drag();

        assert_eq!(
            state.edge(shelf_id, ShelfEdge::Left),
            ShelfEdge::Left,
            "dropping a shelf outside a valid target band must cancel the move"
        );
    }

    #[test]
    fn finishing_shelf_move_preserves_active_container_on_new_edge() {
        let shelf_id = Id::new("movable-shelf");
        let active_container = Id::new("active-container");
        let mut state = ShelfState::default();
        state.set_active_container_for_group(
            shelf_active_container_key_for(shelf_id, ShelfEdge::Left),
            active_container,
        );

        state.begin_drag(shelf_id, ShelfEdge::Left, pos2(10.0, 10.0));
        state.update_drag(pos2(990.0, 400.0), Some(ShelfEdge::Right));
        state.finish_drag();

        assert_eq!(state.edge(shelf_id, ShelfEdge::Left), ShelfEdge::Right);
        assert_eq!(
            state.active_container_for_group(shelf_active_container_key_for(
                shelf_id,
                ShelfEdge::Right
            )),
            Some(active_container),
            "moving a shelf should carry its rendered-edge active selection to the new edge"
        );
        assert_eq!(
            state.active_container_for_group(shelf_active_container_key_for(
                shelf_id,
                ShelfEdge::Left
            )),
            None,
            "the old rendered-edge active key must not keep stale selection after the shelf moves"
        );
    }

    #[test]
    fn canceling_shelf_move_preserves_original_edge() {
        let shelf_id = Id::new("movable-shelf");
        let mut state = ShelfState::default();

        state.begin_drag(shelf_id, ShelfEdge::Left, pos2(10.0, 10.0));
        state.update_drag(pos2(990.0, 400.0), Some(ShelfEdge::Right));
        state.cancel_drag();

        assert_eq!(
            state.edge(shelf_id, ShelfEdge::Left),
            ShelfEdge::Left,
            "escape/cancel should not persist the previewed target edge"
        );
    }

    #[test]
    fn shelf_move_start_rejects_container_rects() {
        let ctx = egui::Context::default();
        let pane_id = Id::new("shelf-pane");
        let container_id = Id::new("container");
        pane::begin_drag_frame(&ctx, pane_id);
        pane::push_rect(
            &ctx,
            pane_id,
            container_id,
            Rect::from_min_size(pos2(40.0, 50.0), vec2(120.0, 180.0)),
        );
        pane::finalize_snapshot(&ctx, pane_id);

        assert!(pointer_over_shelf_container(
            &ctx,
            pane_id,
            pos2(80.0, 100.0)
        ));
        assert!(!pointer_over_shelf_container(
            &ctx,
            pane_id,
            pos2(10.0, 10.0)
        ));
    }

    #[test]
    fn shelf_move_start_uses_container_frame_rect_when_available() {
        let ctx = egui::Context::default();
        let pane_id = Id::new("shelf-pane");
        let container_id = Id::new("container");
        pane::begin_drag_frame(&ctx, pane_id);
        pane::push_rect_with_frame(
            &ctx,
            pane_id,
            container_id,
            Rect::from_min_size(pos2(60.0, 60.0), vec2(80.0, 80.0)),
            Some(Rect::from_min_size(pos2(40.0, 40.0), vec2(120.0, 120.0))),
        );
        pane::finalize_snapshot(&ctx, pane_id);

        assert!(
            pointer_over_shelf_container(&ctx, pane_id, pos2(45.0, 45.0)),
            "frame chrome belongs to the container and must not start a shelf move"
        );
    }

    #[test]
    fn shelf_move_start_rejects_container_dot_handles() {
        let ctx = egui::Context::default();
        let pane_id = Id::new("shelf-pane");
        pane::clear_container_dot_rects(&ctx, pane_id);
        pane::record_container_dot_rect(
            &ctx,
            pane_id,
            Rect::from_min_size(pos2(20.0, 180.0), vec2(220.0, 8.0)),
        );

        assert!(
            pane::pointer_over_container_dots(&ctx, pane_id, pos2(80.0, 184.0)),
            "container resize/reorder dot handles are not empty shelf background"
        );
        assert!(!pane::pointer_over_container_dots(
            &ctx,
            pane_id,
            pos2(80.0, 150.0)
        ));
    }

    #[test]
    fn shelf_resize_direction_matches_edge_handles() {
        assert_eq!(
            resized_shelf_extent(ShelfEdge::Left, 200.0, vec2(35.0, 0.0), 100.0, 400.0),
            235.0,
            "dragging the left shelf handle right should make it wider"
        );
        assert_eq!(
            resized_shelf_extent(ShelfEdge::Right, 200.0, vec2(-35.0, 0.0), 100.0, 400.0),
            235.0,
            "dragging the right shelf handle left should make it wider"
        );
        assert_eq!(
            resized_shelf_extent(ShelfEdge::Bottom, 180.0, vec2(0.0, -40.0), 100.0, 400.0),
            220.0,
            "dragging the bottom shelf handle up should make it taller"
        );
    }

    #[test]
    fn shelf_resize_extent_clamps_to_bounds() {
        assert_eq!(
            resized_shelf_extent(ShelfEdge::Left, 200.0, vec2(-500.0, 0.0), 120.0, 360.0),
            120.0
        );
        assert_eq!(
            resized_shelf_extent(ShelfEdge::Bottom, 200.0, vec2(0.0, -500.0), 120.0, 360.0),
            360.0
        );
    }

    #[test]
    fn shelf_resize_handle_rects_sit_on_inner_edges() {
        let theme = ShelfTheme {
            resize_handle_thickness: 8.0,
            ..*style::theme().shelf()
        };
        let left = Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 600.0));
        let right = Rect::from_min_max(pos2(600.0, 0.0), pos2(800.0, 600.0));
        let bottom = Rect::from_min_max(pos2(0.0, 450.0), pos2(800.0, 600.0));

        assert_eq!(
            resize_handle_rect(ShelfEdge::Left, left, &theme),
            Rect::from_min_max(pos2(192.0, 0.0), pos2(200.0, 600.0))
        );
        assert_eq!(
            resize_handle_rect(ShelfEdge::Right, right, &theme),
            Rect::from_min_max(pos2(600.0, 0.0), pos2(608.0, 600.0))
        );
        assert_eq!(
            resize_handle_rect(ShelfEdge::Bottom, bottom, &theme),
            Rect::from_min_max(pos2(0.0, 450.0), pos2(800.0, 458.0))
        );
    }

    #[test]
    fn moved_container_renders_inside_target_shelf_group() {
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(target_shelf), ShelfEdge::Right);

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
                ),
                ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::WHITE).container(
                    ShelfContainer::tabbed(Id::new("already-there"), "Target", "box", test_tabs()),
                ),
            ],
            &state,
        );

        let target_group = groups
            .iter()
            .find(|group| group.id == target_shelf && group.edge == ShelfEdge::Right)
            .expect("target shelf group should exist");
        assert!(
            target_group
                .containers
                .iter()
                .any(|container| container.spec.container_id() == moved_container),
            "moved container should render in the existing target shelf group"
        );
        assert!(
            !groups.iter().any(|group| {
                group.id == source_shelf
                    && group.edge == ShelfEdge::Right
                    && group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == moved_container)
            }),
            "moved container must not create an overlapping source-owned right shelf"
        );
    }

    #[test]
    fn edge_only_moved_container_renders_inside_existing_edge_shelf_group() {
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_edge(moved_container, ShelfEdge::Right);

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
                ),
                ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::LIGHT_BLUE)
                    .default_size(260.0)
                    .movable()
                    .container(ShelfContainer::tabbed(
                        Id::new("target-container"),
                        "Target",
                        "box",
                        test_tabs(),
                    )),
            ],
            &state,
        );

        let target_group = groups
            .iter()
            .find(|group| group.id == target_shelf && group.edge == ShelfEdge::Right)
            .expect("existing target edge shelf should own the right group");
        assert_eq!(target_group.containers.len(), 2);
        assert_eq!(target_group.accent, Color32::LIGHT_BLUE);
        assert_eq!(target_group.default_size, Some(260.0));
        assert!(target_group.movable);
        assert!(
            target_group
                .containers
                .iter()
                .any(|container| container.spec.container_id() == moved_container),
            "edge-only moved containers should merge into the existing shelf on that edge"
        );
        assert!(!groups.iter().any(|group| {
            group.id == source_shelf
                && group.edge == ShelfEdge::Right
                && group
                    .containers
                    .iter()
                    .any(|container| container.spec.container_id() == moved_container)
        }));
    }

    #[test]
    fn edge_only_moved_container_renders_inside_overridden_edge_shelf_group() {
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_edge(target_shelf, ShelfEdge::Right);
        state.set_container_edge(moved_container, ShelfEdge::Right);

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
                ),
                ShelfDef::new(target_shelf, ShelfEdge::Bottom, Color32::LIGHT_BLUE)
                    .default_size(260.0)
                    .movable()
                    .container(ShelfContainer::tabbed(
                        Id::new("target-container"),
                        "Target",
                        "box",
                        test_tabs(),
                    )),
            ],
            &state,
        );

        let target_group = groups
            .iter()
            .find(|group| group.id == target_shelf && group.edge == ShelfEdge::Right)
            .expect("state-moved target shelf should own the right group");
        assert_eq!(target_group.containers.len(), 2);
        assert_eq!(target_group.accent, Color32::LIGHT_BLUE);
        assert_eq!(target_group.default_size, Some(260.0));
        assert!(target_group.movable);
        assert!(
            target_group
                .containers
                .iter()
                .any(|container| container.spec.container_id() == moved_container),
            "edge-only moved containers should merge into state-moved shelves on that edge"
        );
        assert!(
            !groups
                .iter()
                .any(|group| { group.id == target_shelf && group.edge == ShelfEdge::Bottom })
        );
    }

    #[test]
    fn moved_container_into_empty_target_shelf_does_not_duplicate_area() {
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(target_shelf), ShelfEdge::Right);

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
                ),
                ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::WHITE),
            ],
            &state,
        );

        let target_groups = groups
            .iter()
            .filter(|group| group.id == target_shelf && group.edge == ShelfEdge::Right)
            .count();
        assert_eq!(
            target_groups, 1,
            "empty target shelf and moved container should share one rendered area"
        );
    }

    #[test]
    fn moved_container_into_empty_target_shelf_does_not_render_default_edge() {
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(target_shelf), ShelfEdge::Bottom);

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
                ),
                ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::WHITE),
            ],
            &state,
        );

        assert!(
            !groups
                .iter()
                .any(|group| group.id == target_shelf && group.edge == ShelfEdge::Right),
            "an empty target shelf that only owns moved containers must not render a phantom default-edge group"
        );
        let bottom_group = groups
            .iter()
            .find(|group| group.id == target_shelf && group.edge == ShelfEdge::Bottom)
            .expect("the moved container should render in the target shelf on its active edge");
        assert_eq!(bottom_group.containers.len(), 1);
        assert_eq!(
            bottom_group.containers[0].spec.container_id(),
            moved_container
        );
    }

    #[test]
    fn moved_container_into_empty_target_shelf_only_reserves_target_edge() {
        let theme = *style::theme().shelf();
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(target_shelf), ShelfEdge::Bottom);
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));
        let shelves = vec![
            ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE)
                .default_size(180.0)
                .container(ShelfContainer::tabbed(
                    moved_container,
                    "Moved",
                    "box",
                    test_tabs(),
                )),
            ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::WHITE).default_size(260.0),
        ];

        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        assert!(
            layout.right.is_none(),
            "an empty target shelf that only owns moved containers must not also reserve its default edge"
        );
        assert_eq!(layout.bottom.unwrap().height(), 260.0);
        assert_eq!(layout.viewport.max.y, 540.0);
        assert_eq!(layout.viewport.max.x, 1000.0);
    }

    #[test]
    fn stale_moved_container_owner_does_not_hide_empty_shelf_layout() {
        let theme = *style::theme().shelf();
        let target_shelf = Id::new("target-shelf");
        let stale_container = Id::new("removed-container");
        let mut state = ShelfState::default();
        state.set_container_location(stale_container, Some(target_shelf), ShelfEdge::Bottom);
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));
        let shelves =
            vec![ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::WHITE).default_size(260.0)];

        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        assert!(
            layout.bottom.is_none(),
            "stale state for removed containers must not create a bottom shelf"
        );
        assert_eq!(
            layout.right.unwrap().width(),
            260.0,
            "an actually declared empty shelf should still reserve/render its default edge"
        );
    }

    #[test]
    fn stale_moved_container_owner_does_not_hide_empty_shelf_render_group() {
        let target_shelf = Id::new("target-shelf");
        let stale_container = Id::new("removed-container");
        let mut state = ShelfState::default();
        state.set_container_location(stale_container, Some(target_shelf), ShelfEdge::Bottom);

        let groups = split_shelf_render_groups(
            vec![ShelfDef::new(
                target_shelf,
                ShelfEdge::Right,
                Color32::WHITE,
            )],
            &state,
        );

        assert!(
            groups
                .iter()
                .any(|group| group.id == target_shelf && group.edge == ShelfEdge::Right),
            "stale owner state for removed containers must not suppress the declared empty shelf"
        );
        assert!(
            !groups
                .iter()
                .any(|group| group.id == target_shelf && group.edge == ShelfEdge::Bottom),
            "stale owner state for removed containers must not create a phantom moved edge"
        );
    }

    #[test]
    fn same_edge_shelves_merge_into_one_render_area() {
        let first_shelf = Id::new("first-shelf");
        let second_shelf = Id::new("second-shelf");

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(first_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(Id::new("first-container"), "First", "box", test_tabs()),
                ),
                ShelfDef::new(second_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(
                        Id::new("second-container"),
                        "Second",
                        "box",
                        test_tabs(),
                    ),
                ),
            ],
            &ShelfState::default(),
        );

        let left_groups: Vec<_> = groups
            .iter()
            .filter(|group| group.edge == ShelfEdge::Left)
            .collect();
        assert_eq!(
            left_groups.len(),
            1,
            "one edge must produce one shelf render area"
        );
        assert_eq!(left_groups[0].containers.len(), 2);
    }

    #[test]
    fn moved_shelf_cannot_collapse_into_existing_shelf_edge() {
        let moved_shelf = Id::new("moved-shelf");
        let existing_shelf = Id::new("existing-shelf");
        let moved_container = Id::new("moved-container");
        let existing_container = Id::new("existing-container");
        let mut state = ShelfState::default();
        state.set_edge(moved_shelf, ShelfEdge::Right);

        let shelves = vec![
            ShelfDef::new(moved_shelf, ShelfEdge::Left, Color32::WHITE).container(
                ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
            ),
            ShelfDef::new(existing_shelf, ShelfEdge::Right, Color32::LIGHT_BLUE).container(
                ShelfContainer::tabbed(existing_container, "Existing", "box", test_tabs()),
            ),
        ];

        let edges = shelf_layout_edges(&shelves, &state);
        assert!(edges.contains(&ShelfLayoutEntry {
            base_idx: 0,
            shelf_id: moved_shelf,
            edge: ShelfEdge::Left,
        }));
        assert!(edges.contains(&ShelfLayoutEntry {
            base_idx: 1,
            shelf_id: existing_shelf,
            edge: ShelfEdge::Right,
        }));

        let groups = split_shelf_render_groups(shelves, &state);
        assert!(
            groups.iter().any(|group| {
                group.id == moved_shelf
                    && group.edge == ShelfEdge::Left
                    && group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == moved_container)
            }),
            "a rejected/invalid shelf move must fall back to its original free edge instead of merging into the occupied edge"
        );
        assert!(
            groups.iter().any(|group| {
                group.id == existing_shelf
                    && group.edge == ShelfEdge::Right
                    && group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == existing_container)
            }),
            "the existing shelf keeps its own identity and containers"
        );
        assert!(
            !groups.iter().any(|group| {
                group.edge == ShelfEdge::Right
                    && group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == moved_container)
                    && group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == existing_container)
            }),
            "two different shelves must not collapse into one render group after a conflicting move"
        );
    }

    #[test]
    fn split_shelf_created_by_container_move_can_move_without_merging_with_source() {
        let ctx = egui::Context::default();
        let source_shelf = Id::new("source-shelf");
        let kept_container = Id::new("kept-container");
        let moved_container = Id::new("moved-container");
        let detached_shelf = detached_shelf_id(source_shelf, moved_container);
        let mut state = ShelfState::default();

        commit_container_move(
            &ctx,
            &mut state,
            ShelfContainerMoveState {
                container_id: moved_container,
                source_shelf,
                source_pane: Id::new("source-pane"),
                source_edge: ShelfEdge::Left,
                cursor: pos2(0.0, 0.0),
                target_edge: Some(ShelfEdge::Bottom),
                target_shelf: None,
                target_pane: None,
                target_slot: None,
                container_size: vec2(100.0, 80.0),
            },
        );

        state.begin_drag(detached_shelf, ShelfEdge::Bottom, pos2(0.0, 0.0));
        state.update_drag(pos2(100.0, 100.0), Some(ShelfEdge::Right));
        state.finish_drag();

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE)
                    .container(ShelfContainer::tabbed(
                        kept_container,
                        "Kept",
                        "box",
                        test_tabs(),
                    ))
                    .container(ShelfContainer::tabbed(
                        moved_container,
                        "Moved",
                        "box",
                        test_tabs(),
                    )),
            ],
            &state,
        );

        assert!(
            groups.iter().any(|group| {
                group.id == source_shelf
                    && group.edge == ShelfEdge::Left
                    && group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == kept_container)
                    && !group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == moved_container)
            }),
            "the original shelf must keep only its remaining containers"
        );
        assert!(
            groups.iter().any(|group| {
                group.id == detached_shelf
                    && group.edge == ShelfEdge::Right
                    && group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == moved_container)
                    && !group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == kept_container)
            }),
            "the split-off shelf must move independently without merging back into the source shelf"
        );
    }

    #[test]
    fn moved_container_merges_with_later_declared_target_edge() {
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(target_shelf), ShelfEdge::Right);

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
                ),
                ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::WHITE).container(
                    ShelfContainer::tabbed(
                        Id::new("target-container"),
                        "Target",
                        "box",
                        test_tabs(),
                    ),
                ),
            ],
            &state,
        );

        let right_groups: Vec<_> = groups
            .iter()
            .filter(|group| group.edge == ShelfEdge::Right)
            .collect();
        assert_eq!(right_groups.len(), 1);
        assert_eq!(right_groups[0].containers.len(), 2);
    }

    #[test]
    fn moved_container_uses_target_shelf_extent_for_layout() {
        let theme = *style::theme().shelf();
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(target_shelf), ShelfEdge::Right);
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));
        let shelves = vec![
            ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE)
                .default_size(180.0)
                .container(ShelfContainer::tabbed(
                    moved_container,
                    "Moved",
                    "box",
                    test_tabs(),
                )),
            ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::WHITE).default_size(260.0),
        ];

        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        assert!(layout.left.is_none());
        assert_eq!(layout.right.unwrap().width(), 260.0);
        assert_eq!(layout.viewport.max.x, 740.0);
    }

    #[test]
    fn moved_container_uses_target_shelf_extent_regardless_of_declaration_order() {
        let theme = *style::theme().shelf();
        let source_shelf = Id::new("source-shelf");
        let target_shelf = Id::new("target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(target_shelf), ShelfEdge::Right);
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));
        let shelves = vec![
            ShelfDef::new(target_shelf, ShelfEdge::Right, Color32::WHITE).default_size(260.0),
            ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE)
                .default_size(180.0)
                .container(ShelfContainer::tabbed(
                    moved_container,
                    "Moved",
                    "box",
                    test_tabs(),
                )),
        ];

        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        assert!(layout.left.is_none());
        assert_eq!(layout.right.unwrap().width(), 260.0);
        assert_eq!(layout.viewport.max.x, 740.0);
    }

    #[test]
    fn moved_container_with_missing_owner_uses_existing_edge_shelf_extent() {
        let theme = *style::theme().shelf();
        let source_shelf = Id::new("source-shelf");
        let missing_shelf = Id::new("removed-target-shelf");
        let replacement_shelf = Id::new("replacement-target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(missing_shelf), ShelfEdge::Right);
        let available = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));
        let shelves = vec![
            ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE)
                .default_size(180.0)
                .container(ShelfContainer::tabbed(
                    moved_container,
                    "Moved",
                    "box",
                    test_tabs(),
                )),
            ShelfDef::new(replacement_shelf, ShelfEdge::Right, Color32::WHITE).default_size(260.0),
        ];

        let layout = layout_shelves(available, &shelves, &mut state, &theme);

        assert!(layout.left.is_none());
        assert_eq!(
            layout.right.unwrap().width(),
            260.0,
            "declared containers with stale owner ids should adopt the existing shelf on their edge"
        );
        assert_eq!(layout.viewport.max.x, 740.0);
    }

    #[test]
    fn moved_container_with_missing_owner_renders_in_existing_edge_shelf() {
        let source_shelf = Id::new("source-shelf");
        let missing_shelf = Id::new("removed-target-shelf");
        let replacement_shelf = Id::new("replacement-target-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(missing_shelf), ShelfEdge::Right);

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(source_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
                ),
                ShelfDef::new(replacement_shelf, ShelfEdge::Right, Color32::LIGHT_BLUE)
                    .default_size(260.0)
                    .container(ShelfContainer::tabbed(
                        Id::new("replacement-container"),
                        "Replacement",
                        "box",
                        test_tabs(),
                    )),
            ],
            &state,
        );

        let right_group = groups
            .iter()
            .find(|group| group.id == replacement_shelf && group.edge == ShelfEdge::Right)
            .expect("stale owner ids should fall back to the current shelf on the target edge");
        assert_eq!(right_group.accent, Color32::LIGHT_BLUE);
        assert_eq!(right_group.default_size, Some(260.0));
        assert!(
            right_group
                .containers
                .iter()
                .any(|container| container.spec.container_id() == moved_container)
        );
        assert!(!groups.iter().any(|group| group.id == missing_shelf));
    }

    #[test]
    fn shelf_display_order_prefers_persisted_order_over_hashmap_iteration() {
        let ctx = egui::Context::default();
        let pane_id = Id::new("shelf-pane");
        let first = Id::new("first");
        let second = Id::new("second");
        let third = Id::new("third");
        pane::set_section_order(&ctx, pane_id, vec![second, first, third]);

        let order: Vec<Id> =
            shelf_display_order(&ctx, pane_id, [&first, &second, &third].into_iter()).collect();

        assert_eq!(order, vec![second, first, third]);
    }

    #[test]
    fn active_container_fallback_uses_declared_order_not_response_map_order() {
        let ctx = egui::Context::default();
        let pane_id = Id::new("shelf-pane");
        let first = Id::new("first");
        let second = Id::new("second");
        let third = Id::new("third");

        assert_eq!(
            resolve_visible_active_container(&ctx, pane_id, None, &[first, second, third], |id| id
                == second
                || id == third,),
            Some(second),
            "when no active container is visible, shelves should select the first visible container in declared/layout order, not arbitrary HashMap response order"
        );
    }

    #[test]
    fn same_shelf_groups_on_different_edges_have_independent_active_keys() {
        let shelf_id = Id::new("split-shelf");
        let left_group = ShelfDef::new(shelf_id, ShelfEdge::Left, Color32::WHITE);
        let bottom_group = ShelfDef::new(shelf_id, ShelfEdge::Bottom, Color32::WHITE);

        assert_ne!(
            shelf_active_container_key(&left_group),
            shelf_active_container_key(&bottom_group),
            "one shelf can produce render groups on multiple edges, so active-container state must be per rendered edge group"
        );
    }

    #[test]
    fn moved_container_keeps_current_shelf_owner_when_moved_again() {
        let original_shelf = Id::new("original-shelf");
        let adopted_shelf = Id::new("adopted-shelf");
        let moved_container = Id::new("moved-container");
        let mut state = ShelfState::default();
        state.set_container_location(moved_container, Some(adopted_shelf), ShelfEdge::Bottom);

        let groups = split_shelf_render_groups(
            vec![
                ShelfDef::new(original_shelf, ShelfEdge::Left, Color32::WHITE).container(
                    ShelfContainer::tabbed(moved_container, "Moved", "box", test_tabs()),
                ),
                ShelfDef::new(adopted_shelf, ShelfEdge::Right, Color32::WHITE),
            ],
            &state,
        );

        assert!(
            groups.iter().any(|group| {
                group.id == adopted_shelf
                    && group.edge == ShelfEdge::Bottom
                    && group
                        .containers
                        .iter()
                        .any(|container| container.spec.container_id() == moved_container)
            }),
            "a container moved again should keep the shelf owner it was dragged from"
        );
    }

    #[test]
    fn moving_shelf_carries_adopted_containers_on_source_edge() {
        let shelf_id = Id::new("movable-shelf");
        let adopted_container = Id::new("adopted-container");
        let moved_out_container = Id::new("moved-out-container");
        let mut state = ShelfState::default();
        state.set_container_location(adopted_container, Some(shelf_id), ShelfEdge::Left);
        state.set_container_location(moved_out_container, Some(shelf_id), ShelfEdge::Bottom);

        state.begin_drag(shelf_id, ShelfEdge::Left, pos2(0.0, 0.0));
        state.update_drag(pos2(100.0, 100.0), Some(ShelfEdge::Right));
        state.finish_drag();

        assert_eq!(state.edge(shelf_id, ShelfEdge::Left), ShelfEdge::Right);
        assert_eq!(
            state.container_location(adopted_container, ShelfEdge::Left),
            ShelfContainerLocation {
                shelf_id: Some(shelf_id),
                edge: ShelfEdge::Right,
            }
        );
        assert_eq!(
            state.container_location(moved_out_container, ShelfEdge::Left),
            ShelfContainerLocation {
                shelf_id: Some(shelf_id),
                edge: ShelfEdge::Bottom,
            },
            "containers already moved away from the dragged edge should not be pulled back"
        );
    }

    #[test]
    fn moving_shelf_preserves_user_resized_extent() {
        let shelf_id = Id::new("movable-shelf");
        let mut state = ShelfState::default();
        state.set_edge_size(shelf_id, ShelfEdge::Left, 344.0);

        state.begin_drag(shelf_id, ShelfEdge::Left, pos2(0.0, 0.0));
        state.update_drag(pos2(100.0, 100.0), Some(ShelfEdge::Right));
        state.finish_drag();

        assert_eq!(state.edge(shelf_id, ShelfEdge::Left), ShelfEdge::Right);
        assert_eq!(state.edge_size(shelf_id, ShelfEdge::Right), Some(344.0));
    }

    #[test]
    fn moving_shelf_does_not_copy_side_width_to_bottom_height() {
        let shelf_id = Id::new("movable-shelf");
        let mut state = ShelfState::default();
        state.set_edge_size(shelf_id, ShelfEdge::Left, 344.0);
        state.set_edge_size(shelf_id, ShelfEdge::Bottom, 180.0);

        state.begin_drag(shelf_id, ShelfEdge::Left, pos2(0.0, 0.0));
        state.update_drag(pos2(100.0, 100.0), Some(ShelfEdge::Bottom));
        state.finish_drag();

        assert_eq!(state.edge(shelf_id, ShelfEdge::Left), ShelfEdge::Bottom);
        assert_eq!(
            state.edge_size(shelf_id, ShelfEdge::Bottom),
            Some(180.0),
            "side widths and bottom heights are different axes; moving across axes must not overwrite the target edge size"
        );
    }

    #[test]
    fn moving_shelf_clears_resize_start_state_on_source_and_target_edges() {
        let shelf_id = Id::new("movable-shelf");
        let mut state = ShelfState::default();
        state.resize_starts.insert(
            shelf_id.with(ShelfEdge::Left),
            ShelfResizeStart {
                size: 240.0,
                pointer: pos2(240.0, 100.0),
            },
        );
        state.resize_starts.insert(
            shelf_id.with(ShelfEdge::Right),
            ShelfResizeStart {
                size: 320.0,
                pointer: pos2(760.0, 100.0),
            },
        );

        state.begin_drag(shelf_id, ShelfEdge::Left, pos2(0.0, 0.0));
        state.update_drag(pos2(100.0, 100.0), Some(ShelfEdge::Right));
        state.finish_drag();

        assert!(
            !state
                .resize_starts
                .contains_key(&shelf_id.with(ShelfEdge::Left)),
            "moving a shelf should clear stale resize capture on the source edge"
        );
        assert!(
            !state
                .resize_starts
                .contains_key(&shelf_id.with(ShelfEdge::Right)),
            "moving a shelf should clear stale resize capture on the target edge"
        );
    }
}
