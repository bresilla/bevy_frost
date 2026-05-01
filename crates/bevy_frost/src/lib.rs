//! # bevy_frost — Bevy integration for the frost UI kit.
//!
//! All UI primitives (widgets, ribbons, floating panels, node-graph
//! wrapper, code editor, theme) live in the framework-agnostic
//! [`frostcore`] crate. This crate adds:
//!
//! * [`FrostPlugin`] — one-line install that registers frostcore's
//!   state types as Bevy `Resource`s and runs the theme + ghost
//!   systems every frame.
//! * [`ThemePlugin`] / [`RibbonPlugin`] — granular alternatives if
//!   you want just one piece.
//! * [`GizmoMaterial`] — always-on-top transform-gizmo material
//!   extension (Bevy-specific).
//!
//! Consumers using `use bevy_frost::prelude::*;` keep the same API
//! they had before the workspace split — this crate re-exports
//! everything from `frostcore` verbatim and adds the plugins on top.
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_frost::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(bevy_egui::EguiPlugin::default())
//!         .add_plugins(FrostPlugin)
//!         .run();
//! }
//! ```

pub mod gizmo_material;
pub mod prelude;

// Re-export all of frostcore under `bevy_frost::*` so existing
// consumers don't notice the workspace split.
pub use frostcore::*;

use bevy::ecs::message::Messages;
use bevy::input::mouse::{MouseButtonInput, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPreUpdateSet, EguiPrimaryContextPass};

// ─── Theme ──────────────────────────────────────────────────────────

/// Registers [`frostcore::AccentColor`] + [`frostcore::GlassOpacity`]
/// as Bevy resources and runs [`frostcore::apply_theme`] every frame.
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<frostcore::AccentColor>()
            .init_resource::<frostcore::GlassOpacity>()
            .add_systems(PreUpdate, sync_glass_opacity_system)
            .add_systems(EguiPrimaryContextPass, apply_theme_system);
    }
}

fn sync_glass_opacity_system(opacity: Res<frostcore::GlassOpacity>) {
    frostcore::set_glass_opacity(opacity.0);
}

fn apply_theme_system(
    mut contexts: EguiContexts,
    accent: Res<frostcore::AccentColor>,
    opacity: Res<frostcore::GlassOpacity>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    frostcore::apply_theme(ctx, *accent, *opacity);
}

// ─── Ribbons ────────────────────────────────────────────────────────

/// SystemSet the ribbon ghost paint lives in. Downstream plugins
/// can pin their own ribbon-painting panels `.before(RibbonGhostSet)`
/// to keep the ghost on top of their ribbons.
#[derive(SystemSet, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct RibbonGhostSet;

/// Registers the ribbon `Resource`s and the ghost drop-preview
/// system. [`FrostPlugin`] installs this transitively.
pub struct RibbonPlugin;

impl Plugin for RibbonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<frostcore::RibbonLayout>()
            .init_resource::<frostcore::SideActive>()
            .init_resource::<frostcore::RibbonOpen>()
            .init_resource::<frostcore::RibbonWidth>()
            .init_resource::<frostcore::RibbonPlacement>()
            .init_resource::<frostcore::RibbonDrag>()
            .configure_sets(
                EguiPrimaryContextPass,
                RibbonGhostSet.after(apply_theme_system),
            )
            .add_systems(
                EguiPrimaryContextPass,
                paint_drop_ghost_system.in_set(RibbonGhostSet),
            )
            .add_systems(EguiPrimaryContextPass, debug_toggle_system);
    }
}

fn paint_drop_ghost_system(
    mut contexts: EguiContexts,
    layout: Res<frostcore::RibbonLayout>,
    accent: Res<frostcore::AccentColor>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    frostcore::paint_drop_ghost(ctx, &*layout, *accent);
}

/// **F12** — toggle egui's "show interactive widget bounds" overlay.
/// Renders a colored outline around every widget egui knows about,
/// plus the layout rects driving it. Use this to show the dev where
/// a layout is breaking. Bound globally on the primary egui ctx.
///
/// `Style.debug` is `#[cfg(debug_assertions)]`-gated by egui itself,
/// so this toggle only compiles in debug builds. `make run` runs
/// `--release` — to use F12, run a debug build (e.g. `cargo run -p
/// bevy_frost --example demo`) or override `debug-assertions = true`
/// in the workspace release profile.
fn debug_toggle_system(mut contexts: EguiContexts) {
    let Ok(_ctx) = contexts.ctx_mut() else { return };
    #[cfg(debug_assertions)]
    {
        let pressed = _ctx.input_mut(|i| {
            i.consume_key(bevy_egui::egui::Modifiers::NONE, bevy_egui::egui::Key::F12)
        });
        if pressed {
            _ctx.style_mut(|s| {
                s.debug.show_interactive_widgets = !s.debug.show_interactive_widgets;
                s.debug.show_widget_hits = s.debug.show_interactive_widgets;
            });
        }
    }
}

// ─── Pointer-event firewall ────────────────────────────────────────

/// Drains every relevant pointer input message whenever the OS
/// cursor sits inside the painted rect of any frost pane this
/// frame, so neither scroll, clicks, nor polled mouse-button state
/// bleed through to downstream Bevy systems (e.g. `bevy_glacial`'s
/// chase-camera zoom, a viewport ray-pick that uses
/// `mouse.just_pressed(...)`, a drag-to-pan handler reading
/// `MouseButtonInput`, etc.).
///
/// We DON'T use egui's `is_pointer_over_area` / `layer_id_at`:
/// the former returns `false` for `Order::Background` layers when
/// no `CentralPanel` is installed (frost panes are Background and
/// we have no CentralPanel), and the latter has edge cases around
/// modal / tooltip layers that fire for cursor positions visually
/// over the 3D viewport. Instead, `corekit::pane::Pane2::show`
/// publishes its painted rect to a global ctx-data list each frame
/// (see [`corekit::pane::published_pane_rects`]) and we just check
/// the bevy window's cursor against that list.
///
/// Clearing happens AFTER `EguiPreUpdateSet::ProcessInput` so
/// bevy_egui's input forwarder has already copied the events into
/// egui's own `EguiInput` — the UI keeps responding to clicks /
/// scrolls normally, only Bevy-side consumers see the queue
/// emptied. Polled state (`ButtonInput<MouseButton>`) is also
/// reset so code like `mouse.just_pressed(MouseButton::Left)`
/// inside a 3D-viewport system doesn't fire when the click was
/// actually delivered to the UI.
fn consume_egui_input_system(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut contexts: EguiContexts,
    mut wheel_events: ResMut<Messages<MouseWheel>>,
    mut button_events: ResMut<Messages<MouseButtonInput>>,
    mut mouse_buttons: ResMut<ButtonInput<MouseButton>>,
) {
    let Ok(window) = primary_window.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let pos = egui::pos2(cursor.x, cursor.y);
    let pane_rects = corekit::pane::published_pane_rects(ctx);
    if pane_rects.iter().any(|r| r.contains(pos)) {
        wheel_events.clear();
        button_events.clear();
        mouse_buttons.reset_all();
    }
}

/// Standalone plugin that installs only the egui pointer-event
/// firewall — useful for apps that can't take the full
/// [`FrostPlugin`] (e.g. apps already wiring their own theme +
/// ribbon resources from a different source). Add this alone
/// alongside `EguiPlugin` and you get the same input-absorption
/// behaviour without dragging in `ThemePlugin` / `RibbonPlugin`.
pub struct EguiInputAbsorbPlugin;

impl Plugin for EguiInputAbsorbPlugin {
    fn build(&self, app: &mut App) {
        // Run `.after(EguiPreUpdateSet::ProcessInput)` — bevy_egui's
        // set that copies `Messages<MouseWheel>` into egui's
        // `EguiInput`. If we cleared the queue earlier the UI would
        // miss the scroll entirely. After this set, egui has its
        // copy and we're free to drain so downstream `Update` systems
        // (e.g. bevy_glacial's chase-camera zoom) see nothing.
        app.add_systems(
            PreUpdate,
            consume_egui_input_system.after(EguiPreUpdateSet::ProcessInput),
        );
    }
}

// ─── Combined install ──────────────────────────────────────────────

/// Full frost install — `ThemePlugin` + `RibbonPlugin` +
/// [`EguiInputAbsorbPlugin`]. Idempotent; safe to add alongside any
/// other Bevy plugins.
pub struct FrostPlugin;

impl Plugin for FrostPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        if !app.is_plugin_added::<RibbonPlugin>() {
            app.add_plugins(RibbonPlugin);
        }
        if !app.is_plugin_added::<EguiInputAbsorbPlugin>() {
            app.add_plugins(EguiInputAbsorbPlugin);
        }
    }
}
