//! Host-agnostic app shell helpers.
//!
//! The app shell coordinates permanent ribbons, the active top-level
//! view, and the active view's workspace stack. This is deliberately
//! rendering-light for the first implementation: it resolves slot
//! contents and dispatches actions, while host crates decide how to
//! paint the resolved items.

use egui::{Color32, Id};

use crate::{
    WorkspaceCtx,
    ribbon::{
        ResolvedSlotRibbon, RibbonAction, RibbonActionError, RibbonActionResult,
        RibbonOverrideLayer, RibbonScope, RibbonSlotDef, RibbonSlotItem, dispatch_ribbon_action,
        draw_slot_ribbons, resolve_slot_items, restore_workspace_slot_override,
    },
    view::{ViewCtx, ViewRouter, ViewRouterError},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedRibbon {
    pub id: Id,
    pub scope: RibbonScope,
    pub edge: crate::ribbon::RibbonEdge,
    pub cluster: crate::ribbon::RibbonCluster,
    pub items: Vec<RibbonSlotItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppShellResolution {
    pub ribbons: Vec<ResolvedRibbon>,
}

/// API-level app chrome contract.
///
/// Frost has exactly one persistent main bar. Hosts construct an
/// `AppShellChrome` with that bar first, then append optional
/// permanent bars. Active views/workspaces may override or explicitly
/// hide individual slots, but they do not rebuild the main bar by
/// passing a different arbitrary ribbon list each frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppShellChrome {
    main_bar: RibbonSlotDef,
    permanent_ribbons: Vec<RibbonSlotDef>,
}

impl AppShellChrome {
    #[must_use]
    pub fn new(mut main_bar: RibbonSlotDef) -> Self {
        main_bar.scope = RibbonScope::Permanent;
        Self {
            main_bar,
            permanent_ribbons: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_permanent_ribbon(mut self, mut ribbon: RibbonSlotDef) -> Self {
        ribbon.scope = RibbonScope::Permanent;
        self.permanent_ribbons.push(ribbon);
        self
    }

    pub fn push_permanent_ribbon(&mut self, mut ribbon: RibbonSlotDef) {
        ribbon.scope = RibbonScope::Permanent;
        self.permanent_ribbons.push(ribbon);
    }

    #[must_use]
    pub fn main_bar(&self) -> &RibbonSlotDef {
        &self.main_bar
    }

    #[must_use]
    pub fn permanent_ribbons(&self) -> &[RibbonSlotDef] {
        &self.permanent_ribbons
    }

    #[must_use]
    pub fn permanent_ribbon_defs(&self) -> Vec<RibbonSlotDef> {
        let mut out = Vec::with_capacity(1 + self.permanent_ribbons.len());
        out.push(self.main_bar.clone());
        out.extend(self.permanent_ribbons.iter().cloned());
        out
    }
}

#[derive(Debug)]
pub enum AppShellError {
    View(ViewRouterError),
    Action(RibbonActionError),
}

impl From<ViewRouterError> for AppShellError {
    fn from(value: ViewRouterError) -> Self {
        Self::View(value)
    }
}

impl From<RibbonActionError> for AppShellError {
    fn from(value: RibbonActionError) -> Self {
        Self::Action(value)
    }
}

impl From<ResolvedRibbon> for ResolvedSlotRibbon {
    fn from(value: ResolvedRibbon) -> Self {
        Self {
            id: value.id,
            scope: value.scope,
            edge: value.edge,
            cluster: value.cluster,
            items: value.items,
        }
    }
}

impl AppShellResolution {
    #[must_use]
    pub fn as_slot_ribbons(&self) -> Vec<ResolvedSlotRibbon> {
        self.ribbons.iter().cloned().map(Into::into).collect()
    }
}

/// Resolve permanent + active view ribbons into concrete slot items.
///
/// Override priority follows the PLAN:
///
/// ```text
/// deepest workspace > active view > permanent default
/// ```
///
/// For the current implementation, the active workspace contributes
/// a built-in `system.close_or_restore -> PopWorkspace` override
/// whenever the active view stack is at L1+.
pub fn resolve_app_shell_ribbons(
    router: &mut ViewRouter,
    permanent_ribbons: &[RibbonSlotDef],
) -> Result<AppShellResolution, AppShellError> {
    resolve_app_shell_ribbons_with_workspace_chrome(router, permanent_ribbons, &[], &[])
}

/// Resolve an API-enforced shell chrome contract.
///
/// Prefer this over passing ad-hoc permanent ribbon slices in new
/// hosts: it guarantees a single persistent main bar is always present
/// and first in resolution order.
pub fn resolve_app_shell_chrome(
    router: &mut ViewRouter,
    chrome: &AppShellChrome,
) -> Result<AppShellResolution, AppShellError> {
    resolve_app_shell_chrome_with_workspace(router, chrome, &[], &[])
}

/// Resolve API-enforced shell chrome with active workspace chrome.
pub fn resolve_app_shell_chrome_with_workspace(
    router: &mut ViewRouter,
    chrome: &AppShellChrome,
    workspace_ribbons: &[RibbonSlotDef],
    workspace_layers: &[RibbonOverrideLayer],
) -> Result<AppShellResolution, AppShellError> {
    let permanent_ribbons = chrome.permanent_ribbon_defs();
    resolve_app_shell_ribbons_with_workspace_chrome(
        router,
        &permanent_ribbons,
        workspace_ribbons,
        workspace_layers,
    )
}

/// Like [`resolve_app_shell_ribbons`] but lets the active
/// workspace/module provide additional override layers.
///
/// `workspace_layers` should be ordered shallowest to deepest. They
/// are applied after the active view override layer. When the active
/// workspace is L1+, the built-in restore override is inserted before
/// caller-provided workspace layers, so a module can deliberately
/// replace even that slot if needed.
pub fn resolve_app_shell_ribbons_with_workspace_layers(
    router: &mut ViewRouter,
    permanent_ribbons: &[RibbonSlotDef],
    workspace_layers: &[RibbonOverrideLayer],
) -> Result<AppShellResolution, AppShellError> {
    resolve_app_shell_ribbons_with_workspace_chrome(
        router,
        permanent_ribbons,
        &[],
        workspace_layers,
    )
}

/// Resolve permanent + view-local + workspace-local ribbons.
///
/// `workspace_ribbons` are supplied by the active L1/L2 module
/// workspace renderer (usually collected through [`crate::WorkspaceCtx`]).
/// They participate only when their [`RibbonScope::WorkspaceLevel`]
/// matches the active workspace id.
pub fn resolve_app_shell_ribbons_with_workspace_chrome(
    router: &mut ViewRouter,
    permanent_ribbons: &[RibbonSlotDef],
    workspace_ribbons: &[RibbonSlotDef],
    workspace_layers: &[RibbonOverrideLayer],
) -> Result<AppShellResolution, AppShellError> {
    let active_view_id = router.active()?;
    let active_depth = router.active_workspace()?.depth();

    let (view_ribbons, view_overrides) = {
        let entry = router.active_entry_mut()?;
        (entry.view.ribbons(), entry.view.ribbon_overrides())
    };

    let mut layers = Vec::new();
    layers.push(view_overrides);
    if active_depth > 0 {
        layers.push(RibbonOverrideLayer::new(vec![
            restore_workspace_slot_override(),
        ]));
    }
    layers.extend_from_slice(workspace_layers);

    let mut resolved = AppShellResolution::default();
    for ribbon in permanent_ribbons
        .iter()
        .chain(view_ribbons.iter())
        .chain(workspace_ribbons.iter())
        .filter(|ribbon| match ribbon.scope {
            RibbonScope::Permanent => true,
            RibbonScope::View(id) => id == active_view_id,
            RibbonScope::WorkspaceLevel(id) => router
                .active_workspace()
                .map(|workspace| workspace.current().id == id)
                .unwrap_or(false),
        })
    {
        let items = ribbon
            .slots
            .iter()
            .flat_map(|slot| resolve_slot_items(slot, &layers))
            .collect();
        resolved.ribbons.push(ResolvedRibbon {
            id: ribbon.id,
            scope: ribbon.scope,
            edge: ribbon.edge,
            cluster: ribbon.cluster,
            items,
        });
    }

    Ok(resolved)
}

/// Dispatch a root-shell ribbon action.
pub fn dispatch_app_shell_action(
    router: &mut ViewRouter,
    action: RibbonAction,
) -> Result<RibbonActionResult, AppShellError> {
    Ok(dispatch_ribbon_action(action, router)?)
}

/// Minimal root render entry point.
///
/// This calls the active L0 view only when the active workspace stack
/// is at root. L1+ module workspace rendering will layer onto this
/// once modules can register workspace renderers.
pub fn show_app_shell(
    egui_ctx: &egui::Context,
    router: &mut ViewRouter,
    permanent_ribbons: &[RibbonSlotDef],
    accent: Color32,
) -> Result<AppShellResolution, AppShellError> {
    let resolved = resolve_app_shell_ribbons(router, permanent_ribbons)?;
    let depth = router.active_workspace()?.depth();
    if depth == 0 {
        let entry = router.active_entry_mut()?;
        let mut ctx = ViewCtx {
            egui_ctx,
            workspace: &mut entry.workspace,
            accent,
        };
        entry.view.show(&mut ctx);
    }
    Ok(resolved)
}

/// Resolve, paint, and dispatch slot-based app-shell ribbons, then
/// render the active L0 view when the active stack is at root.
pub fn show_app_shell_chrome_with_slot_ribbons(
    egui_ctx: &egui::Context,
    router: &mut ViewRouter,
    chrome: &AppShellChrome,
    accent: Color32,
) -> Result<(AppShellResolution, Vec<RibbonActionResult>), AppShellError> {
    let permanent_ribbons = chrome.permanent_ribbon_defs();
    show_app_shell_with_slot_ribbons(egui_ctx, router, &permanent_ribbons, accent)
}

pub fn show_app_shell_with_slot_ribbons(
    egui_ctx: &egui::Context,
    router: &mut ViewRouter,
    permanent_ribbons: &[RibbonSlotDef],
    accent: Color32,
) -> Result<(AppShellResolution, Vec<RibbonActionResult>), AppShellError> {
    let resolved = resolve_app_shell_ribbons(router, permanent_ribbons)?;
    let clicks = draw_slot_ribbons(egui_ctx, accent, &resolved.as_slot_ribbons());
    let mut results = Vec::with_capacity(clicks.len());
    for click in clicks {
        results.push(dispatch_app_shell_action(router, click.action)?);
    }

    let depth = router.active_workspace()?.depth();
    if depth == 0 {
        let entry = router.active_entry_mut()?;
        let mut ctx = ViewCtx {
            egui_ctx,
            workspace: &mut entry.workspace,
            accent,
        };
        entry.view.show(&mut ctx);
    }

    Ok((resolved, results))
}

/// Resolve, paint, dispatch, and render either the active L0 view or
/// the active L1+ module workspace through a host-supplied renderer.
///
/// The app shell does not own module instances, so the host maps
/// `WorkspaceCtx::level.owner` / module ids to the concrete active
/// module and calls its `workspace`/body renderer inside
/// `render_workspace`. Any ribbons or override layers added to the
/// [`WorkspaceCtx`] are then folded into the slot-resolution pass
/// before painting permanent/view/workspace chrome.
pub fn show_app_shell_with_workspace_renderer<F>(
    egui_ctx: &egui::Context,
    router: &mut ViewRouter,
    permanent_ribbons: &[RibbonSlotDef],
    accent: Color32,
    render_workspace: F,
) -> Result<(AppShellResolution, Vec<RibbonActionResult>), AppShellError>
where
    F: FnOnce(&egui::Context, &mut WorkspaceCtx<'_>),
{
    let depth = router.active_workspace()?.depth();
    let mut workspace_ribbons = Vec::new();
    let mut workspace_layers = Vec::new();

    if depth > 0 {
        let entry = router.active_entry_mut()?;
        let mut workspace_ctx = WorkspaceCtx::new(&mut entry.workspace, accent);
        render_workspace(egui_ctx, &mut workspace_ctx);
        workspace_ribbons.extend_from_slice(workspace_ctx.ribbons());
        workspace_layers.extend_from_slice(workspace_ctx.ribbon_overrides());
    }

    let resolved = resolve_app_shell_ribbons_with_workspace_chrome(
        router,
        permanent_ribbons,
        &workspace_ribbons,
        &workspace_layers,
    )?;
    let clicks = draw_slot_ribbons(egui_ctx, accent, &resolved.as_slot_ribbons());
    let mut results = Vec::with_capacity(clicks.len());
    for click in clicks {
        results.push(dispatch_app_shell_action(router, click.action)?);
    }

    if depth == 0 {
        let entry = router.active_entry_mut()?;
        let mut ctx = ViewCtx {
            egui_ctx,
            workspace: &mut entry.workspace,
            accent,
        };
        entry.view.show(&mut ctx);
    }

    Ok((resolved, results))
}
