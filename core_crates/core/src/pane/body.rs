//! Typed pane-body API.
//!
//! A pane is allowed to host *containers* and nothing else. This
//! module enforces that with two pieces:
//!
//! * [`ContainerSpec`] — one container ready to render. Its body
//!   is one of a fixed set of kinds (a pod list, a tab list, or —
//!   `pub(crate)` only — a raw closure used by frost's own extras
//!   to host non-`'static` content like the node graph). External
//!   callers can only build specs through the typed `normal` /
//!   `tabbed` constructors.
//!
//! * [`PaneBody`] — the typed wrapper the [`super::Pane::show`]
//!   closure receives. It collects [`ContainerSpec`]s through
//!   `add_normal` / `add_tabbed` / `add` and hands them off to
//!   [`render_containers`] when the closure returns. No raw egui
//!   [`Ui`] is ever exposed to the user.

use std::collections::HashMap;

use egui::{Color32, Id, Ui};

use crate::container::{Normal, SeparatorOrient, Tab, container_flow, set_container_flow};
use crate::pod::{Pod, PodResponse};

use super::{PaneAnchor, active_drag, paint_container_dots, section_order_for};

/// One container, ready to render inside a pane.
///
/// Construct through the typed entry points:
///
/// * [`ContainerSpec::normal`] — single-body container hosting a
///   `Vec<Pod>`.
/// * [`ContainerSpec::tabbed`] — folder-tabbed container hosting a
///   `Vec<Tab>`.
///
/// Anything else (raw egui closures) lives behind `pub(crate)`
/// constructors used only by `frost_core::extras::*` to host
/// host-widget integrations (node graph / code editor) without
/// leaking arbitrary-closure access to consumer code.
pub struct ContainerSpec<'a> {
    id: Id,
    title: String,
    icon: &'static str,
    body: SpecBody<'a>,
}

/// Internal body kind for a [`ContainerSpec`]. `Raw` is
/// `pub(crate)` so external callers can never construct a container
/// whose body is an arbitrary egui closure.
pub(crate) enum SpecBody<'a> {
    Pods(Vec<Pod>),
    Tabs(Vec<Tab>),
    Raw(Box<dyn FnOnce(&mut Ui) + 'a>),
}

impl<'a> ContainerSpec<'a> {
    /// Single-body container hosting a pod list.
    #[must_use]
    pub fn normal(
        id: impl Into<Id>,
        title: impl Into<String>,
        icon: &'static str,
        pods: Vec<Pod>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon,
            body: SpecBody::Pods(pods),
        }
    }

    /// Folder-tabbed container hosting a tab list.
    #[must_use]
    pub fn tabbed(
        id: impl Into<Id>,
        title: impl Into<String>,
        icon: &'static str,
        tabs: Vec<Tab>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon,
            body: SpecBody::Tabs(tabs),
        }
    }

    /// Crate-internal raw-closure constructor. Used by
    /// `frost_core::extras::*` to wrap host-widget integrations
    /// (node graph, code editor) that need non-`'static` borrows.
    /// Not reachable from outside `frost_core`.
    #[must_use]
    pub(crate) fn raw_internal<F>(
        id: impl Into<Id>,
        title: impl Into<String>,
        icon: &'static str,
        body: F,
    ) -> Self
    where
        F: FnOnce(&mut Ui) + 'a,
    {
        Self {
            id: id.into(),
            title: title.into(),
            icon,
            body: SpecBody::Raw(Box::new(body)),
        }
    }

    /// The stable container id (used by reorder persistence + the
    /// pod response map).
    #[must_use]
    pub fn container_id(&self) -> Id {
        self.id
    }
}

/// Typed wrapper around a pane's body Ui. Only exposes operations
/// that add containers — there is no way to get at the inner
/// [`egui::Ui`] from outside `frost_core`, so the closure body
/// passed to [`super::Pane::show`] cannot paint raw egui widgets.
///
/// Imperative builder: call [`add_normal`](Self::add_normal),
/// [`add_tabbed`](Self::add_tabbed), or the generic
/// [`add`](Self::add) any number of times. Containers paint in the
/// order returned by [`section_order_for`] (so the user's
/// drag-reorder persists across frames), not in call order.
pub struct PaneBody<'ui, 'spec> {
    ui: &'ui mut Ui,
    pane_id: Id,
    anchor: PaneAnchor,
    accent: Color32,
    pending: Vec<ContainerSpec<'spec>>,
}

impl<'ui, 'spec> PaneBody<'ui, 'spec> {
    pub(crate) fn new(ui: &'ui mut Ui, pane_id: Id, anchor: PaneAnchor, accent: Color32) -> Self {
        Self {
            ui,
            pane_id,
            anchor,
            accent,
            pending: Vec::new(),
        }
    }

    /// The anchor of the pane this body is in.
    #[must_use]
    pub fn anchor(&self) -> PaneAnchor {
        self.anchor
    }

    /// The accent colour the pane was built with.
    #[must_use]
    pub fn accent(&self) -> Color32 {
        self.accent
    }

    /// The pane's stable id.
    #[must_use]
    pub fn pane_id(&self) -> Id {
        self.pane_id
    }

    /// Direct access to the underlying egui context — useful for
    /// reading persisted state or sending viewport commands. There
    /// is intentionally **no** `ui()` accessor; the typed wrapper
    /// is the only way to paint into the pane.
    #[must_use]
    pub fn ctx(&self) -> &egui::Context {
        self.ui.ctx()
    }

    /// Append a normal container (single body, pod list).
    pub fn add_normal(
        &mut self,
        id: impl Into<Id>,
        title: impl Into<String>,
        icon: &'static str,
        pods: Vec<Pod>,
    ) -> &mut Self {
        self.pending
            .push(ContainerSpec::normal(id, title, icon, pods));
        self
    }

    /// Append a folder-tabbed container.
    pub fn add_tabbed(
        &mut self,
        id: impl Into<Id>,
        title: impl Into<String>,
        icon: &'static str,
        tabs: Vec<Tab>,
    ) -> &mut Self {
        self.pending
            .push(ContainerSpec::tabbed(id, title, icon, tabs));
        self
    }

    /// Append a pre-built [`ContainerSpec`]. Useful when an
    /// extension (e.g. `frost_core::extras::graph`) provides a
    /// typed constructor that returns a `ContainerSpec` for you
    /// to forward in.
    pub fn add(&mut self, spec: ContainerSpec<'spec>) -> &mut Self {
        self.pending.push(spec);
        self
    }

    /// Paint every container queued so far and return the per-
    /// container pod-response map. Useful when a pane body needs
    /// to wire `PodResponse` changes back into Bevy `Resource`s
    /// or eframe app state **inside the same closure** (e.g. a
    /// theme picker that updates an `AccentColor` resource from
    /// the colour pod). After the call, the queue is empty and
    /// further `add_*` calls accumulate again. `Pane::show`
    /// invokes `render` (via the crate-internal `finish`) once
    /// after the closure returns, so an unconsumed queue is
    /// painted automatically.
    pub fn render(&mut self) -> HashMap<Id, Vec<PodResponse>> {
        let specs = std::mem::take(&mut self.pending);
        render_containers(self.ui, self.pane_id, self.anchor, self.accent, specs)
    }

    /// Crate-internal: drain any remaining containers and return
    /// their pod-response maps. Called by `Pane::show` once the
    /// user's body closure returns.
    pub(crate) fn finish(mut self) -> HashMap<Id, Vec<PodResponse>> {
        self.render()
    }
}

/// Render a stack of containers inside a pane body — same layout
/// the demo's old `render_containers` performed, now owned by
/// `frost_core` so every consumer gets identical behaviour:
///
/// * Containers paint in the order from [`section_order_for`] so
///   drag-reorder persists.
/// * Between containers (and after the last), [`paint_container_dots`]
///   paints the three-dot drag handle.
/// * Dragging a handle updates the persisted container flow via
///   [`set_container_flow`]; folded containers ignore drag so the
///   user can't silently grow / shrink an invisible region.
pub(crate) fn render_containers<'a>(
    body_ui: &mut Ui,
    pane_id: Id,
    anchor: PaneAnchor,
    accent: Color32,
    containers: Vec<ContainerSpec<'a>>,
) -> HashMap<Id, Vec<PodResponse>> {
    let defaults: Vec<Id> = containers.iter().map(|c| c.id).collect();
    let order = section_order_for(body_ui.ctx(), pane_id, &defaults);
    let mut by_id: HashMap<Id, ContainerSpec<'a>> =
        containers.into_iter().map(|c| (c.id, c)).collect();

    // ── Tab pool for cross-container tab transfer ──
    //
    // Drain every `SpecBody::Tabs` into a single pool keyed by
    // `tab_id` so the tab-drag router can pull tabs out by id at
    // render time regardless of which container originally declared
    // them. Non-tab specs stay in `by_id` and render unchanged.
    let mut tab_pool: HashMap<Id, crate::container::Tab> = HashMap::new();
    let mut tabbed_specs: HashMap<Id, (String, &'static str)> = HashMap::new();
    let mut declared_tabs_per_container: HashMap<Id, Vec<Id>> = HashMap::new();
    let mut all_tabs_in_pane: Vec<(Id, Id)> = Vec::new();
    let cids_with_tabs: Vec<Id> = by_id
        .iter()
        .filter_map(|(id, spec)| matches!(spec.body, SpecBody::Tabs(_)).then_some(*id))
        .collect();
    for cid in cids_with_tabs {
        if let Some(spec) = by_id.remove(&cid) {
            let ContainerSpec {
                title, icon, body, ..
            } = spec;
            if let SpecBody::Tabs(tabs) = body {
                tabbed_specs.insert(cid, (title, icon));
                let mut ids = Vec::with_capacity(tabs.len());
                for tab in tabs {
                    let tid = tab.id();
                    ids.push(tid);
                    all_tabs_in_pane.push((tid, cid));
                    tab_pool.insert(tid, tab);
                }
                declared_tabs_per_container.insert(cid, ids);
            }
        }
    }

    let containers_stack_horizontally = !anchor.title_side().is_horizontal_strip();
    let dots_orient = if containers_stack_horizontally {
        SeparatorOrient::Vertical
    } else {
        SeparatorOrient::Horizontal
    };
    let title_at_end = anchor.title_side().is_at_end();
    let pane_horizontal_strip = anchor.title_side().is_horizontal_strip();

    let mut responses: HashMap<Id, Vec<PodResponse>> = HashMap::new();
    for cid in order.into_iter() {
        // Tabbed containers — pull routed tabs from the pool.
        if let Some((title, icon)) = tabbed_specs.remove(&cid) {
            let empty: Vec<Id> = Vec::new();
            let defaults_here = declared_tabs_per_container.get(&cid).unwrap_or(&empty);
            let routed_ids = super::tab_drag::route(
                body_ui.ctx(),
                pane_id,
                cid,
                defaults_here,
                &all_tabs_in_pane,
            );
            let mut routed_tabs: Vec<crate::container::Tab> = Vec::with_capacity(routed_ids.len());
            for tid in &routed_ids {
                if let Some(tab) = tab_pool.remove(tid) {
                    routed_tabs.push(tab);
                }
            }
            if routed_tabs.is_empty() {
                // No tabs land in this container after routing —
                // skip render entirely so an empty strip doesn't
                // paint a phantom container.
                continue;
            }
            let normal = Normal::new(title.as_str(), anchor, accent, cid).icon(icon);
            let resp = normal.show_tabs(body_ui, routed_tabs);
            responses.insert(cid, resp);
            let dragging_self = active_drag(body_ui.ctx())
                .and_then(|(_, s)| s.item)
                .map(|item| item == cid)
                .unwrap_or(false);
            if dragging_self {
                continue;
            }
            let dot_resp = paint_container_dots(body_ui, dots_orient, cid, accent);
            let body_open: bool = body_ui.ctx().data_mut(|d| {
                d.get_persisted::<bool>(cid.with("body_open"))
                    .unwrap_or(true)
            });
            if dot_resp.dragged() && body_open {
                let cur = container_flow(body_ui.ctx(), cid, pane_horizontal_strip);
                let raw = if containers_stack_horizontally {
                    dot_resp.drag_delta().x
                } else {
                    dot_resp.drag_delta().y
                };
                let delta = if title_at_end { -raw } else { raw };
                set_container_flow(body_ui.ctx(), cid, cur + delta, pane_horizontal_strip);
            }
            continue;
        }

        let Some(spec) = by_id.remove(&cid) else {
            continue;
        };
        let normal = Normal::new(spec.title.as_str(), anchor, accent, cid).icon(spec.icon);
        let resp = match spec.body {
            SpecBody::Pods(pods) => normal.show(body_ui, pods),
            SpecBody::Tabs(_tabs) => {
                // Tabs are handled by the tab-pool branch above; this
                // arm is unreachable because the pool drained every
                // `SpecBody::Tabs`. Keep the match exhaustive.
                Vec::new()
            }
            SpecBody::Raw(body) => {
                normal.show_raw(body_ui, body);
                // Raw bodies don't produce pod responses — return
                // an empty Vec so the response map stays consistent.
                Vec::new()
            }
        };
        responses.insert(cid, resp);

        // Skip the dot handle while THIS container is being
        // drag-reordered — the floating preview already paints a
        // copy with its handle.
        let dragging_self = active_drag(body_ui.ctx())
            .and_then(|(_, s)| s.item)
            .map(|item| item == cid)
            .unwrap_or(false);
        if dragging_self {
            continue;
        }

        let dot_resp = paint_container_dots(body_ui, dots_orient, cid, accent);
        let body_open: bool = body_ui.ctx().data_mut(|d| {
            d.get_persisted::<bool>(cid.with("body_open"))
                .unwrap_or(true)
        });
        if dot_resp.dragged() && body_open {
            let cur = container_flow(body_ui.ctx(), cid, pane_horizontal_strip);
            let raw = if containers_stack_horizontally {
                dot_resp.drag_delta().x
            } else {
                dot_resp.drag_delta().y
            };
            let delta = if title_at_end { -raw } else { raw };
            set_container_flow(body_ui.ctx(), cid, cur + delta, pane_horizontal_strip);
        }
    }
    responses
}
