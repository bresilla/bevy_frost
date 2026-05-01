//! newui — Phase 2 deliverable for `PLAN_NEWUI.md`, plus the same
//! Bevy 3D scene the existing demo ships (planet + cloud shell +
//! swatch cubes + sun + chase camera). Clicking a swatch cube
//! repaints the accent across every visible Pane2 — gives us a way
//! to verify theme tinting before we add real widgets.
//!
//! Run with `make run-newui`.

use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use bevy_glacial::prelude::*;
// Everything UI-side comes from `corekit` now — ribbon types,
// resources, draw_assembly, theme runtime, pane. That way the
// ribbon buttons and the panes share a single global theme state
// (corekit's). Going through `bevy_frost::prelude::*` would hand
// us `frostcore`'s parallel state, where the buttons never see the
// PRO/GAME swap.
use corekit::container::Normal;
use corekit::pane::{PaneAnchor, Pane2, RailZone};
// Re-exports from `frostcore` (via `bevy_frost::*`) — used for the
// "scalable pod" demo slots (mix == 16 / 17) that show off a code
// editor and node-graph canvas inside a `Pod::fill()` slot.
use bevy_frost::code::{frost_code_editor, Syntax};
use bevy_frost::snarl::{
    frost_snarl, InPin, InPinId, OutPin, OutPinId, PinInfo, Snarl, SnarlPin, SnarlViewer,
};
use corekit::ribbon::{
    draw_assembly, find_item, find_ribbon, RibbonCluster, RibbonDef, RibbonDrag, RibbonEdge,
    RibbonGlyph, RibbonItem, RibbonMode, RibbonOpen, RibbonPlacement, RibbonRole,
};
use corekit::style::{AccentColor, GlassOpacity, Mode};

// ─── Ribbon / pane ids ──────────────────────────────────────────────

const RIBBON_LEFT:   &str = "newui_ribbon_left";
const RIBBON_RIGHT:  &str = "newui_ribbon_right";
const RIBBON_TOP:    &str = "newui_ribbon_top";
const RIBBON_BOTTOM: &str = "newui_ribbon_bottom";

const PANE_L_S: &str = "newui_pane_LS";
const PANE_L_M: &str = "newui_pane_LM";
const PANE_L_E: &str = "newui_pane_LE";
const PANE_R_S: &str = "newui_pane_RS";
const PANE_R_M: &str = "newui_pane_RM";
const PANE_R_E: &str = "newui_pane_RE";
const PANE_T_S: &str = "newui_pane_TS";
const PANE_T_M: &str = "newui_pane_TM";
const PANE_T_E: &str = "newui_pane_TE";
const PANE_B_S: &str = "newui_pane_BS";
const PANE_B_M: &str = "newui_pane_BM";
const PANE_B_E: &str = "newui_pane_BE";

const ACTION_THEME:  &str = "newui_action_theme";
const ACTION_MODE:   &str = "newui_action_mode";
const ACTION_PASTEL: &str = "newui_action_pastel";

const PANE_DEFS: &[(&str, &str, PaneAnchor, &str)] = &[
    (RIBBON_LEFT,   PANE_L_S, PaneAnchor::LeftRail(RailZone::Start),   "L START"),
    (RIBBON_LEFT,   PANE_L_M, PaneAnchor::LeftRail(RailZone::Middle),  "L MIDDLE"),
    (RIBBON_LEFT,   PANE_L_E, PaneAnchor::LeftRail(RailZone::End),     "L END"),
    (RIBBON_RIGHT,  PANE_R_S, PaneAnchor::RightRail(RailZone::Start),  "R START"),
    (RIBBON_RIGHT,  PANE_R_M, PaneAnchor::RightRail(RailZone::Middle), "R MIDDLE"),
    (RIBBON_RIGHT,  PANE_R_E, PaneAnchor::RightRail(RailZone::End),    "R END"),
    (RIBBON_TOP,    PANE_T_S, PaneAnchor::TopRail(RailZone::Start),    "T START"),
    (RIBBON_TOP,    PANE_T_M, PaneAnchor::TopRail(RailZone::Middle),   "T MIDDLE"),
    (RIBBON_TOP,    PANE_T_E, PaneAnchor::TopRail(RailZone::End),      "T END"),
    (RIBBON_BOTTOM, PANE_B_S, PaneAnchor::BottomRail(RailZone::Start), "B START"),
    (RIBBON_BOTTOM, PANE_B_M, PaneAnchor::BottomRail(RailZone::Middle),"B MIDDLE"),
    (RIBBON_BOTTOM, PANE_B_E, PaneAnchor::BottomRail(RailZone::End),   "B END"),
];

// All four ribbons accept buttons dragged from each other. With
// `draggable: true` + the `accepts` list filled in, `draw_assembly`
// lets the user pick up any button and drop it on any rail/cluster.
// The pane that opens for that button re-anchors automatically to
// wherever the button currently lives — drag a "TS" button to the
// LEFT rail and clicking it now opens at the LEFT rail's start.
const RIBBONS: &[RibbonDef] = &[
    RibbonDef { id: RIBBON_LEFT,   edge: RibbonEdge::Left,   role: RibbonRole::Panel,
                mode: RibbonMode::ThreeSided, draggable: true,
                accepts: &[RIBBON_RIGHT, RIBBON_TOP, RIBBON_BOTTOM] },
    RibbonDef { id: RIBBON_RIGHT,  edge: RibbonEdge::Right,  role: RibbonRole::Panel,
                mode: RibbonMode::ThreeSided, draggable: true,
                accepts: &[RIBBON_LEFT, RIBBON_TOP, RIBBON_BOTTOM] },
    RibbonDef { id: RIBBON_TOP,    edge: RibbonEdge::Top,    role: RibbonRole::Panel,
                mode: RibbonMode::ThreeSided, draggable: true,
                accepts: &[RIBBON_LEFT, RIBBON_RIGHT, RIBBON_BOTTOM] },
    RibbonDef { id: RIBBON_BOTTOM, edge: RibbonEdge::Bottom, role: RibbonRole::Panel,
                mode: RibbonMode::ThreeSided, draggable: true,
                accepts: &[RIBBON_LEFT, RIBBON_RIGHT, RIBBON_TOP] },
];

const RIBBON_ITEMS: &[RibbonItem] = &[
    RibbonItem { id: PANE_L_S, ribbon: RIBBON_LEFT,   cluster: RibbonCluster::Start,  slot: 0,
                 glyph: RibbonGlyph::Text("LS"), tooltip: "Left rail · Start", child_ribbon: None, role: None },
    RibbonItem { id: PANE_L_M, ribbon: RIBBON_LEFT,   cluster: RibbonCluster::Middle, slot: 0,
                 glyph: RibbonGlyph::Text("LM"), tooltip: "Left rail · Middle", child_ribbon: None, role: None },
    RibbonItem { id: PANE_L_E, ribbon: RIBBON_LEFT,   cluster: RibbonCluster::End,    slot: 0,
                 glyph: RibbonGlyph::Text("LE"), tooltip: "Left rail · End", child_ribbon: None, role: None },
    RibbonItem { id: PANE_R_S, ribbon: RIBBON_RIGHT,  cluster: RibbonCluster::Start,  slot: 0,
                 glyph: RibbonGlyph::Text("RS"), tooltip: "Right rail · Start", child_ribbon: None, role: None },
    RibbonItem { id: PANE_R_M, ribbon: RIBBON_RIGHT,  cluster: RibbonCluster::Middle, slot: 0,
                 glyph: RibbonGlyph::Text("RM"), tooltip: "Right rail · Middle", child_ribbon: None, role: None },
    RibbonItem { id: PANE_R_E, ribbon: RIBBON_RIGHT,  cluster: RibbonCluster::End,    slot: 0,
                 glyph: RibbonGlyph::Text("RE"), tooltip: "Right rail · End", child_ribbon: None, role: None },
    RibbonItem { id: PANE_T_S, ribbon: RIBBON_TOP,    cluster: RibbonCluster::Start,  slot: 0,
                 glyph: RibbonGlyph::Text("TS"), tooltip: "Top rail · Start", child_ribbon: None, role: None },
    RibbonItem { id: PANE_T_M, ribbon: RIBBON_TOP,    cluster: RibbonCluster::Middle, slot: 0,
                 glyph: RibbonGlyph::Text("TM"), tooltip: "Top rail · Middle", child_ribbon: None, role: None },
    RibbonItem { id: PANE_T_E, ribbon: RIBBON_TOP,    cluster: RibbonCluster::End,    slot: 0,
                 glyph: RibbonGlyph::Text("TE"), tooltip: "Top rail · End", child_ribbon: None, role: None },
    RibbonItem { id: PANE_B_S, ribbon: RIBBON_BOTTOM, cluster: RibbonCluster::Start,  slot: 0,
                 glyph: RibbonGlyph::Text("BS"), tooltip: "Bottom rail · Start", child_ribbon: None, role: None },
    RibbonItem { id: PANE_B_M, ribbon: RIBBON_BOTTOM, cluster: RibbonCluster::Middle, slot: 0,
                 glyph: RibbonGlyph::Text("BM"), tooltip: "Bottom rail · Middle", child_ribbon: None, role: None },
    RibbonItem { id: PANE_B_E, ribbon: RIBBON_BOTTOM, cluster: RibbonCluster::End,    slot: 0,
                 glyph: RibbonGlyph::Text("BE"), tooltip: "Bottom rail · End", child_ribbon: None, role: None },
    RibbonItem { id: ACTION_THEME, ribbon: RIBBON_TOP, cluster: RibbonCluster::Middle, slot: 1,
                 glyph: RibbonGlyph::Text("⊕"), tooltip: "Cycle theme (PRO ↔ GAME)",
                 child_ribbon: None, role: Some(RibbonRole::Icon) },
    RibbonItem { id: ACTION_MODE,  ribbon: RIBBON_TOP, cluster: RibbonCluster::Middle, slot: 2,
                 glyph: RibbonGlyph::Text("☼"), tooltip: "Cycle mode (Dark ↔ Light)",
                 child_ribbon: None, role: Some(RibbonRole::Icon) },
    RibbonItem { id: ACTION_PASTEL, ribbon: RIBBON_TOP, cluster: RibbonCluster::Middle, slot: 3,
                 glyph: RibbonGlyph::Text("◐"), tooltip: "Toggle pastel accent",
                 child_ribbon: None, role: Some(RibbonRole::Icon) },
];

// ─── Theme + scene state ───────────────────────────────────────────

#[derive(Resource, Clone, Copy, Debug)]
struct ThemeFamily(u8);
impl Default for ThemeFamily { fn default() -> Self { Self(0) } }

#[derive(Resource, Clone, Copy, Debug)]
struct ThemeModeRes(u8);
impl Default for ThemeModeRes { fn default() -> Self { Self(0) } }

/// Toggles `Theme::pastel_accent`. `true` (default) → accents flow
/// through `adapt_accent_to_mode` (whiter accents pulled to less
/// luminance, darker ones lifted) — the readable-on-any-surface
/// pastel pull. `false` → raw user-picked accent, neon-saturated.
#[derive(Resource, Clone, Copy, Debug)]
struct PastelToggle(bool);
impl Default for PastelToggle { fn default() -> Self { Self(true) } }

/// Marker + per-cube data for the swatch cubes — clicking one
/// repaints `AccentColor` and lifts the cube in the scene.
#[derive(Component)]
struct ColorCube {
    egui_col: egui::Color32,
    base_color: Color,
}

#[derive(Resource, Default)]
struct SelectedSwatch(Option<Entity>);

const PLANET_RADIUS: f32 = 6_371_000.0;
const CLOUD_ALTITUDE_M: f32 = 4_000.0;

// ─── App ───────────────────────────────────────────────────────────

fn main() {
    // `bevy_glacial::WindowSettingsPlugin` persists the primary
    // window's size + position to
    // `${XDG_CONFIG_HOME:-~/.config}/newui/window.txt`. Load the
    // saved geometry up-front so the first paint uses it; the
    // plugin (registered below) writes back on resize / move.
    let geometry = WindowGeometry::load("newui");
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(geometry.to_window("newui — Phase 2")),
                    ..default()
                })
                // Surface egui's debug-level chatter (request_discard
                // reasons, id-collision diagnostics, etc.) on stdout
                // so we can see WHY a particular frame triggered a
                // PERF overlay or an id-clash 🔥 marker. Bevy's
                // default filter is INFO, which hides everything
                // egui logs below WARN. Override with `RUST_LOG=...`
                // at runtime if you want even more.
                .set(bevy::log::LogPlugin {
                    level: bevy::log::Level::DEBUG,
                    filter: "info,wgpu=error,bevy_render=error,bevy_winit=error,naga=warn,\
                             egui=debug,egui_flex=debug,corekit=debug,bevy_frost=debug"
                        .into(),
                    ..default()
                }),
        )
        .add_plugins(bevy_egui::EguiPlugin::default())
        // No `FrostPlugin` — that wires `frostcore`'s theme runtime
        // and ribbon resources, which would shadow the ones we
        // initialise from `corekit` below. We register corekit's
        // resources manually and run `apply_theme` inside
        // `ui_system`, which is enough to drive PRO/GAME for both
        // the ribbon buttons (corekit's draw_assembly + paint) AND
        // the panes (Pane2).
        //
        // We DO add `EguiInputAbsorbPlugin` — drains
        // `Messages<MouseWheel>` whenever the cursor sits over a
        // frost pane, so scrolling inside a pane doesn't also
        // drive `bevy_glacial`'s chase-camera zoom.
        .add_plugins(bevy_frost::EguiInputAbsorbPlugin)
        .add_plugins(GlacialPlugins)
        .add_plugins(WindowSettingsPlugin::new("newui"))
        .insert_resource(ClearColor(Color::srgb(0.06, 0.08, 0.12)))
        .insert_resource(GroundGrid {
            visible: true,
            color: Color::srgba(0.30, 0.38, 0.50, 0.42),
        })
        // corekit ribbon resources (the bevy feature derives Resource
        // on each via cfg_attr).
        .init_resource::<AccentColor>()
        .init_resource::<GlassOpacity>()
        .init_resource::<RibbonOpen>()
        .init_resource::<RibbonPlacement>()
        .init_resource::<RibbonDrag>()
        .init_resource::<ThemeFamily>()
        .init_resource::<ThemeModeRes>()
        .init_resource::<PastelToggle>()
        .init_resource::<SelectedSwatch>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (pick_cube, update_swatch_selection))
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

// ─── Scene setup (mirrors the demo's planet + cube grid + sun) ─────

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Planet sphere — warm tan ground, tangent at world y=0.
    let planet_mesh = meshes.add(Sphere::new(PLANET_RADIUS).mesh().uv(1024, 512));
    let planet_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.48, 0.33),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Name::new("Planet"),
        Transform::from_xyz(0.0, -PLANET_RADIUS, 0.0),
        Mesh3d(planet_mesh),
        MeshMaterial3d(planet_mat),
        NotShadowCaster,
        NotShadowReceiver,
    ));

    // Cloud shell — translucent white sphere.
    let shell_radius = PLANET_RADIUS + CLOUD_ALTITUDE_M;
    let cloud_mesh = meshes.add(Sphere::new(shell_radius).mesh().uv(64, 32));
    let cloud_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.35),
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        unlit: false,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Name::new("CloudShell"),
        Transform::from_xyz(0.0, -PLANET_RADIUS, 0.0),
        Mesh3d(cloud_mesh),
        MeshMaterial3d(cloud_mat),
        NotShadowCaster,
    ));

    // Swatch cubes — 3 × 2 grid, click to repaint the accent.
    let cube_mesh = meshes.add(Cuboid::from_length(1.0));
    let swatch: [(f32, f32, f32); 6] = [
        (0.90, 0.30, 0.30),
        (0.95, 0.65, 0.20),
        (0.95, 0.90, 0.30),
        (0.35, 0.85, 0.45),
        (0.30, 0.60, 0.95),
        (0.75, 0.45, 0.95),
    ];
    const GRID_COLS: usize = 3;
    const GRID_SPACING: f32 = 2.0;
    for (i, &(r, g, b)) in swatch.iter().enumerate() {
        let col = (i % GRID_COLS) as f32;
        let row = (i / GRID_COLS) as f32;
        let x = (col - (GRID_COLS as f32 - 1.0) * 0.5) * GRID_SPACING;
        let z = (row - 0.5) * GRID_SPACING;
        let bevy_col = Color::srgb(r, g, b);
        let egui_col = egui::Color32::from_rgb(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        );
        commands.spawn((
            Name::new(format!("Swatch[{i}]")),
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: bevy_col,
                perceptual_roughness: 0.6,
                ..default()
            })),
            Transform::from_xyz(x, 0.5, z),
            ColorCube { egui_col, base_color: bevy_col },
        ));
    }

    // Sun.
    let sun_shadow = CascadeShadowConfigBuilder {
        num_cascades: 1,
        minimum_distance: 0.1,
        maximum_distance: 100.0,
        first_cascade_far_bound: 100.0,
        overlap_proportion: 0.0,
    }
    .build();
    commands.spawn((
        Name::new("Sun"),
        Transform::from_xyz(5.0, 50.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            illuminance: 10_000.0,
            shadows_enabled: true,
            ..default()
        },
        sun_shadow,
    ));

    // Camera + atmospheric fog + ambient fill.
    let projection = Projection::Perspective(PerspectiveProjection {
        near: 0.1,
        far: PLANET_RADIUS * 2.5,
        ..default()
    });
    let fog = DistanceFog {
        color: Color::srgb(0.10, 0.13, 0.20),
        falloff: FogFalloff::Atmospheric {
            extinction: Vec3::new(0.00008, 0.00012, 0.00020),
            inscattering: Vec3::new(0.00010, 0.00015, 0.00025),
        },
        ..default()
    };
    let chase = ChaseCamera::default();
    let mut cam_tr = Transform::default();
    apply_rig(&chase, &mut cam_tr);
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        cam_tr,
        projection,
        fog,
        AmbientLight {
            color: Color::WHITE,
            brightness: 120.0,
            ..default()
        },
        chase,
    ));
}

// ─── Cube picking (left-click → recolour accent) ───────────────────

fn pick_cube(
    mouse: Res<ButtonInput<MouseButton>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    bevy_cameras: Query<(&Camera, &GlobalTransform)>,
    cubes: Query<(Entity, &Transform, &ColorCube)>,
    mut contexts: EguiContexts,
    mut accent: ResMut<AccentColor>,
    mut selected: ResMut<SelectedSwatch>,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }
    if mouse.pressed(MouseButton::Right) { return; }
    if contexts.ctx_mut().map(|c| c.wants_pointer_input()).unwrap_or(false) {
        return;
    }

    let Some(cursor) = primary_window.single().ok().and_then(|w| w.cursor_position()) else {
        return;
    };
    let Ok((camera, cam_tr)) = bevy_cameras.single() else { return };
    let Ok(ray) = camera.viewport_to_world(cam_tr, cursor) else { return };
    let origin = ray.origin;
    let direction = *ray.direction;

    let mut best: Option<(f32, Entity, egui::Color32)> = None;
    for (entity, tr, cube) in &cubes {
        let min = tr.translation - Vec3::splat(0.5);
        let max = tr.translation + Vec3::splat(0.5);
        if let Some(t) = ray_aabb_hit(origin, direction, min, max) {
            match best {
                Some((bt, _, _)) if bt <= t => {}
                _ => best = Some((t, entity, cube.egui_col)),
            }
        }
    }
    if let Some((_, entity, color)) = best {
        accent.0 = color;
        selected.0 = Some(entity);
    }
}

fn update_swatch_selection(
    time: Res<Time>,
    selected: Res<SelectedSwatch>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cubes: Query<(
        Entity,
        &ColorCube,
        &MeshMaterial3d<StandardMaterial>,
        &mut Transform,
    )>,
) {
    const REST_Y: f32 = 0.5;
    const LIFT_Y: f32 = 0.9;
    const EASE: f32 = 8.0;
    let k = (EASE * time.delta_secs()).min(0.9);
    for (entity, cube, mat_handle, mut tr) in &mut cubes {
        let is_sel = selected.0 == Some(entity);
        let target_y = if is_sel { LIFT_Y } else { REST_Y };
        tr.translation.y += (target_y - tr.translation.y) * k;
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            let base = cube.base_color.to_linear();
            let gain = if is_sel { 1.8 } else { 0.0 };
            mat.emissive = LinearRgba::new(
                base.red * gain,
                base.green * gain,
                base.blue * gain,
                1.0,
            );
        }
    }
}

fn ray_aabb_hit(origin: Vec3, direction: Vec3, min: Vec3, max: Vec3) -> Option<f32> {
    let mut tmin = 0.0_f32;
    let mut tmax = f32::INFINITY;
    for i in 0..3 {
        let (o, d, lo, hi) = match i {
            0 => (origin.x, direction.x, min.x, max.x),
            1 => (origin.y, direction.y, min.y, max.y),
            _ => (origin.z, direction.z, min.z, max.z),
        };
        if d.abs() < 1e-6 {
            if o < lo || o > hi { return None; }
        } else {
            let mut t1 = (lo - o) / d;
            let mut t2 = (hi - o) / d;
            if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax { return None; }
        }
    }
    Some(tmin.max(0.0))
}

// ─── UI system ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn ui_system(
    mut contexts: EguiContexts,
    accent: Res<AccentColor>,
    glass: Res<GlassOpacity>,
    mut open: ResMut<RibbonOpen>,
    mut placement: ResMut<RibbonPlacement>,
    mut drag: ResMut<RibbonDrag>,
    mut family: ResMut<ThemeFamily>,
    mut mode: ResMut<ThemeModeRes>,
    mut pastel: ResMut<PastelToggle>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    // Drive corekit's theme runtime each frame. `set_theme` swaps
    // the global; `apply_theme` registers the bundled Iosevka faces
    // and pushes the theme-derived `egui::Visuals`. Both ribbon
    // buttons (drawn by corekit's `draw_assembly`) and Pane2 read
    // from this same global, so PRO ↔ GAME cycles every visible
    // surface together.
    let mut active_theme = match (family.0, mode.0) {
        (0, 0) => corekit::style::theme_pro(Mode::Dark),
        (0, 1) => corekit::style::theme_pro(Mode::Light),
        (1, 0) => corekit::style::theme_game(Mode::Dark),
        (1, 1) => corekit::style::theme_game(Mode::Light),
        _      => corekit::style::theme_pro(Mode::Dark),
    };
    active_theme.pastel_accent = pastel.0;
    corekit::style::set_theme(active_theme);
    corekit::style::apply_theme(ctx, *accent, *glass);

    // Mirror the same Theme/Mode on `frostcore::style` — the
    // wrapper widgets we still pull from frostcore (`frost_snarl`,
    // `frost_code_editor`) read frostcore's parallel theme global
    // for their fills, borders, and font sizes. Without this, the
    // graph and code editor would always render with the default
    // PRO Dark style regardless of the live theme cycle the rest
    // of the UI follows. AccentColor / GlassOpacity are simple
    // newtypes on each side, so the values transfer 1:1.
    let frostcore_theme = match (family.0, mode.0) {
        (0, 0) => frostcore::style::theme_pro(frostcore::style::Mode::Dark),
        (0, 1) => frostcore::style::theme_pro(frostcore::style::Mode::Light),
        (1, 0) => frostcore::style::theme_game(frostcore::style::Mode::Dark),
        (1, 1) => frostcore::style::theme_game(frostcore::style::Mode::Light),
        _      => frostcore::style::theme_pro(frostcore::style::Mode::Dark),
    };
    frostcore::style::set_theme(frostcore_theme);
    frostcore::style::apply_theme(
        ctx,
        frostcore::style::AccentColor(accent.0),
        frostcore::style::GlassOpacity(glass.0),
    );

    // Surface egui's request_discard reasons + multipass-in-row
    // count to the terminal. egui only paints these as a red
    // PERF-WARNING overlay (see `Context::end_pass`), it doesn't
    // log them — so without this we couldn't see what's triggering
    // the overlay during animation.
    let (discard_reasons, multipass) = ctx.output(|o| {
        (
            o.request_discard_reasons.clone(),
            o.num_completed_passes,
        )
    });
    if !discard_reasons.is_empty() {
        eprintln!(
            "[egui] frame had {} request_discard call(s) ({} pass(es))",
            discard_reasons.len(),
            multipass,
        );
        for r in &discard_reasons {
            eprintln!("  └─ {r}");
        }
    }

    // F10 → toggle the FROST custom inspector. Tags only the
    // surfaces we care about (panes, containers, pods) with labels
    // we wrote — see `corekit::debug`. Hovering any tagged surface
    // outlines its rect and stamps a label chip at the corner. No
    // `container_pointer` / horizontal / clip noise. Works in both
    // `debug` and `--release` builds since it's our code, not
    // egui's `cfg(debug_assertions)`-gated overlay.
    {
        let inspect = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F10));
        if inspect {
            let on = !corekit::debug::is_enabled(ctx);
            corekit::debug::set_enabled(ctx, on);
        }
    }
    // F12 → egui's stock "show interactive widget bounds" overlay.
    // Coloured outline + hit-rect on every widget egui knows about
    // — useful when chasing a hit-target / layout bug, noisy for
    // anything else. `Style.debug` is `cfg(debug_assertions)`-
    // gated by egui, so this is a no-op in `--release`.
    #[cfg(debug_assertions)]
    {
        let pressed = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F12));
        if pressed {
            ctx.style_mut(|s| {
                s.debug.show_interactive_widgets = !s.debug.show_interactive_widgets;
                s.debug.show_widget_hits = s.debug.show_interactive_widgets;
            });
        }
    }
    // Paint the inspector overlay AFTER everything else has
    // rendered this frame. No-op when the inspector is off.
    corekit::debug::paint(ctx);

    // Pastelized accent flows through chrome (ribbon paint, panel
    // fills, borders, glass tint) so the `Theme::pastel_accent`
    // toggle visibly retints surfaces. Title text is exempt — the
    // text-paint helpers internally read `style::raw_accent()` so
    // titles always show the user's literal pick.
    let accent_col = corekit::style::active_accent();
    let clicks = draw_assembly(
        ctx, accent_col, RIBBONS, RIBBON_ITEMS,
        &mut open, &mut placement, &mut drag,
        |_| false,
    );

    for click in clicks {
        if click.item == ACTION_THEME { family.0 = (family.0 + 1) % 2; }
        else if click.item == ACTION_MODE { mode.0 = (mode.0 + 1) % 2; }
        else if click.item == ACTION_PASTEL { pastel.0 = !pastel.0; }
    }

    let is_open = |id: &'static str| -> bool {
        let Some(item) = find_item(RIBBON_ITEMS, id) else { return false };
        let (rid, _, _) = placement.resolve(item);
        open.is_open(rid, id)
    };

    // Resolve the LIVE pane anchor — `placement.resolve` returns
    // wherever the button currently lives (after any drag), so a
    // button moved from TopRail::Start to LeftRail::End now opens
    // at the new corner.
    let live_anchor = |id: &'static str| -> Option<PaneAnchor> {
        let item = find_item(RIBBON_ITEMS, id)?;
        let (rid, cluster, _) = placement.resolve(item);
        let def = find_ribbon(RIBBONS, rid)?;
        let zone = match cluster {
            RibbonCluster::Start  => RailZone::Start,
            RibbonCluster::Middle => RailZone::Middle,
            RibbonCluster::End    => RailZone::End,
        };
        Some(match def.edge {
            RibbonEdge::Left   => PaneAnchor::LeftRail(zone),
            RibbonEdge::Right  => PaneAnchor::RightRail(zone),
            RibbonEdge::Top    => PaneAnchor::TopRail(zone),
            RibbonEdge::Bottom => PaneAnchor::BottomRail(zone),
        })
    };

    for &(_, button_id, default_anchor, label) in PANE_DEFS {
        if is_open(button_id) {
            // Fall back to the declared default if we can't resolve
            // (shouldn't happen — every pane button is in RIBBON_ITEMS).
            let anchor = live_anchor(button_id).unwrap_or(default_anchor);
            Pane2::new(button_id, label, anchor, accent_col)
                // Pane only resizes on the SPAN axis — flow-axis
                // size auto-derives from the sum of container body
                // flows + per-container chrome. Each container's
                // own size is dragged via the inter-container
                // separator below.
                .resize(corekit::pane::PaneResize::SPAN)
                .show(ctx, |body_ui| {
                    // Three independent containers, each with its own
                    // toggle state (id derived from the pane's button
                    // id + an index).
                    const CONTAINERS_PER_PANE: usize = 3;
                    // Build the default id list in declaration order,
                    // then ask the pane for the user's persisted
                    // drag-reorder order — which preserves any drag
                    // commits across runs and falls back to defaults
                    // for new ids.
                    let pane_egui_id = egui::Id::new(button_id);
                    let defaults: Vec<egui::Id> = (0..CONTAINERS_PER_PANE)
                        .map(|i| egui::Id::new((button_id, "container", i)))
                        .collect();
                    let order = corekit::pane::section_order_for(
                        body_ui.ctx(),
                        pane_egui_id,
                        &defaults,
                    );
                    // Container-resize-handle orientation matches
                    // the container stack direction: vertical-strip
                    // panes (Left/Right rail middle, corner-zone
                    // Top/Bottom) stack containers along X, so the
                    // dot handle runs top↕bottom (vertical);
                    // horizontal-strip panes stack along Y, so the
                    // handle runs left↔right (horizontal).
                    let containers_stack_horizontally =
                        !anchor.title_side().is_horizontal_strip();
                    let dots_orient = if containers_stack_horizontally {
                        corekit::container::SeparatorOrient::Vertical
                    } else {
                        corekit::container::SeparatorOrient::Horizontal
                    };
                    for cid in order.into_iter() {
                        // Recover the original index for the title /
                        // text-input key so renaming after a reorder
                        // still maps each id back to "container N".
                        let i = defaults.iter().position(|d| *d == cid).unwrap_or(0);
                        let title = format!("{} {}", label, i + 1);
                        // Deterministic per-container "randomness":
                        // hash the container id into a u64, then
                        // peel a few digits off the bottom for the
                        // pod count (1..=4) and per-pod search count
                        // (1..=3). Stable across frames because the
                        // cid is stable.
                        let seed: u64 = cid.value();
                        // ONE widget per pod. Multi-row widgets (tree,
                        // select_list, hybrid_select_list) are still
                        // ONE widget — they happen to paint several
                        // rows internally. `pod_count` is between 2
                        // and 5 so each container shows a spread of
                        // kinds without any pod hosting more than its
                        // single widget.
                        let pod_count = ((seed % 4) as usize) + 2;
                        let pods: Vec<_> = (0..pod_count)
                            .map(|p| {
                                let pod_id = cid.with(("newui_pod", p));
                                // Spread `p` across the full 64-bit
                                // pod_seed via golden-ratio multiply
                                // — XOR with low bits alone would be
                                // discarded by the `>> 4` mask below
                                // and every pod would land on the
                                // same `mix`.
                                let pod_seed = seed
                                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                                    ^ (p as u64).wrapping_mul(0xD737_3348_5DDB_E5C5);
                                // 5-bit mix (32 values) — adds slots 16
                                // (code editor) and 17 (node graph)
                                // beyond the original 16 widget kinds.
                                let mix = (pod_seed >> 4) & 0b11111;
                                // Only widgets that benefit from a
                                // hide/reveal viewport (tree, select
                                // lists) get the LineDots resizable
                                // separator. Everything else gets the
                                // plain Line — the rule is "if there
                                // is a dotted handle, dragging it
                                // actually does something visible".
                                // - Resizable: user-draggable handle
                                //   reveals/hides rows. Visible dotted
                                //   separator (LineDots).
                                // - Fill: auto-expands to consume
                                //   leftover container space; user
                                //   can't drag. Plain Line separator
                                //   (no drag handle, but still a
                                //   visible divider so neighbour pods
                                //   read distinct).
                                let is_resizable = matches!(mix, 9 | 10);
                                let is_fill = matches!(mix, 14 | 16 | 17);
                                let is_last = p + 1 == pod_count;
                                let separator_style = if is_resizable {
                                    corekit::container::SeparatorStyle::LineDots
                                } else if is_last {
                                    corekit::container::SeparatorStyle::None
                                } else {
                                    corekit::container::SeparatorStyle::Line
                                };
                                let mut pod = corekit::pod::Pod::new(pod_id)
                                    .with_separator(separator_style);
                                if is_resizable {
                                    pod = pod.resizable();
                                }
                                if is_fill {
                                    pod = pod.fill();
                                }
                                {
                                    // Single-widget per pod — keep
                                    // the existing one-letter `s`
                                    // alias the inner branches use
                                    // for trailing-suffix formatting,
                                    // pinned to 0 since there's no
                                    // longer an inner loop.
                                    let s: usize = 0;
                                    let _ = s;
                                    pod = match mix {
                                        0 => pod.with_search(
                                            format!(
                                                "pod {} · search {}…",
                                                p + 1,
                                                s + 1
                                            ),
                                            accent_col,
                                        ),
                                        1 => pod.with_button(
                                            format!("pod {} · btn {}", p + 1, s + 1),
                                            accent_col,
                                        ),
                                        2 => {
                                            // 2U "card" button — primary
                                            // label + dim subtitle + an
                                            // animated hover fill so the
                                            // user sees that the 2-layer
                                            // shape supports every
                                            // animation the 1U one does.
                                            use corekit::widget::FillStyle::*;
                                            let styles = [
                                                SlideLeft,
                                                CircleGrow,
                                                BandsMeet,
                                                ParallelogramMeet,
                                                Bowtie,
                                                CornerSquares,
                                                DiagonalTriangles,
                                                Equalizer,
                                                HorizontalSlideDelayed,
                                                VerticalSlideDelayed,
                                                CrissCross,
                                            ];
                                            let pick = ((pod_seed >> (s * 4 + 16))
                                                as usize)
                                                % styles.len();
                                            pod.with_button_styled(
                                                format!("Preset {}", s + 1),
                                                accent_col,
                                                Some("stacked subtitle"),
                                                Option::<&str>::None,
                                                Some(styles[pick]),
                                            )
                                        }
                                        3 => {
                                            use corekit::widget::FillStyle::*;
                                            let styles = [
                                                SlideLeft,
                                                Parallelogram,
                                                ParallelogramMeet,
                                                Bowtie,
                                                BandsMeet,
                                                CornerSquares,
                                                DiagonalTriangles,
                                                CircleGrow,
                                                Equalizer,
                                                HorizontalSlide,
                                                HorizontalSlideDelayed,
                                                VerticalSlideDelayed,
                                                CrissCross,
                                            ];
                                            let pick = ((pod_seed >> (s * 4 + 16))
                                                as usize)
                                                % styles.len();
                                            pod.with_button_animated(
                                                format!("anim {}", s + 1),
                                                accent_col,
                                                styles[pick],
                                            )
                                        }
                                        4 => pod.with_toggle_initial(
                                            format!("toggle {}", s + 1),
                                            accent_col,
                                            (pod_seed >> 8) & 1 == 1,
                                        ),
                                        5 => {
                                            let frac = (((pod_seed >> (s * 5))
                                                & 0xFF)
                                                as f32)
                                                / 255.0;
                                            pod.with_progress(
                                                format!("progress {}", s + 1),
                                                frac,
                                                format!(
                                                    "{:.0}%",
                                                    frac * 100.0
                                                ),
                                                accent_col,
                                            )
                                        }
                                        6 => {
                                            let v = ((pod_seed >> (s * 5)) & 0xFF)
                                                as f64
                                                / 255.0;
                                            pod.with_slider(
                                                format!("slider {}", s + 1),
                                                v,
                                                0.0..=1.0,
                                                2,
                                                "",
                                                accent_col,
                                            )
                                        }
                                        7 => pod.with_drag_value(
                                            format!("value {}", s + 1),
                                            ((pod_seed >> (s * 5)) & 0xFF) as f64,
                                            0.5,
                                            0.0..=255.0,
                                            1,
                                            "",
                                        ),
                                        8 => pod.with_dropdown(
                                            ["Alpha", "Beta", "Gamma", "Delta"],
                                            ((pod_seed >> (s * 4)) as usize) % 4,
                                            accent_col,
                                        ),
                                        9 => {
                                            // Multi-row select list as
                                            // ONE widget. 8 items;
                                            // selection persists in
                                            // ctx data via the pod's
                                            // own slot key.
                                            let items: Vec<String> = (1..=8u8)
                                                .map(|i| format!("Item {i}"))
                                                .collect();
                                            let trailing: Vec<String> = (1..=8u8)
                                                .map(|i| format!("#{i}"))
                                                .collect();
                                            pod.with_select_list(
                                                items,
                                                Some(trailing),
                                                accent_col,
                                            )
                                        }
                                        10 => {
                                            // Multi-row hybrid select
                                            // list as ONE widget — body
                                            // click + radio pin per
                                            // row, single-pin across
                                            // the list.
                                            let items: Vec<String> = (1..=8u8)
                                                .map(|i| format!("Layer {i}"))
                                                .collect();
                                            let trailing: Vec<String> = (1..=8u8)
                                                .map(|i| format!("L{i}"))
                                                .collect();
                                            pod.with_hybrid_select_list(
                                                items,
                                                Some(trailing),
                                                accent_col,
                                            )
                                        }
                                        11 => {
                                            let r = ((pod_seed >> 0) & 0xFF)
                                                as f32
                                                / 255.0;
                                            let g = ((pod_seed >> 8) & 0xFF)
                                                as f32
                                                / 255.0;
                                            let b = ((pod_seed >> 16) & 0xFF)
                                                as f32
                                                / 255.0;
                                            pod.with_color_rgb(
                                                format!("rgb {}", s + 1),
                                                [r, g, b],
                                                accent_col,
                                            )
                                        }
                                        12 => {
                                            let r = ((pod_seed >> 0) & 0xFF)
                                                as f32
                                                / 255.0;
                                            let g = ((pod_seed >> 8) & 0xFF)
                                                as f32
                                                / 255.0;
                                            let b = ((pod_seed >> 16) & 0xFF)
                                                as f32
                                                / 255.0;
                                            pod.with_color_rgba(
                                                format!("rgba {}", s + 1),
                                                [r, g, b, 0.7],
                                                accent_col,
                                            )
                                        }
                                        13 => {
                                            // Pick a different Fluent
                                            // icon per pod_seed so the
                                            // demo cycles through real
                                            // glyphs instead of always
                                            // showing the same one.
                                            const ICONS: &[&str] = &[
                                                "settings", "folder", "code",
                                                "search", "person", "image",
                                                "flowchart", "list",
                                            ];
                                            let pick = ((pod_seed >> 4) as usize)
                                                % ICONS.len();
                                            pod.with_card_button(
                                                ICONS[pick],
                                                format!("Card {}", s + 1),
                                                "glyph + subtitle",
                                                accent_col,
                                            )
                                        }
                                        14 => {
                                            // Inline scene-graph tree
                                            // demo. Stable id derived
                                            // from the pod_seed so each
                                            // tree gets its own
                                            // expanded / selected /
                                            // visibility state.
                                            let seed = pod_seed;
                                            let tree_root =
                                                egui::Id::new(("frost_demo_tree", seed));
                                            // 7 rows × 1U each ≈ 7
                                            // unit-equivalents — passed
                                            // via with_custom_units so
                                            // the inter-pod resize
                                            // share is proportional.
                                            pod.with_custom_units(7, move |ui| {
                                                demo_tree(ui, tree_root, accent_col);
                                            })
                                        }
                                        15 => {
                                            // Read-only info row —
                                            // label-left, monospace
                                            // value-right. Mirrors the
                                            // old `readout_row` from
                                            // frostcore (used in the
                                            // demo to show the tree's
                                            // currently-selected
                                            // path, etc.).
                                            const LABELS: &[&str] = &[
                                                "selected", "active layer",
                                                "current tool", "frame",
                                                "fps", "memory",
                                                "draw calls", "triangles",
                                            ];
                                            const VALUES: &[&str] = &[
                                                "/World/Robot/base",
                                                "World layer",
                                                "Move (G)",
                                                "00:01:42 · 2517",
                                                "60.0",
                                                "412 MB",
                                                "1 248",
                                                "8.4M",
                                            ];
                                            let pick = ((pod_seed >> 8) as usize)
                                                % LABELS.len();
                                            pod.with_readout(LABELS[pick], VALUES[pick])
                                        }
                                        16 => {
                                            // Code editor in a fill
                                            // pod — buffer state
                                            // persists in egui ctx
                                            // data per pod_seed.
                                            let code_id = egui::Id::new((
                                                "frost_demo_code", pod_seed,
                                            ));
                                            pod.with_custom_units(10, move |ui| {
                                                let mut text: String = ui
                                                    .ctx()
                                                    .data(|d| {
                                                        d.get_temp::<String>(code_id)
                                                    })
                                                    .unwrap_or_else(|| {
                                                        DEFAULT_CODE.to_string()
                                                    });
                                                let avail =
                                                    ui.available_size_before_wrap();
                                                frost_code_editor(
                                                    ui,
                                                    code_id,
                                                    &mut text,
                                                    Syntax::rust(),
                                                    accent_col,
                                                    avail,
                                                );
                                                ui.ctx().data_mut(|d| {
                                                    d.insert_temp(code_id, text)
                                                });
                                            })
                                        }
                                        17 => {
                                            // Node graph in a fill
                                            // pod — Snarl<GraphNode>
                                            // round-trips through ctx
                                            // data per pod_seed each
                                            // frame.
                                            let graph_id = egui::Id::new((
                                                "frost_demo_graph", pod_seed,
                                            ));
                                            pod.with_custom_units(10, move |ui| {
                                                let mut graph: Snarl<GraphNode> = ui
                                                    .ctx()
                                                    .data(|d| {
                                                        d.get_temp::<
                                                            Snarl<GraphNode>,
                                                        >(graph_id)
                                                    })
                                                    .unwrap_or_else(default_graph);
                                                let mut viewer = GraphViewer;
                                                let avail =
                                                    ui.available_size_before_wrap();
                                                frost_snarl(
                                                    ui,
                                                    graph_id,
                                                    &mut graph,
                                                    &mut viewer,
                                                    accent_col,
                                                    avail,
                                                );
                                                ui.ctx().data_mut(|d| {
                                                    d.insert_temp(graph_id, graph)
                                                });
                                            })
                                        }
                                        _ => pod.with_button(
                                            format!("btn {}", s + 1),
                                            accent_col,
                                        ),
                                    };
                                }
                                pod
                            })
                            .collect();
                        Normal::new(title, anchor, accent_col, cid)
                            .icon("settings")
                            .show(body_ui, pods);
                        // Container resize handle — three dots,
                        // painted AFTER every container including
                        // the last. Drag delta updates THIS
                        // container's persisted flow size; the
                        // pane auto-grows to fit.
                        //
                        // For bottom / right-anchored panes the
                        // body extends in the negative axis
                        // direction (bottom_up / right_to_left), so
                        // the handle painted "after" the container
                        // ends up on the side AWAY from the title
                        // (above for bottom, left for right). To
                        // grow the container, the user drags the
                        // handle FURTHER from the title — which is
                        // a NEGATIVE axis delta. Negate so the
                        // grow direction matches the cursor in
                        // both anchorings.
                        let title_at_end =
                            anchor.title_side().is_at_end();
                        // Skip painting the dot handle while THIS
                        // container is being dragged (reorder) —
                        // the floating drag preview already paints
                        // a copy of the container with its handle,
                        // so painting the original's handle here
                        // produces a duplicate "double dots" look
                        // until the user releases.
                        let dragging_self = corekit::pane::active_drag(body_ui.ctx())
                            .and_then(|(_, s)| s.item)
                            .map(|item| item == cid)
                            .unwrap_or(false);
                        if dragging_self {
                            continue;
                        }
                        let resp = corekit::pane::paint_container_dots(
                            body_ui,
                            dots_orient,
                            cid,
                            accent_col,
                        );
                        // Folded containers ignore the resize drag —
                        // the body slot they're sized from is hidden,
                        // so dragging the dots while folded would
                        // silently grow / shrink an invisible region
                        // and only become visible to the user when
                        // they unfold.
                        let body_open: bool = body_ui.ctx().data_mut(|d| {
                            d.get_persisted::<bool>(cid.with("body_open"))
                                .unwrap_or(true)
                        });
                        if resp.dragged() && body_open {
                            // `containers_stack_horizontally` is the
                            // inverse of `is_horizontal_strip` for
                            // the parent pane (containers stack
                            // horizontally precisely in vertical-
                            // strip panes).
                            let pane_horizontal_strip =
                                anchor.title_side().is_horizontal_strip();
                            let cur = corekit::container::container_flow(
                                body_ui.ctx(),
                                cid,
                                pane_horizontal_strip,
                            );
                            let raw = if containers_stack_horizontally {
                                resp.drag_delta().x
                            } else {
                                resp.drag_delta().y
                            };
                            let delta = if title_at_end { -raw } else { raw };
                            corekit::container::set_container_flow(
                                body_ui.ctx(),
                                cid,
                                cur + delta,
                                pane_horizontal_strip,
                            );
                        }
                    }
                });
        }
    }
}

// ─── Demo tree ─────────────────────────────────────────────────────
//
// Hand-authored mini scene-graph used to demo the `tree_row` widget
// inside a Pod's `with_custom` slot. State (expanded flags, selection,
// per-row visibility / lock toggles) lives in egui ctx data keyed off
// each node's path, so the tree survives reorders and theme switches
// without the example having to thread its own `App`-level resource.

// Each row: `(path, name, icon, children, material_color)`.
// `material_color` is painted as a `TreeIconKind::Color` swatch in
// the row's right-gutter — matches the legacy demo's "where is the
// red thing" affordance.
type DemoTreeRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
    egui::Color32,
);

const DEMO_TREE: &[DemoTreeRow] = &[
    (
        "/World", "World", "folder",
        &["/World/Robot", "/World/Lights"],
        egui::Color32::from_rgb(0x55, 0x6E, 0x9C),
    ),
    (
        "/World/Robot", "Robot", "person",
        &["/World/Robot/base", "/World/Robot/arm"],
        egui::Color32::from_rgb(0xE0, 0x6C, 0x4F),
    ),
    (
        "/World/Robot/base", "base", "code",
        &[],
        egui::Color32::from_rgb(0x4D, 0xA8, 0xDA),
    ),
    (
        "/World/Robot/arm", "arm", "code",
        &["/World/Robot/arm/grip"],
        egui::Color32::from_rgb(0xE6, 0xB7, 0x3D),
    ),
    (
        "/World/Robot/arm/grip", "grip", "code",
        &[],
        egui::Color32::from_rgb(0x9C, 0x55, 0xC0),
    ),
    (
        "/World/Lights", "Lights", "image",
        &["/World/Lights/sun"],
        egui::Color32::from_rgb(0xF5, 0xC2, 0x42),
    ),
    (
        "/World/Lights/sun", "sun", "image",
        &[],
        egui::Color32::from_rgb(0xFF, 0xE5, 0x6B),
    ),
];

fn demo_tree_node(path: &str) -> Option<&'static DemoTreeRow> {
    DEMO_TREE.iter().find(|(p, _, _, _, _)| *p == path)
}

fn demo_tree(ui: &mut egui::Ui, root_id: egui::Id, accent: egui::Color32) {
    let sel_key = root_id.with("frost_demo_tree_selected");
    let mut selected: String = ui
        .ctx()
        .data(|d| d.get_temp::<String>(sel_key))
        .unwrap_or_default();
    let initial_selected = selected.clone();

    let mut frame_clicked: Option<String> = None;
    walk_demo_tree(ui, root_id, "/World", 0, &selected, accent, &mut frame_clicked);

    if let Some(p) = frame_clicked {
        selected = p;
    }
    if selected != initial_selected {
        ui.ctx().data_mut(|d| d.insert_temp(sel_key, selected));
    }
}

fn walk_demo_tree(
    ui: &mut egui::Ui,
    root_id: egui::Id,
    path: &'static str,
    depth: u32,
    selected: &str,
    accent: egui::Color32,
    clicked: &mut Option<String>,
) {
    let Some((p, name, icon, children, material)) = demo_tree_node(path) else {
        return;
    };
    let is_branch = !children.is_empty();
    // Per-node persisted state.
    let exp_key = root_id.with(("frost_demo_tree_expanded", *p));
    let eye_key = root_id.with(("frost_demo_tree_eye", *p));
    let lock_key = root_id.with(("frost_demo_tree_lock", *p));
    let mut expanded: bool = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<bool>(exp_key))
        .unwrap_or(true);
    let mut eye_on: bool = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<bool>(eye_key))
        .unwrap_or(true);
    let mut lock_on: bool = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<bool>(lock_key))
        .unwrap_or(false);
    // `Color` slots are read-only — the slot still requires a `&mut
    // bool` so the slice shape stays uniform across all rows; the
    // boolean is never read or flipped, so a throwaway local
    // suffices.
    let mut swatch_dummy = false;

    let mut slots = [
        corekit::widget::TreeIconSlot::new(corekit::widget::TreeIconKind::Eye, &mut eye_on)
            .with_tooltip("Toggle visibility"),
        corekit::widget::TreeIconSlot::new(corekit::widget::TreeIconKind::Lock, &mut lock_on)
            .with_tooltip("Toggle lock"),
        corekit::widget::TreeIconSlot::new(
            corekit::widget::TreeIconKind::Color(*material),
            &mut swatch_dummy,
        )
        .with_tooltip("Material colour"),
    ];
    let resp = corekit::widget::tree_row(
        ui,
        *p,
        depth,
        if is_branch { Some(&mut expanded) } else { None },
        Some(*icon),
        *name,
        selected == *p,
        accent,
        &mut slots,
    );
    if resp.body.clicked() {
        *clicked = Some((*p).to_string());
    }

    ui.ctx().data_mut(|d| {
        d.insert_persisted(exp_key, expanded);
        d.insert_persisted(eye_key, eye_on);
        d.insert_persisted(lock_key, lock_on);
    });

    if is_branch && expanded {
        for child in *children {
            walk_demo_tree(ui, root_id, child, depth + 1, selected, accent, clicked);
        }
    }
}

// ─── Demo node-graph (Snarl) ───────────────────────────────────────
//
// Tiny add-numbers graph reused from the legacy `demo.rs`. Purpose
// here is to prove `Pod::fill()` hosts heavy embedded canvases (node
// graph, code editor) without fighting the container's auto-fit.
//
// One `Snarl<GraphNode>` lives in egui ctx data per `pod_seed`, so
// each pane's graph keeps its own state across frames; clones are
// cheap (a few nodes + connections) so `get_temp` → render → write
// back round-tripping is fine.

#[derive(Clone)]
enum GraphNode {
    Number(f64),
    Add,
    Output,
}

impl GraphNode {
    fn title(&self) -> &'static str {
        match self {
            GraphNode::Number(_) => "Number",
            GraphNode::Add => "Add",
            GraphNode::Output => "Output",
        }
    }
    fn inputs(&self) -> usize {
        match self {
            GraphNode::Number(_) => 0,
            GraphNode::Add => 2,
            GraphNode::Output => 1,
        }
    }
    fn outputs(&self) -> usize {
        match self {
            GraphNode::Number(_) => 1,
            GraphNode::Add => 1,
            GraphNode::Output => 0,
        }
    }
}

fn eval_output(snarl: &Snarl<GraphNode>, pin: &OutPin) -> f64 {
    match snarl.get_node(pin.id.node) {
        Some(GraphNode::Number(v)) => *v,
        Some(GraphNode::Add) => {
            let mut sum = 0.0;
            for i in 0..2 {
                let in_pin = snarl.in_pin(InPinId { node: pin.id.node, input: i });
                for remote in &in_pin.remotes {
                    let out_pin = snarl.out_pin(*remote);
                    sum += eval_output(snarl, &out_pin);
                }
            }
            sum
        }
        _ => 0.0,
    }
}

fn eval_input(snarl: &Snarl<GraphNode>, pin: &InPin) -> f64 {
    pin.remotes
        .iter()
        .map(|remote| eval_output(snarl, &snarl.out_pin(*remote)))
        .sum()
}

#[derive(Default)]
struct GraphViewer;

impl SnarlViewer<GraphNode> for GraphViewer {
    fn title(&mut self, node: &GraphNode) -> String {
        node.title().into()
    }
    fn inputs(&mut self, node: &GraphNode) -> usize {
        node.inputs()
    }
    fn outputs(&mut self, node: &GraphNode) -> usize {
        node.outputs()
    }
    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<GraphNode>,
    ) -> impl SnarlPin + 'static {
        match snarl.get_node(pin.id.node) {
            Some(GraphNode::Add) => {
                let name = if pin.id.input == 0 { "a" } else { "b" };
                if pin.remotes.is_empty() {
                    ui.label(format!("{name} = 0"));
                } else {
                    ui.label(format!("{name} = {:.2}", eval_input(snarl, pin)));
                }
            }
            Some(GraphNode::Output) => {
                let v = eval_input(snarl, pin);
                ui.label(format!("= {v:.3}"));
            }
            _ => {}
        }
        PinInfo::circle()
    }
    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<GraphNode>,
    ) -> impl SnarlPin + 'static {
        if let Some(GraphNode::Number(v)) = snarl.get_node_mut(pin.id.node) {
            ui.add(egui::DragValue::new(v).speed(0.05).fixed_decimals(2));
        } else if let Some(GraphNode::Add) = snarl.get_node(pin.id.node) {
            let v = eval_output(snarl, pin);
            ui.label(format!("= {v:.3}"));
        }
        PinInfo::circle()
    }
    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<GraphNode>) -> bool {
        true
    }
    fn show_graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        snarl: &mut Snarl<GraphNode>,
    ) {
        ui.label("Add node");
        if ui.button("Number").clicked() {
            snarl.insert_node(pos, GraphNode::Number(0.0));
            ui.close();
        }
        if ui.button("Add").clicked() {
            snarl.insert_node(pos, GraphNode::Add);
            ui.close();
        }
        if ui.button("Output").clicked() {
            snarl.insert_node(pos, GraphNode::Output);
            ui.close();
        }
    }
}

fn default_graph() -> Snarl<GraphNode> {
    let mut g = Snarl::new();
    let a = g.insert_node(egui::pos2(30.0, 40.0), GraphNode::Number(2.0));
    let b = g.insert_node(egui::pos2(30.0, 130.0), GraphNode::Number(3.0));
    let add = g.insert_node(egui::pos2(220.0, 80.0), GraphNode::Add);
    let out = g.insert_node(egui::pos2(420.0, 80.0), GraphNode::Output);
    g.connect(OutPinId { node: a, output: 0 }, InPinId { node: add, input: 0 });
    g.connect(OutPinId { node: b, output: 0 }, InPinId { node: add, input: 1 });
    g.connect(OutPinId { node: add, output: 0 }, InPinId { node: out, input: 0 });
    g
}

const DEFAULT_CODE: &str = "// Frost code editor demo — Rust syntax highlighting.
fn fibonacci(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 2..=n {
        let next = a + b;
        a = b;
        b = next;
    }
    b
}

fn main() {
    let label = \"fib(20)\";
    println!(\"{label} = {}\", fibonacci(20));
}
";
