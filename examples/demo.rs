//! `bevy_frost` widget gallery + layout showcase.
//!
//! What this demonstrates:
//!
//! * [`FrostPlugin`] install on top of a stock Bevy + `bevy_egui`
//!   app.
//! * An empty 3D scene (ground plane + accent cube + light) — a
//!   stand-in for whatever real application would render into the
//!   viewport.
//! * Left / Right [`SideRibbon`] rails, each with **multiple buttons
//!   that share one panel slot**: clicking any button on a side
//!   opens its panel *in place* and auto-closes whichever panel was
//!   already open on that side. Exclusivity is driven by the
//!   [`SideActive`] resource the crate already ships with.
//! * Every stateless widget module the crate ships — toggles, drag
//!   values, sliders, progress bars, colour pickers, buttons,
//!   subsections, readouts, the whole lot.
//!
//! Run with
//!
//! ```text
//! cargo run --example demo
//! ```
//!
//! or `make run` from the repo root once direnv has loaded the flake
//! (wraps `cargo run --example demo` in `nixVulkan`).

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use bevy_frost::prelude::*;

// Menu identifiers. One per button. `SideActive` holds "which of
// these is open on the left rail / on the right rail" (at most one
// each) — we just compare these constants against it.
const MENU_WIDGETS: &str = "demo_menu_widgets";
const MENU_CONTAINERS: &str = "demo_menu_containers";
const MENU_THEME: &str = "demo_menu_theme";
const MENU_ABOUT: &str = "demo_menu_about";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_frost demo".into(),
                resolution: (1280u32, 800u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(FrostPlugin)
        .init_resource::<DemoState>()
        .add_systems(Startup, setup_scene)
        .add_systems(EguiPrimaryContextPass, (draw_ribbons, draw_panels).chain())
        .run();
}

// ─── Demo state — the values the widgets actually bind to ───────────

#[derive(Resource)]
struct DemoState {
    // Sample values.
    power: bool,
    headlights: bool,
    gravity: f64,
    speed_limit: f64,
    engine_power: f64,
    throttle: f64,
    brake: f64,
    fuel_fraction: f32,
    tint: [f32; 3],
    position: [f64; 3],
    rotation_deg: [f64; 3],
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            power: true,
            headlights: false,
            gravity: 9.81,
            speed_limit: 24.0,
            engine_power: 180.0,
            throttle: 0.35,
            brake: 0.0,
            fuel_fraction: 0.72,
            tint: [0.65, 0.54, 0.98],
            position: [0.0, 1.5, 0.0],
            rotation_deg: [0.0, 45.0, 0.0],
        }
    }
}

// ─── Scene setup ────────────────────────────────────────────────────

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(50.0, 0.1, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.2, 0.22),
            perceptual_roughness: 0.95,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.05, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.65, 0.54, 0.98),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(6.0, 5.0, 8.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
    ));
}

// ─── Ribbons — menu-toggle buttons pinned to the edges ──────────────

fn draw_ribbons(
    mut contexts: EguiContexts,
    accent: Res<AccentColor>,
    mut side_active: ResMut<SideActive>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let accent_col = accent.0;

    // Two menus on the left rail — both open at the SAME spot
    // (LEFT_TOP corner of the panel body). `SideActive::toggle(Left,
    // id)` is the one-liner that makes opening a new menu
    // automatically close whichever was open on the same side.
    let left = SideRibbon::new(Side::Left, 2);
    left.button(
        "demo_btn_widgets",
        ctx,
        0,
        "W",
        "Widgets gallery",
        side_active.on(RibbonKind::Left) == Some(MENU_WIDGETS),
        accent_col,
        || side_active.toggle(RibbonKind::Left, MENU_WIDGETS),
    );
    left.button(
        "demo_btn_containers",
        ctx,
        1,
        "C",
        "Containers showcase",
        side_active.on(RibbonKind::Left) == Some(MENU_CONTAINERS),
        accent_col,
        || side_active.toggle(RibbonKind::Left, MENU_CONTAINERS),
    );

    // Two menus on the right rail — same deal.
    let right = SideRibbon::new(Side::Right, 2);
    right.button(
        "demo_btn_theme",
        ctx,
        0,
        "T",
        "Theme & colour",
        side_active.on(RibbonKind::Right) == Some(MENU_THEME),
        accent_col,
        || side_active.toggle(RibbonKind::Right, MENU_THEME),
    );
    right.button(
        "demo_btn_about",
        ctx,
        1,
        "?",
        "About this demo",
        side_active.on(RibbonKind::Right) == Some(MENU_ABOUT),
        accent_col,
        || side_active.toggle(RibbonKind::Right, MENU_ABOUT),
    );
}

// ─── Panels — each anchors to the top of its rail ───────────────────

fn draw_panels(
    mut contexts: EguiContexts,
    accent: Res<AccentColor>,
    side_active: Res<SideActive>,
    mut state: ResMut<DemoState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let accent_col = accent.0;

    // `floating_window` wants a `&mut bool` for its open flag (it
    // doesn't write it in this version, but still needs the slot).
    // A throwaway local is fine because the real open-state lives in
    // `SideActive`.
    let mut keep_open = true;

    if side_active.on(RibbonKind::Left) == Some(MENU_WIDGETS) {
        floating_window(
            ctx,
            "demo_panel_widgets",
            "Widgets",
            egui::Align2::LEFT_TOP,
            egui::vec2(320.0, 600.0),
            &mut keep_open,
            accent_col,
            |ui| widgets_panel(ui, &mut state, accent_col),
        );
    } else if side_active.on(RibbonKind::Left) == Some(MENU_CONTAINERS) {
        floating_window(
            ctx,
            "demo_panel_containers",
            "Containers",
            egui::Align2::LEFT_TOP,
            egui::vec2(320.0, 400.0),
            &mut keep_open,
            accent_col,
            |ui| containers_panel(ui, &mut state, accent_col),
        );
    }

    if side_active.on(RibbonKind::Right) == Some(MENU_THEME) {
        floating_window(
            ctx,
            "demo_panel_theme",
            "Theme",
            egui::Align2::RIGHT_TOP,
            egui::vec2(300.0, 240.0),
            &mut keep_open,
            accent_col,
            |ui| theme_panel(ui, &mut state, accent_col),
        );
    } else if side_active.on(RibbonKind::Right) == Some(MENU_ABOUT) {
        floating_window(
            ctx,
            "demo_panel_about",
            "About",
            egui::Align2::RIGHT_TOP,
            egui::vec2(300.0, 220.0),
            &mut keep_open,
            accent_col,
            |ui| about_panel(ui, accent_col),
        );
    }
}

// ─── Panel bodies ───────────────────────────────────────────────────

fn widgets_panel(ui: &mut egui::Ui, state: &mut DemoState, accent: egui::Color32) {
    section(ui, "demo_flags", "Flags", accent, true, |ui| {
        toggle(ui, "power", &mut state.power, accent);
        toggle(ui, "headlights", &mut state.headlights, accent);
    });

    section(ui, "demo_numbers", "Numbers", accent, true, |ui| {
        drag_value(ui, "gravity (m/s²)", &mut state.gravity, 0.05, 0.0..=30.0, 2, "");
        drag_value(ui, "speed limit (m/s)", &mut state.speed_limit, 0.1, 0.0..=100.0, 1, "");
        drag_value(ui, "engine power (kW)", &mut state.engine_power, 1.0, 0.0..=2_000.0, 0, "");
    });

    section(ui, "demo_bars", "Bars", accent, true, |ui| {
        pretty_slider(ui, "throttle", &mut state.throttle, 0.0..=1.0, 2, "", accent);
        pretty_slider(ui, "brake", &mut state.brake, 0.0..=1.0, 2, "", accent);
        pretty_progressbar_text(
            ui,
            "fuel",
            state.fuel_fraction,
            &format!("{:.0}%", state.fuel_fraction * 100.0),
            accent,
        );
    });

    section(ui, "demo_buttons", "Buttons", accent, false, |ui| {
        if wide_button(ui, "Refuel", accent).clicked() {
            state.fuel_fraction = 1.0;
        }
        card_button(
            ui,
            "★",
            "Primary action",
            "Two-line card button with glyph + subtitle",
            accent,
        );
    });
}

fn containers_panel(ui: &mut egui::Ui, state: &mut DemoState, accent: egui::Color32) {
    section(ui, "demo_transform", "Transform", accent, true, |ui| {
        subsection(
            ui,
            "demo_tr_pos",
            "Position",
            Some("drag, double-click to type"),
            accent,
            true,
            |ui| {
                axis_drag(ui, "X", egui::Color32::from_rgb(0xE0, 0x43, 0x3B),
                    &mut state.position[0], 0.05, " m", 3);
                axis_drag(ui, "Y", egui::Color32::from_rgb(0x7F, 0xB4, 0x35),
                    &mut state.position[1], 0.05, " m", 3);
                axis_drag(ui, "Z", egui::Color32::from_rgb(0x2E, 0x83, 0xE6),
                    &mut state.position[2], 0.05, " m", 3);
            },
        );

        subsection(
            ui,
            "demo_tr_rot",
            "Rotation",
            Some("Euler XYZ, degrees"),
            accent,
            true,
            |ui| {
                axis_drag(ui, "X", egui::Color32::from_rgb(0xE0, 0x43, 0x3B),
                    &mut state.rotation_deg[0], 1.0, "°", 2);
                axis_drag(ui, "Y", egui::Color32::from_rgb(0x7F, 0xB4, 0x35),
                    &mut state.rotation_deg[1], 1.0, "°", 2);
                axis_drag(ui, "Z", egui::Color32::from_rgb(0x2E, 0x83, 0xE6),
                    &mut state.rotation_deg[2], 1.0, "°", 2);
            },
        );
    });
}

fn theme_panel(ui: &mut egui::Ui, state: &mut DemoState, accent: egui::Color32) {
    section(ui, "demo_theme_colour", "Accent", accent, true, |ui| {
        color_rgb(ui, "tint", &mut state.tint);
        sub_caption(
            ui,
            "AccentColor is a resource; set it to a `Color32` to recolour every widget in one line.",
        );
    });
}

fn about_panel(ui: &mut egui::Ui, accent: egui::Color32) {
    section(ui, "demo_about_intro", "bevy_frost", accent, true, |ui| {
        sub_caption(
            ui,
            "Reusable glass-themed editor UI kit for Bevy + egui.",
        );
        readout_row(ui, "version", env!("CARGO_PKG_VERSION"));
        readout_row(ui, "bevy", "0.18");
        readout_row(ui, "bevy_egui", "0.39");
    });
}
