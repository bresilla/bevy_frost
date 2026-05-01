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
                .resize(corekit::pane::PaneResize::BOTH)
                .show(ctx, |body_ui| {
                    // Three independent containers, each with its own
                    // toggle state (id derived from the pane's button
                    // id + an index).
                    const CONTAINERS_PER_PANE: usize = 3;
                    // Approximate per-container chrome on the main
                    // axis (title strip + section padding + outer
                    // margins + title-body gap). Used to convert the
                    // pane's user-resized body main extent into a
                    // body_flow per container so the containers
                    // collectively fill the pane on the flow axis.
                    const CHROME_PER_CONTAINER: f32 = 38.0;
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
                    // Pull the pane's user-resized body main extent
                    // and split it across the containers, subtracting
                    // each one's chrome. Resizing the pane on its
                    // inner edge (drag handle) updates this value;
                    // the next frame's body_flow_per_container grows
                    // / shrinks accordingly so the stack of
                    // containers always spans the pane edge-to-edge.
                    let pane_body_flow = corekit::pane::user_flow(
                        body_ui.ctx(),
                        pane_egui_id,
                    );
                    // No artificial floor on the per-container body
                    // slot — the pane's flow-axis resize handle
                    // already refuses to shrink below the registered
                    // container chrome floor, so when the user pushes
                    // the pane to that floor `body_flow_per_container`
                    // naturally lands at 0 and each container shows
                    // just its title strip without overflow.
                    let body_flow_per_container = ((pane_body_flow
                        - CHROME_PER_CONTAINER * CONTAINERS_PER_PANE as f32)
                        / CONTAINERS_PER_PANE as f32)
                        .max(0.0);
                    for cid in order {
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
                        let pod_count = ((seed % 4) as usize) + 1;
                        let pods: Vec<_> = (0..pod_count)
                            .map(|p| {
                                let pod_id = cid.with(("newui_pod", p));
                                let pod_seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                                    ^ (p as u64);
                                let search_count =
                                    ((pod_seed % 3) as usize) + 1;
                                let mut pod = corekit::pod::Pod::new(pod_id);
                                for s in 0..search_count {
                                    pod = pod.with_search(
                                        format!("pod {} · search {}…", p + 1, s + 1),
                                        accent_col,
                                    );
                                }
                                pod
                            })
                            .collect();
                        Normal::new(title, anchor, accent_col, cid)
                            .icon("settings")
                            .body_flow(body_flow_per_container)
                            .show(body_ui, pods);
                    }
                });
        }
    }
}
