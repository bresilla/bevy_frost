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

use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver};
use bevy::mesh::PrimitiveTopology;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use bevy_frost::prelude::*;
use bevy_frost::style::srgb_to_egui;

// Menu identifiers. One per button. `SideActive` holds "which of
// these is open on the left rail / on the right rail" (at most one
// each) — we just compare these constants against it.
const MENU_WIDGETS: &str = "demo_menu_widgets";
const MENU_CONTAINERS: &str = "demo_menu_containers";
const MENU_SCENE: &str = "demo_menu_scene";
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
        .init_resource::<GroundGrid>()
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (camera_control, camera_zoom, update_grid).chain(),
        )
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
    position: [f64; 3],
    rotation_deg: [f64; 3],

    // Scene outliner — list of fake entities, a transient
    // `selected` (set by row click), and a durable `following`
    // (set by the right-edge radio only).
    scene_entities: Vec<String>,
    scene_selected: Option<usize>,
    scene_following: Option<usize>,
    /// Bumped every time you double-click a row — demo readout that
    /// proves the body's double-click is independent of the radio.
    scene_double_click_count: u32,
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
            position: [0.0, 1.5, 0.0],
            rotation_deg: [0.0, 45.0, 0.0],
            scene_entities: vec![
                "Planet".to_string(),
                "CloudShell".to_string(),
                "Sun".to_string(),
                "Grid[L0]".to_string(),
                "Grid[L1]".to_string(),
                "Grid[L2]".to_string(),
                "OriginCube".to_string(),
            ],
            scene_selected: Some(0),
            scene_following: None,
            scene_double_click_count: 0,
        }
    }
}

// ─── Scene setup ────────────────────────────────────────────────────
//
// Stripped-down copy of gearbox's "globe" scene: a curved, Earth-
// sized tan planet sphere for ground, a translucent cloud shell a
// few km above it, a LOD line-grid that tracks the camera with
// per-level fade + major/minor line emphasis + radial edge fade,
// atmospheric distance fog for horizon falloff, and a cascaded sun.

/// Earth radius in metres.
const PLANET_RADIUS: f32 = 6_371_000.0;
/// How high above the ground the cloud shell sits.
const CLOUD_ALTITUDE_M: f32 = 4_000.0;

// ── Grid LOD constants (copied from gearbox_viz::grid) ──────────────

/// Cell size per level (metres). Decades apart.
const LEVEL_STEPS: [f32; 4] = [1.0, 10.0, 100.0, 1_000.0];
/// Half-extent of each level's square (metres).
const LEVEL_HALF: [f32; 4] = [50.0, 500.0, 5_000.0, 50_000.0];
/// Every Nth line is a major line (brighter alpha).
const MAJOR_EVERY: i32 = 10;
/// Major-line alpha boost (multiplied against the base colour alpha).
const MAJOR_BOOST: f32 = 2.2;
/// Grid rides this height above the tangent plane.
const GRID_Y: f32 = 0.05;
/// Gaussian fade params over `log10(cam_dist / step)`.
const GAUSS_PEAK: f32 = 1.0;
const GAUSS_WIDTH: f32 = 0.55;

#[derive(Resource, Clone, Copy)]
struct GroundGrid {
    color: Color,
}

impl Default for GroundGrid {
    fn default() -> Self {
        Self {
            color: Color::srgba(80.0 / 255.0, 70.0 / 255.0, 70.0 / 255.0, 0.35),
        }
    }
}

#[derive(Component)]
struct LocalGrid {
    level: u8,
    material: Handle<StandardMaterial>,
}

// ── Chase camera (simplified copy of gearbox_viz::camera) ───────────

/// Orbit camera rig — pan / orbit / zoom like gearbox.
#[derive(Component, Clone)]
struct ChaseCamera {
    focus: Vec3,
    yaw: f32,
    elevation: f32,
    distance: f32,
    min_distance: f32,
    max_distance: f32,
    pan_sensitivity: f32,
    orbit_speed: f32,
    zoom_step: f64,
    zoom_smoothing: f64,
    last_middle_click_secs: f32,
}

impl Default for ChaseCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            yaw: 0.0,
            elevation: 25f32.to_radians(),
            distance: 14.0,
            min_distance: 3.0,
            max_distance: 1_000.0,
            pan_sensitivity: 0.0012,
            orbit_speed: 0.005,
            zoom_step: 0.05,
            zoom_smoothing: 6.0,
            last_middle_click_secs: -10.0,
        }
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid_cfg: Res<GroundGrid>,
) {
    // ── Planet sphere — huge, warm tan ground. y=-radius so tangent
    //    point (the local "floor") is at world y=0.
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

    // ── Cloud shell — translucent white sphere slightly larger than
    //    the planet, gives you a sky to look at without requiring a
    //    full textured cloud layer.
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

    // ── LOD ground grid — four line-meshes (1 m / 10 m / 100 m /
    //    1 km cells). Each level fades in when its cell size best
    //    matches the current zoom and slides with the camera focus.
    for level in 0..LEVEL_STEPS.len() {
        let step = LEVEL_STEPS[level];
        let half = LEVEL_HALF[level];
        let mesh = meshes.add(build_level_mesh(&grid_cfg, step, half));
        let mat = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            ..default()
        });
        commands.spawn((
            Name::new(format!("LocalGrid[L{level}]")),
            LocalGrid { level: level as u8, material: mat.clone() },
            Transform::from_xyz(0.0, GRID_Y, 0.0),
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            NotShadowCaster,
            Visibility::Visible,
        ));
    }

    // ── Origin marker — small accent-tinted cube so the scene isn't
    //    totally empty.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_length(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.65, 0.54, 0.98),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // ── Sun — single cascade, ~100 m max, matches gearbox's
    //    vehicle-neighbourhood shadow quality.
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

    // ── Camera + atmospheric fog + ambient fill. Fog extinction tuned
    //    to Rayleigh-ish falloff so the horizon gently shifts toward
    //    sky blue. Far plane extends past the planet so the cloud
    //    shell isn't clipped.
    let projection = Projection::Perspective(PerspectiveProjection {
        near: 0.1,
        far: PLANET_RADIUS * 2.5,
        ..default()
    });
    let fog = DistanceFog {
        color: Color::srgb(0.55, 0.70, 0.86),
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

// ─── Grid LOD mesh ──────────────────────────────────────────────────

/// Build one LOD level's mesh — a `LineList` XZ grid with per-line
/// radial alpha fade and a `MAJOR_EVERY` brightness boost on every
/// tenth line. Matches gearbox's grid look 1:1.
fn build_level_mesh(cfg: &GroundGrid, step: f32, half: f32) -> Mesh {
    let s = cfg.color.to_srgba();
    let base_rgba = [s.red, s.green, s.blue, s.alpha];

    let n = (half / step) as i32;
    let total_lines = (2 * n + 1) * 2;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((total_lines * 2) as usize);
    let mut colors: Vec<[f32; 4]> = Vec::with_capacity((total_lines * 2) as usize);

    let line_color = |i: i32| -> [f32; 4] {
        let t = (i.abs() as f32) / (n as f32);
        let edge_fade = {
            let u = (1.0 - t).clamp(0.0, 1.0);
            u * u * (3.0 - 2.0 * u) // smoothstep
        };
        let major = i.rem_euclid(MAJOR_EVERY) == 0;
        let boost = if major { MAJOR_BOOST } else { 1.0 };
        [
            base_rgba[0],
            base_rgba[1],
            base_rgba[2],
            (base_rgba[3] * edge_fade * boost).clamp(0.0, 1.0),
        ]
    };

    for i in -n..=n {
        let z = i as f32 * step;
        let c = line_color(i);
        positions.push([-half, 0.0, z]);
        positions.push([half, 0.0, z]);
        colors.push(c);
        colors.push(c);
    }
    for i in -n..=n {
        let x = i as f32 * step;
        let c = line_color(i);
        positions.push([x, 0.0, -half]);
        positions.push([x, 0.0, half]);
        colors.push(c);
        colors.push(c);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::LineList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}

fn level_fade(cam_dist: f32, step: f32) -> f32 {
    let log_r = (cam_dist / step).max(1e-3).log10();
    let z = (log_r - GAUSS_PEAK) / GAUSS_WIDTH;
    (-0.5 * z * z).exp()
}

/// Slide every grid level with the chase-camera focus, snapped to
/// the major step so the lines stay world-aligned, and write each
/// level's Gaussian fade into its material alpha.
fn update_grid(
    cameras: Query<&ChaseCamera>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cfg: Res<GroundGrid>,
    mut grids: Query<(&LocalGrid, &mut Transform, &mut Visibility)>,
) {
    let Ok(cam) = cameras.single() else { return };
    let cam_dist = cam.distance.max(0.1);

    for (grid, mut tr, mut vis) in grids.iter_mut() {
        let step = LEVEL_STEPS[grid.level as usize];
        let snap_step = step * MAJOR_EVERY as f32;
        tr.translation.x = (cam.focus.x / snap_step).round() * snap_step;
        tr.translation.y = GRID_Y;
        tr.translation.z = (cam.focus.z / snap_step).round() * snap_step;

        let fade = level_fade(cam_dist, step);
        let a = cfg.color.alpha() * fade;
        *vis = if a > 0.005 { Visibility::Visible } else { Visibility::Hidden };
        if let Some(m) = materials.get_mut(&grid.material) {
            let srgba = cfg.color.to_srgba();
            m.base_color = Color::srgba(srgba.red, srgba.green, srgba.blue, a);
        }
    }
}

// ─── Camera control systems ─────────────────────────────────────────

fn apply_rig(cam: &ChaseCamera, tr: &mut Transform) {
    let horizontal = cam.distance * cam.elevation.cos();
    let vertical = cam.distance * cam.elevation.sin();
    let offset = Vec3::new(
        horizontal * cam.yaw.sin(),
        vertical,
        horizontal * cam.yaw.cos(),
    );
    let cam_world = cam.focus + offset;
    *tr = Transform::from_translation(cam_world).looking_at(cam.focus, Vec3::Y);
}

fn cursor_ray_to_ground(camera: &Camera, cam_tr: &GlobalTransform, cursor: Vec2) -> Option<Vec3> {
    let ray = camera.viewport_to_world(cam_tr, cursor).ok()?;
    let origin = ray.origin;
    let direction = *ray.direction;
    if direction.y.abs() < 1e-6 {
        return None;
    }
    let t = -origin.y / direction.y;
    if t < 0.0 {
        return None;
    }
    Some(origin + direction * t)
}

/// Middle-drag → pan; Left+Right-drag → orbit; double-middle-click →
/// snap focus to cursor's world point. Same bindings as gearbox.
fn camera_control(
    time: Res<Time>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    bevy_cameras: Query<(&Camera, &GlobalTransform)>,
    mut contexts: EguiContexts,
    mut pan_anchor: Local<Option<Vec2>>,
    mut orbit_anchor: Local<Option<Vec2>>,
    mut cameras: Query<(&mut ChaseCamera, &mut Transform)>,
) {
    // Don't hijack mouse gestures when the pointer is over an egui
    // panel / ribbon — lets panels scroll, sliders drag, buttons
    // click without also panning the world.
    if contexts.ctx_mut().map(|c| c.wants_pointer_input()).unwrap_or(false) {
        *pan_anchor = None;
        *orbit_anchor = None;
        return;
    }

    let middle_pressed = mouse_buttons.pressed(MouseButton::Middle);
    let left_pressed = mouse_buttons.pressed(MouseButton::Left);
    let right_pressed = mouse_buttons.pressed(MouseButton::Right);
    let both_lr = left_pressed && right_pressed;

    if !middle_pressed {
        *pan_anchor = None;
    }
    if !both_lr {
        *orbit_anchor = None;
    }

    let cursor_position = primary_window.single().ok().and_then(|w| w.cursor_position());

    let mut pan_delta = Vec2::ZERO;
    if middle_pressed {
        if let Some(pos) = cursor_position {
            if let Some(anchor) = *pan_anchor {
                pan_delta = pos - anchor;
            }
            *pan_anchor = Some(pos);
        }
    }
    let mut orbit_delta = Vec2::ZERO;
    if both_lr {
        if let Some(pos) = cursor_position {
            if orbit_anchor.is_none() {
                *orbit_anchor = Some(pos);
            }
            if let Some(anchor) = *orbit_anchor {
                orbit_delta = pos - anchor;
            }
            *orbit_anchor = Some(pos);
        }
    }

    let now = time.elapsed_secs();

    for (mut cam, mut tr) in &mut cameras {
        if mouse_buttons.just_pressed(MouseButton::Middle) {
            let is_double = now - cam.last_middle_click_secs < 0.35;
            cam.last_middle_click_secs = now;
            if is_double {
                if let (Some(cursor), Ok((camera, cam_tr))) =
                    (cursor_position, bevy_cameras.single())
                {
                    if let Some(hit) = cursor_ray_to_ground(camera, cam_tr, cursor) {
                        cam.focus = hit;
                    }
                }
            }
        }

        if pan_delta != Vec2::ZERO {
            let pan_speed = cam.distance * cam.pan_sensitivity;
            let forward = Vec3::new(cam.yaw.sin(), 0.0, cam.yaw.cos());
            let right = Vec3::new(forward.z, 0.0, -forward.x);
            cam.focus += (-right * pan_delta.x - forward * pan_delta.y) * pan_speed;
        }
        if orbit_delta != Vec2::ZERO {
            cam.yaw -= orbit_delta.x * cam.orbit_speed;
            cam.elevation += orbit_delta.y * cam.orbit_speed;
            cam.elevation = cam.elevation.clamp(5f32.to_radians(), 89f32.to_radians());
        }

        apply_rig(&cam, &mut tr);
    }
}

/// Scroll-wheel zoom — logarithmic with exponential smoothing.
fn camera_zoom(
    time: Res<Time>,
    mut contexts: EguiContexts,
    mut wheel: MessageReader<MouseWheel>,
    mut zoom_target: Local<Option<f64>>,
    mut cameras: Query<(&mut ChaseCamera, &mut Transform)>,
) {
    // Don't consume the wheel if egui wants it (e.g. scrolling a
    // panel list). Drain the reader so events don't queue up.
    let over_ui = contexts
        .ctx_mut()
        .map(|c| c.wants_pointer_input())
        .unwrap_or(false);
    if over_ui {
        wheel.read().for_each(drop);
        return;
    }

    let mut scroll_delta = 0.0_f64;
    for event in wheel.read() {
        scroll_delta += match event.unit {
            MouseScrollUnit::Line => event.y as f64,
            MouseScrollUnit::Pixel => event.y as f64 / 32.0,
        };
    }

    let Ok((mut cam, mut tr)) = cameras.single_mut() else { return };

    let target = zoom_target.get_or_insert(cam.distance as f64);
    let min = cam.min_distance as f64;
    let max = cam.max_distance as f64;

    if scroll_delta != 0.0 {
        let log_target = target.max(0.1).log10();
        let new_log = log_target - scroll_delta * cam.zoom_step;
        *target = 10f64.powf(new_log).clamp(min, max);
    }

    let dt = time.delta_secs_f64();
    let log_current = (cam.distance as f64).max(0.1).ln();
    let log_target = target.max(0.1).ln();
    let log_diff = log_target - log_current;
    if log_diff.abs() > 1e-4 {
        let new_log = log_current + log_diff * (cam.zoom_smoothing * dt).min(0.9);
        cam.distance = new_log.exp() as f32;
        apply_rig(&cam, &mut tr);
    } else if log_diff.abs() > 1e-5 {
        cam.distance = *target as f32;
        apply_rig(&cam, &mut tr);
    }
}

// ─── Ribbons — draggable menu-toggle buttons ────────────────────────
//
// `RibbonLayout` (as opposed to the simpler, static `SideRibbon`) is
// what makes buttons draggable: the user can pick one up from the
// left rail and drop it onto the right rail, and the panel that
// opens for that button automatically re-anchors to the rail where
// the button currently lives. `SideActive::is_menu_open(&layout, id)`
// asks the layout for the button's current side, so exclusivity
// follows the drag — one open menu per rail, no matter where the
// button was originally declared.

fn draw_ribbons(
    mut contexts: EguiContexts,
    accent: Res<AccentColor>,
    mut side_active: ResMut<SideActive>,
    mut layout: ResMut<RibbonLayout>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let accent_col = accent.0;

    // If a button changed sides on the previous frame, drop whatever
    // `SideActive` was remembering there — otherwise the ribbon
    // highlight and the panel anchor disagree for a frame.
    side_active.invalidate_stale(&layout);

    // Snapshot open state so the button closure can write a click
    // flag without conflicting with the immutable borrow below.
    let widgets_open = side_active.is_menu_open(&layout, MENU_WIDGETS);
    let containers_open = side_active.is_menu_open(&layout, MENU_CONTAINERS);
    let scene_open = side_active.is_menu_open(&layout, MENU_SCENE);
    let theme_open = side_active.is_menu_open(&layout, MENU_THEME);
    let about_open = side_active.is_menu_open(&layout, MENU_ABOUT);

    let mut click_widgets = false;
    let mut click_containers = false;
    let mut click_scene = false;
    let mut click_theme = false;
    let mut click_about = false;

    // Left rail — draggable to the right rail; can NOT leave the side
    // rails (SideRails constraint).
    layout.button(
        ctx, MENU_WIDGETS,
        RibbonConstraint::SideRails, RibbonKind::Left, 0,
        "W", "Widgets gallery",
        widgets_open, accent_col,
        || { click_widgets = true; },
    );
    layout.button(
        ctx, MENU_CONTAINERS,
        RibbonConstraint::SideRails, RibbonKind::Left, 1,
        "C", "Containers showcase",
        containers_open, accent_col,
        || { click_containers = true; },
    );
    layout.button(
        ctx, MENU_SCENE,
        RibbonConstraint::SideRails, RibbonKind::Left, 2,
        "S", "Scene outliner",
        scene_open, accent_col,
        || { click_scene = true; },
    );
    layout.button(
        ctx, MENU_THEME,
        RibbonConstraint::SideRails, RibbonKind::Right, 0,
        "T", "Theme & colour",
        theme_open, accent_col,
        || { click_theme = true; },
    );
    layout.button(
        ctx, MENU_ABOUT,
        RibbonConstraint::SideRails, RibbonKind::Right, 1,
        "?", "About this demo",
        about_open, accent_col,
        || { click_about = true; },
    );

    // Apply clicks after the immutable borrow above has ended.
    if click_widgets { side_active.toggle_menu(&layout, MENU_WIDGETS); }
    if click_containers { side_active.toggle_menu(&layout, MENU_CONTAINERS); }
    if click_scene { side_active.toggle_menu(&layout, MENU_SCENE); }
    if click_theme { side_active.toggle_menu(&layout, MENU_THEME); }
    if click_about { side_active.toggle_menu(&layout, MENU_ABOUT); }
}

// ─── Panels — each anchors to whichever rail its button sits on ─────

fn draw_panels(
    mut contexts: EguiContexts,
    side_active: Res<SideActive>,
    layout: Res<RibbonLayout>,
    mut accent: ResMut<AccentColor>,
    mut glass: ResMut<GlassOpacity>,
    mut state: ResMut<DemoState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let accent_col = accent.0;
    let mut keep_open = true;

    if side_active.is_menu_open(&layout, MENU_WIDGETS) {
        floating_window(
            ctx,
            "demo_panel_widgets",
            "Widgets",
            layout.panel_anchor(MENU_WIDGETS),
            egui::vec2(320.0, 600.0),
            &mut keep_open,
            accent_col,
            |ui| widgets_panel(ui, &mut state, accent_col),
        );
    }
    if side_active.is_menu_open(&layout, MENU_CONTAINERS) {
        floating_window(
            ctx,
            "demo_panel_containers",
            "Containers",
            layout.panel_anchor(MENU_CONTAINERS),
            egui::vec2(320.0, 400.0),
            &mut keep_open,
            accent_col,
            |ui| containers_panel(ui, &mut state, accent_col),
        );
    }
    if side_active.is_menu_open(&layout, MENU_SCENE) {
        floating_window(
            ctx,
            "demo_panel_scene",
            "Elements",
            layout.panel_anchor(MENU_SCENE),
            egui::vec2(320.0, 360.0),
            &mut keep_open,
            accent_col,
            |ui| elements_panel(ui, &mut state, accent_col),
        );
    }
    if side_active.is_menu_open(&layout, MENU_THEME) {
        floating_window(
            ctx,
            "demo_panel_theme",
            "Theme",
            layout.panel_anchor(MENU_THEME),
            egui::vec2(300.0, 280.0),
            &mut keep_open,
            accent_col,
            |ui| theme_panel(ui, &mut accent, &mut glass, accent_col),
        );
    }
    if side_active.is_menu_open(&layout, MENU_ABOUT) {
        floating_window(
            ctx,
            "demo_panel_about",
            "About",
            layout.panel_anchor(MENU_ABOUT),
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

fn elements_panel(ui: &mut egui::Ui, state: &mut DemoState, accent: egui::Color32) {
    section(ui, "demo_elements", "Elements", accent, true, |ui| {
        // Scrollable list of rows. Body click = transient select,
        // body double-click = arbitrary action (here we bump a
        // counter), right-edge radio = durable "pin" (one-at-a-time).
        // The two click targets do NOT leak into each other.
        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                for (idx, name) in state.scene_entities.iter().enumerate() {
                    let selected = state.scene_selected == Some(idx);
                    let pinned = state.scene_following == Some(idx);
                    let trailing = format!("#{idx}");
                    let resp = hybrid_select_row(
                        ui,
                        idx,
                        name,
                        Some(&trailing),
                        selected,
                        pinned,
                        accent,
                    );
                    if resp.body.clicked() {
                        state.scene_selected = Some(idx);
                    }
                    if resp.body.double_clicked() {
                        state.scene_double_click_count =
                            state.scene_double_click_count.wrapping_add(1);
                    }
                    if resp.radio.clicked() {
                        state.scene_following = if pinned { None } else { Some(idx) };
                    }
                }
            });

        // Readouts that prove the split semantics — sit in the same
        // container so you can see the list + its state side by side.
        row_separator(ui);
        let selected = state
            .scene_selected
            .and_then(|i| state.scene_entities.get(i))
            .cloned()
            .unwrap_or_else(|| "—".into());
        let pinned = state
            .scene_following
            .and_then(|i| state.scene_entities.get(i))
            .cloned()
            .unwrap_or_else(|| "—".into());
        readout_row(ui, "selected (transient)", &selected);
        readout_row(ui, "pinned (durable)", &pinned);
        readout_row(
            ui,
            "double-clicks",
            &state.scene_double_click_count.to_string(),
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

fn theme_panel(
    ui: &mut egui::Ui,
    accent_res: &mut AccentColor,
    glass: &mut GlassOpacity,
    accent: egui::Color32,
) {
    // Accent. Mutating `AccentColor` triggers the crate's `apply_theme`
    // system, which re-paints *every* widget — buttons, frames,
    // borders, slider fills, the lot — from this single source of
    // truth.
    section(ui, "demo_theme_colour", "Accent", accent, true, |ui| {
        let c = accent_res.0;
        let mut rgb = [
            c.r() as f32 / 255.0,
            c.g() as f32 / 255.0,
            c.b() as f32 / 255.0,
        ];
        if color_rgb(ui, "accent", &mut rgb).changed() {
            accent_res.0 = srgb_to_egui(rgb);
        }
        sub_caption(
            ui,
            "Changing this recolours every widget in the app — one resource, one brush.",
        );
    });

    // Glass opacity — ditto. `GlassOpacity(u8)` in `0..=100`.
    section(ui, "demo_theme_glass", "Glass", accent, true, |ui| {
        let mut v = glass.0 as f64;
        if pretty_slider(ui, "opacity", &mut v, 1.0..=100.0, 0, "%", accent).changed() {
            glass.0 = v.round().clamp(1.0, 100.0) as u8;
        }
        sub_caption(
            ui,
            "Lower values let the 3D scene bleed through every panel.",
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
