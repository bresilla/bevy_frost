//! Persistent docked Shelves.
//!
//! A Shelf is structural chrome: it reserves space on the left,
//! right, or bottom side of the workspace and hosts typed tabbed
//! containers. It is deliberately not a ribbon-opened floating
//! [`crate::pane::Pane`].

use std::collections::HashMap;

use egui::{Color32, Id, Rect, Sense, Stroke, UiBuilder, Vec2, pos2, vec2};

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
    pub fn container(mut self, container: ShelfContainer<'a>) -> Self {
        self.containers.push(container);
        self
    }

    #[must_use]
    pub fn containers(mut self, containers: impl IntoIterator<Item = ShelfContainer<'a>>) -> Self {
        self.containers.extend(containers);
        self
    }

    fn default_extent(&self, theme: &ShelfTheme) -> f32 {
        self.default_size.unwrap_or(if self.edge.is_side() {
            theme.side_default_size
        } else {
            theme.bottom_default_size
        })
    }

    fn min_extent(&self, theme: &ShelfTheme) -> f32 {
        self.min_size.unwrap_or(theme.min_size)
    }

    fn max_extent(&self, theme: &ShelfTheme) -> f32 {
        self.max_size.unwrap_or(theme.max_size)
    }
}

/// Persistent Shelf UI state: user sizes and per-Shelf active group.
#[derive(Debug, Default)]
pub struct ShelfState {
    sizes: HashMap<Id, f32>,
    active_containers: HashMap<Id, Id>,
}

impl ShelfState {
    #[must_use]
    pub fn size(&self, shelf_id: Id) -> Option<f32> {
        self.sizes.get(&shelf_id).copied()
    }

    pub fn set_size(&mut self, shelf_id: Id, size: f32) {
        self.sizes.insert(shelf_id, size);
    }

    #[must_use]
    pub fn active_container(&self, shelf_id: Id) -> Option<Id> {
        self.active_containers.get(&shelf_id).copied()
    }

    pub fn set_active_container(&mut self, shelf_id: Id, container_id: Id) {
        self.active_containers.insert(shelf_id, container_id);
    }

    fn extent_for(&mut self, shelf: &ShelfDef<'_>, theme: &ShelfTheme) -> f32 {
        let min = shelf.min_extent(theme);
        let max = shelf.max_extent(theme);
        let value = self
            .sizes
            .entry(shelf.id)
            .or_insert_with(|| shelf.default_extent(theme).clamp(min, max));
        *value = value.clamp(min, max);
        *value
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
}

/// Reserve structural Shelf space and return the remaining viewport.
pub fn layout_shelves(
    available: Rect,
    shelves: &[ShelfDef<'_>],
    state: &mut ShelfState,
    theme: &ShelfTheme,
) -> ShelfLayout {
    let mut viewport = available;
    let mut left = None;
    let mut right = None;
    let mut bottom = None;

    for shelf in shelves {
        let extent = state.extent_for(shelf, theme);
        match shelf.edge {
            ShelfEdge::Left => {
                let rect =
                    Rect::from_min_max(viewport.min, pos2(viewport.min.x + extent, viewport.max.y));
                viewport.min.x = (viewport.min.x + extent).min(viewport.max.x);
                left = Some(rect);
            }
            ShelfEdge::Right => {
                let rect =
                    Rect::from_min_max(pos2(viewport.max.x - extent, viewport.min.y), viewport.max);
                viewport.max.x = (viewport.max.x - extent).max(viewport.min.x);
                right = Some(rect);
            }
            ShelfEdge::Bottom => {
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

/// Paint all Shelves and their typed tabbed containers.
pub fn show_shelves<'a>(
    ctx: &egui::Context,
    layout: ShelfLayout,
    shelves: Vec<ShelfDef<'a>>,
    state: &mut ShelfState,
) {
    publish_shelf_layout(ctx, layout);
    let theme = style::theme();
    let shelf_theme = *theme.shelf();

    for shelf in shelves {
        let Some(rect) = layout.rect_for(shelf.edge) else {
            continue;
        };

        let area = egui::Area::new(shelf.id.with("frost_shelf_area"))
            .order(egui::Order::Middle)
            .fixed_pos(rect.min)
            .interactable(true);

        area.show(ctx, |ui| {
            let shelf_rect = Rect::from_min_size(ui.min_rect().min, rect.size());
            ui.set_min_size(rect.size());
            ui.interact(shelf_rect, shelf.id.with("background"), Sense::hover());
            paint_shelf_background(ui, shelf_rect, shelf.accent, &shelf_theme);
            resize_shelf(ui, &shelf, state, &shelf_theme, shelf_rect);

            let content_rect = shelf_rect.shrink(shelf_theme.padding);
            render_shelf_body(ui, content_rect, shelf, state);
        });
    }
}

fn render_shelf_body<'a>(
    ui: &mut egui::Ui,
    content_rect: Rect,
    shelf: ShelfDef<'a>,
    state: &mut ShelfState,
) {
    let pane_id = shelf.id.with("shelf_pane_scope");
    let anchor = shelf.edge.container_anchor();
    // Container stack axis mirrors `Pane::lay_out_flex`: vertical-
    // strip title sides stack containers horizontally, horizontal-
    // strip title sides stack them vertically. The previous
    // implementation hard-coded `top_down` for every Shelf edge, so
    // the Bottom shelf stacked containers vertically when they
    // should flow horizontally — and the drag ghost-gap allocated
    // along the wrong axis as a result.
    let horizontal_stack = !anchor.title_side().is_horizontal_strip();
    let layout = if horizontal_stack {
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
    let mut viewport = ui.new_child(UiBuilder::new().max_rect(content_rect).layout(layout));
    // Zero item spacing so the ghost gap sits flush against
    // neighbouring containers (matches `Pane::lay_out_flex`).
    viewport.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

    pane::begin_drag_frame(viewport.ctx(), pane_id);
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

    let specs = shelf
        .containers
        .into_iter()
        .map(|container| container.spec)
        .collect();
    let responses =
        crate::pane::render_containers(&mut viewport, pane_id, anchor, shelf.accent, specs);

    if let Some(container_id) = responses.keys().next().copied() {
        state.set_active_container(shelf.id, container_id);
    }

    // ── Trailing ghost gap ──
    //
    // Same logic as `Pane::lay_out_flex`: when the cursor's slot is
    // past the last rendered container, paint the gap inline at the
    // end of the viewport so the trailing drop position is visible.
    let drag_state = pane::drag_state(viewport.ctx(), pane_id);
    if let Some(dragged_id) = drag_state.item {
        let snap = pane::snapshot(viewport.ctx(), pane_id);
        let total = pane::current_cache(viewport.ctx(), pane_id).len();
        let cursor = viewport.ctx().pointer_interact_pos().or(drag_state.cursor);
        if let Some(c) = cursor {
            let cursor_axis = if horizontal_stack { c.x } else { c.y };
            let target_idx = pane::compute_target(&snap, dragged_id, cursor_axis, horizontal_stack);
            if target_idx >= total {
                if let Some(entry) = pane::dragged_entry(&snap, dragged_id) {
                    pane::paint_ghost_gap_entry_inline(
                        &mut viewport,
                        entry,
                        shelf.accent,
                        horizontal_stack,
                    );
                }
            }
        }
    }

    pane::finalize_snapshot(viewport.ctx(), pane_id);

    if let Some(dragged_id) = drag_state.item {
        let snap = pane::snapshot(viewport.ctx(), pane_id);
        let cursor = viewport.ctx().pointer_interact_pos().or(drag_state.cursor);
        if let Some(c) = cursor {
            pane::paint_drag_preview(viewport.ctx(), pane_id, &snap, dragged_id, c, shelf.accent);
            viewport.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }

        if viewport.ctx().input(|i| i.pointer.any_released()) {
            if let Some(c) = cursor {
                let cursor_axis = if horizontal_stack { c.x } else { c.y };
                let target_idx =
                    pane::compute_target(&snap, dragged_id, cursor_axis, horizontal_stack);
                let defaults: Vec<Id> = snap.iter().map(|e| e.id).collect();
                let mut order = pane::section_order_for(viewport.ctx(), pane_id, &defaults);
                order.retain(|cid| *cid != dragged_id);
                let clamped = target_idx.min(order.len());
                order.insert(clamped, dragged_id);
                pane::set_section_order(viewport.ctx(), pane_id, order);
            }
            pane::clear_drag(viewport.ctx(), pane_id);
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
                None,
            );
            viewport.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        if viewport.ctx().input(|i| i.pointer.any_released()) {
            if let Some(c) = cursor {
                if let Some((tgt_cid, slot)) =
                    pane::tab_drag::find_drop_target(viewport.ctx(), pane_id, c)
                {
                    pane::tab_drag::commit_drop(
                        viewport.ctx(),
                        pane_id,
                        tab_drag_state.tab_id,
                        tab_drag_state.source_container,
                        tgt_cid,
                        slot,
                    );
                }
            }
            pane::tab_drag::clear_drag(viewport.ctx(), pane_id);
        }
    }
}

/// Publish the post-Shelf viewport as the chrome bounds for floating
/// ribbons/panes. Call this before drawing ribbons when Shelves are
/// present; [`show_shelves`] does it automatically.
pub fn publish_shelf_layout(ctx: &egui::Context, layout: ShelfLayout) {
    ctx.data_mut(|d| {
        d.insert_temp(
            crate::ribbon::assembly::chrome_bounds_key(),
            layout.viewport,
        );
    });
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
    state: &mut ShelfState,
    theme: &ShelfTheme,
    rect: Rect,
) {
    let thickness = theme.resize_handle_thickness;
    let handle = match shelf.edge {
        ShelfEdge::Left => Rect::from_min_size(
            pos2(rect.max.x - thickness, rect.min.y),
            vec2(thickness, rect.height()),
        ),
        ShelfEdge::Right => Rect::from_min_size(rect.min, vec2(thickness, rect.height())),
        ShelfEdge::Bottom => Rect::from_min_size(rect.min, vec2(rect.width(), thickness)),
    };
    let resp = ui.interact(handle, shelf.id.with("resize"), Sense::drag());
    if resp.dragged() {
        let cur = state
            .size(shelf.id)
            .unwrap_or_else(|| shelf.default_extent(theme));
        let raw_delta = match shelf.edge {
            ShelfEdge::Left => resp.drag_delta().x,
            ShelfEdge::Right => -resp.drag_delta().x,
            ShelfEdge::Bottom => -resp.drag_delta().y,
        };
        let next = (cur + raw_delta).clamp(shelf.min_extent(theme), shelf.max_extent(theme));
        state.set_size(shelf.id, next);
        ui.ctx().request_repaint();
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
