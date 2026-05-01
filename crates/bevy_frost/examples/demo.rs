//! `bevy_frost` widget gallery + layout showcase, reimplemented on
//! top of `corekit` (the new pane / ribbon / container / pod /
//! widget stack). Mirrors the layout of the legacy frostcore demo
//! one panel at a time:
//!
//! * **Widgets** — Flags / Numbers / Bars / Buttons / Animated.
//! * **Containers** — Position + Rotation, axis-coloured drag values.
//! * **Elements** — scene tree (eye/lock/colour slots) + flat
//!   hybrid_select roster.
//! * **Theme** — Profile dropdowns + accent picker + glass slider.
//! * **Keys** — keybinding rows (readouts).
//! * **About** — version + dependency readouts.
//!
//! Run: `cargo run -p bevy_frost --example demo`.

use bevy::light::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use bevy_glacial::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use corekit::container::{Normal, SeparatorStyle};
use corekit::pane::{PaneAnchor, Pane2, RailZone};
use corekit::pod::Pod;
use corekit::ribbon::{
    draw_assembly, find_item, find_ribbon, RibbonCluster, RibbonDef, RibbonDrag, RibbonEdge,
    RibbonGlyph, RibbonItem, RibbonMode, RibbonOpen, RibbonPlacement, RibbonRole,
};
use corekit::style::{srgb_to_egui, AccentColor, GlassOpacity, Mode};
use corekit::widget::{FillStyle, TreeIconKind, TreeIconSlot};
// Vendored extras — node graph (`egui-snarl`) and code editor
// (`egui_code_editor`). Both live under `bevy_frost::extras`.
use bevy_frost::extras::code::{frost_code_editor, Syntax};
use bevy_frost::extras::graph::{
    frost_snarl, InPin, InPinId, OutPin, OutPinId, PinInfo, Snarl, SnarlPin, SnarlViewer,
};

// ─── Ribbon / pane ids ──────────────────────────────────────────────

const RIBBON_LEFT:   &str = "demo_ribbon_left";
const RIBBON_RIGHT:  &str = "demo_ribbon_right";
const RIBBON_TOP:    &str = "demo_ribbon_top";
const RIBBON_BOTTOM: &str = "demo_ribbon_bottom";

const PANE_WIDGETS:    &str = "demo_pane_widgets";
const PANE_CONTAINERS: &str = "demo_pane_containers";
const PANE_SCENE:      &str = "demo_pane_scene";
const PANE_EDITOR:     &str = "demo_pane_editor";
const PANE_THEME:      &str = "demo_pane_theme";
const PANE_KEYS:       &str = "demo_pane_keys";
const PANE_ABOUT:      &str = "demo_pane_about";

const ACTION_PREV_CUBE: &str = "demo_action_prev_cube";
const ACTION_NEXT_CUBE: &str = "demo_action_next_cube";

const PANE_DEFS: &[(&str, &str, PaneAnchor, &str)] = &[
    (RIBBON_LEFT,   PANE_WIDGETS,    PaneAnchor::LeftRail(RailZone::Start),    "Widgets"),
    (RIBBON_LEFT,   PANE_CONTAINERS, PaneAnchor::LeftRail(RailZone::Middle),   "Containers"),
    (RIBBON_LEFT,   PANE_SCENE,      PaneAnchor::LeftRail(RailZone::End),      "Elements"),
    (RIBBON_RIGHT,  PANE_THEME,      PaneAnchor::RightRail(RailZone::Start),   "Theme"),
    (RIBBON_RIGHT,  PANE_KEYS,       PaneAnchor::RightRail(RailZone::Middle),  "Keys"),
    (RIBBON_TOP,    PANE_ABOUT,      PaneAnchor::TopRail(RailZone::Start),     "About"),
    (RIBBON_BOTTOM, PANE_EDITOR,     PaneAnchor::BottomRail(RailZone::Start),  "Editor"),
];

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
    // LEFT rail — primary navigation cluster.
    RibbonItem { id: PANE_WIDGETS,    ribbon: RIBBON_LEFT,   cluster: RibbonCluster::Start, slot: 0,
                 glyph: RibbonGlyph::Icon("apps"),       tooltip: "Widgets gallery",     child_ribbon: None, role: None },
    RibbonItem { id: PANE_CONTAINERS, ribbon: RIBBON_LEFT,   cluster: RibbonCluster::Start, slot: 1,
                 glyph: RibbonGlyph::Icon("box"),        tooltip: "Containers showcase", child_ribbon: None, role: None },
    RibbonItem { id: PANE_SCENE,      ribbon: RIBBON_LEFT,   cluster: RibbonCluster::Start, slot: 2,
                 glyph: RibbonGlyph::Icon("folder"),     tooltip: "Scene outliner",      child_ribbon: None, role: None },
    // RIGHT rail — theme + input.
    RibbonItem { id: PANE_THEME,      ribbon: RIBBON_RIGHT,  cluster: RibbonCluster::Start, slot: 0,
                 glyph: RibbonGlyph::Icon("color"),      tooltip: "Theme & colour",      child_ribbon: None, role: None },
    RibbonItem { id: PANE_KEYS,       ribbon: RIBBON_RIGHT,  cluster: RibbonCluster::Start, slot: 1,
                 glyph: RibbonGlyph::Icon("keyboard"),   tooltip: "Keys & gestures",     child_ribbon: None, role: None },
    // TOP rail — meta.
    RibbonItem { id: PANE_ABOUT,      ribbon: RIBBON_TOP,    cluster: RibbonCluster::Start, slot: 0,
                 glyph: RibbonGlyph::Icon("info"),       tooltip: "About this demo",     child_ribbon: None, role: None },
    // BOTTOM rail — Editor (placeholder; the legacy graph + code
    // wrappers lived in `frostcore` which has been removed) and the
    // one-shot cube-cycle action buttons in the End cluster.
    RibbonItem { id: PANE_EDITOR,     ribbon: RIBBON_BOTTOM, cluster: RibbonCluster::Start, slot: 0,
                 glyph: RibbonGlyph::Icon("flowchart"),  tooltip: "Editor",              child_ribbon: None, role: None },
    RibbonItem { id: ACTION_PREV_CUBE, ribbon: RIBBON_BOTTOM, cluster: RibbonCluster::End,   slot: 0,
                 glyph: RibbonGlyph::Icon("arrow-left"),  tooltip: "Previous cube",
                 child_ribbon: None, role: Some(RibbonRole::Icon) },
    RibbonItem { id: ACTION_NEXT_CUBE, ribbon: RIBBON_BOTTOM, cluster: RibbonCluster::End,   slot: 1,
                 glyph: RibbonGlyph::Icon("arrow-right"), tooltip: "Next cube",
                 child_ribbon: None, role: Some(RibbonRole::Icon) },
];

// ─── Theme + scene state ───────────────────────────────────────────

#[derive(Resource, Clone, Copy, Debug, Default)]
struct ThemeFamily(u8);

#[derive(Resource, Clone, Copy, Debug, Default)]
struct ThemeModeRes(u8);

#[derive(Resource, Clone, Copy, Debug)]
struct PastelToggle(bool);
impl Default for PastelToggle { fn default() -> Self { Self(true) } }

#[derive(Resource, Clone, Copy, Debug)]
struct TintRgba(pub [f32; 4]);
impl Default for TintRgba { fn default() -> Self { Self([0.5, 0.7, 0.9, 0.6]) } }

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
    let geometry = WindowGeometry::load("bevy_frost_demo");
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(geometry.to_window("bevy_frost — demo")),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    level: bevy::log::Level::INFO,
                    filter: "info,wgpu=error,bevy_render=error,bevy_winit=error,naga=warn"
                        .into(),
                    ..default()
                }),
        )
        .add_plugins(bevy_egui::EguiPlugin::default())
        .add_plugins(bevy_frost::EguiInputAbsorbPlugin)
        .add_plugins(GlacialPlugins)
        .add_plugins(WindowSettingsPlugin::new("bevy_frost_demo"))
        .insert_resource(ClearColor(Color::srgb(0.06, 0.08, 0.12)))
        .insert_resource(GroundGrid {
            visible: true,
            color: Color::srgba(0.30, 0.38, 0.50, 0.42),
        })
        .init_resource::<AccentColor>()
        .init_resource::<GlassOpacity>()
        .init_resource::<RibbonOpen>()
        .init_resource::<RibbonPlacement>()
        .init_resource::<RibbonDrag>()
        .init_resource::<ThemeFamily>()
        .init_resource::<ThemeModeRes>()
        .init_resource::<PastelToggle>()
        .init_resource::<TintRgba>()
        .init_resource::<SelectedSwatch>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (pick_cube, update_swatch_selection))
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

// ─── Scene setup ───────────────────────────────────────────────────

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

// ─── Cube picking ──────────────────────────────────────────────────

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
    mut cubes: Query<(Entity, &ColorCube, &MeshMaterial3d<StandardMaterial>, &mut Transform)>,
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
            mat.emissive =
                LinearRgba::new(base.red * gain, base.green * gain, base.blue * gain, 1.0);
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

// ─── UI ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn ui_system(
    mut contexts: EguiContexts,
    mut accent: ResMut<AccentColor>,
    mut glass: ResMut<GlassOpacity>,
    mut open: ResMut<RibbonOpen>,
    mut placement: ResMut<RibbonPlacement>,
    mut drag: ResMut<RibbonDrag>,
    mut family: ResMut<ThemeFamily>,
    mut mode: ResMut<ThemeModeRes>,
    mut pastel: ResMut<PastelToggle>,
    mut tint: ResMut<TintRgba>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

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

    let accent_col = corekit::style::active_accent();
    let clicks = draw_assembly(
        ctx, accent_col, RIBBONS, RIBBON_ITEMS,
        &mut open, &mut placement, &mut drag,
        |_| false,
    );
    // PREV / NEXT cube — one-shot icon buttons in the BOTTOM rail's
    // End cluster. Each click rotates the AccentColor through the
    // hardcoded swatch row defined in `setup_scene`. Mirrors the
    // legacy demo's `ACTION_PREV_CUBE` / `ACTION_NEXT_CUBE` flow,
    // minus the `SelectedSwatch` entity bookkeeping (we just bump
    // the colour — the next swatch click on the cube updates the
    // entity highlight).
    const SWATCH_RGB: &[(u8, u8, u8)] = &[
        (230, 76, 76), (242, 166, 51), (242, 230, 76),
        (89, 217, 115), (76, 153, 242), (191, 115, 242),
    ];
    for click in clicks {
        if click.item == ACTION_PREV_CUBE || click.item == ACTION_NEXT_CUBE {
            let cur = accent.0;
            let cur_idx = SWATCH_RGB
                .iter()
                .position(|&(r, g, b)| egui::Color32::from_rgb(r, g, b) == cur)
                .unwrap_or(0);
            let next_idx = if click.item == ACTION_PREV_CUBE {
                (cur_idx + SWATCH_RGB.len() - 1) % SWATCH_RGB.len()
            } else {
                (cur_idx + 1) % SWATCH_RGB.len()
            };
            let (r, g, b) = SWATCH_RGB[next_idx];
            accent.0 = egui::Color32::from_rgb(r, g, b);
        }
    }

    let is_open = |id: &'static str| -> bool {
        let Some(item) = find_item(RIBBON_ITEMS, id) else { return false };
        let (rid, _, _) = placement.resolve(item);
        open.is_open(rid, id)
    };
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
        if !is_open(button_id) { continue; }
        let anchor = live_anchor(button_id).unwrap_or(default_anchor);
        Pane2::new(button_id, label, anchor, accent_col)
            .resize(corekit::pane::PaneResize::SPAN)
            .show(ctx, |body_ui| match button_id {
                PANE_WIDGETS    => widgets_pane(body_ui, anchor, accent_col),
                PANE_CONTAINERS => containers_pane(body_ui, anchor, accent_col),
                PANE_SCENE      => scene_pane(body_ui, anchor, accent_col),
                PANE_THEME      => theme_pane(
                    body_ui, anchor, accent_col,
                    &mut accent, &mut glass, &mut family, &mut mode,
                    &mut pastel, &mut tint,
                ),
                PANE_KEYS       => keys_pane(body_ui, anchor, accent_col),
                PANE_ABOUT      => about_pane(body_ui, anchor, accent_col),
                PANE_EDITOR     => editor_pane(body_ui, anchor, accent_col),
                _ => {}
            });
    }
}

// ─── Container-rendering helper ────────────────────────────────────

/// One container ready to render — its id, title, icon, and the pods
/// to drop in its body. Used by [`render_containers`].
struct ContainerSpec {
    id: egui::Id,
    title: String,
    icon: &'static str,
    pods: Vec<Pod>,
}

/// Render a vertical stack of containers inside a pane body, with
/// the inter-container three-dot drag handle painted between them
/// (and after the last). Drag delta updates each container's
/// persisted flow size via `set_container_flow`, with the orientation
/// auto-derived from the parent pane's rail (vertical-strip panes
/// stack containers horizontally → vertical handle; horizontal-strip
/// panes stack vertically → horizontal handle). Folded containers
/// ignore drag so the user can't silently grow / shrink an invisible
/// region.
fn render_containers(
    body_ui: &mut egui::Ui,
    pane_id: egui::Id,
    anchor: PaneAnchor,
    accent: egui::Color32,
    containers: Vec<ContainerSpec>,
) -> std::collections::HashMap<egui::Id, Vec<corekit::pod::PodResponse>> {
    let defaults: Vec<egui::Id> = containers.iter().map(|c| c.id).collect();
    let order = corekit::pane::section_order_for(body_ui.ctx(), pane_id, &defaults);
    let mut by_id: std::collections::HashMap<egui::Id, ContainerSpec> =
        containers.into_iter().map(|c| (c.id, c)).collect();

    let containers_stack_horizontally = !anchor.title_side().is_horizontal_strip();
    let dots_orient = if containers_stack_horizontally {
        corekit::container::SeparatorOrient::Vertical
    } else {
        corekit::container::SeparatorOrient::Horizontal
    };
    let title_at_end = anchor.title_side().is_at_end();
    let pane_horizontal_strip = anchor.title_side().is_horizontal_strip();

    let mut responses: std::collections::HashMap<
        egui::Id,
        Vec<corekit::pod::PodResponse>,
    > = std::collections::HashMap::new();
    for cid in order.into_iter() {
        let Some(spec) = by_id.remove(&cid) else { continue };
        let resp = Normal::new(spec.title.as_str(), anchor, accent, cid)
            .icon(spec.icon)
            .show(body_ui, spec.pods);
        responses.insert(cid, resp);

        // Skip painting the dot handle while THIS container is being
        // drag-reordered — the floating preview already paints a
        // copy with its handle, so painting the original's handle
        // here produces a "double dots" duplicate until release.
        let dragging_self = corekit::pane::active_drag(body_ui.ctx())
            .and_then(|(_, s)| s.item)
            .map(|item| item == cid)
            .unwrap_or(false);
        if dragging_self { continue; }

        let dot_resp =
            corekit::pane::paint_container_dots(body_ui, dots_orient, cid, accent);
        let body_open: bool = body_ui.ctx().data_mut(|d| {
            d.get_persisted::<bool>(cid.with("body_open")).unwrap_or(true)
        });
        if dot_resp.dragged() && body_open {
            let cur = corekit::container::container_flow(
                body_ui.ctx(),
                cid,
                pane_horizontal_strip,
            );
            let raw = if containers_stack_horizontally {
                dot_resp.drag_delta().x
            } else {
                dot_resp.drag_delta().y
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
    responses
}

// ─── Per-pane content ──────────────────────────────────────────────

fn cid(pane: &str, suffix: &str) -> egui::Id {
    egui::Id::new((pane, suffix))
}
fn pid(pane: &str, container: &str, idx: usize) -> egui::Id {
    egui::Id::new((pane, container, "pod", idx))
}

/// **Widgets pane** — one container per widget category.
fn widgets_pane(body: &mut egui::Ui, anchor: PaneAnchor, accent: egui::Color32) {
    let pane_id = egui::Id::new(PANE_WIDGETS);
    let anim = |name: &str, style: FillStyle, sep: SeparatorStyle, idx: usize| -> Pod {
        Pod::new(pid(PANE_WIDGETS, "anim", idx))
            .with_separator(sep)
            .with_button_animated(name, accent, style)
    };
    render_containers(body, pane_id, anchor, accent, vec![
        ContainerSpec {
            id: cid(PANE_WIDGETS, "flags"),
            title: "Flags".into(),
            icon: "flag",
            pods: vec![
                Pod::new(pid(PANE_WIDGETS, "flags", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_toggle_initial("power", accent, true),
                Pod::new(pid(PANE_WIDGETS, "flags", 1))
                    .with_separator(SeparatorStyle::None)
                    .with_toggle_initial("headlights", accent, false),
            ],
        },
        ContainerSpec {
            id: cid(PANE_WIDGETS, "numbers"),
            title: "Numbers".into(),
            icon: "calculator",
            pods: vec![
                Pod::new(pid(PANE_WIDGETS, "numbers", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("gravity", 9.81, 0.05, 0.0..=30.0, 2, " m/s²"),
                Pod::new(pid(PANE_WIDGETS, "numbers", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("speed limit", 60.0, 0.1, 0.0..=200.0, 1, " m/s"),
                Pod::new(pid(PANE_WIDGETS, "numbers", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_drag_value("engine power", 750.0, 1.0, 0.0..=2000.0, 0, " kW"),
            ],
        },
        ContainerSpec {
            id: cid(PANE_WIDGETS, "bars"),
            title: "Bars".into(),
            icon: "gauge",
            pods: vec![
                Pod::new(pid(PANE_WIDGETS, "bars", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_slider("throttle", 0.4, 0.0..=1.0, 2, "", accent),
                Pod::new(pid(PANE_WIDGETS, "bars", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_slider("brake", 0.0, 0.0..=1.0, 2, "", accent),
                Pod::new(pid(PANE_WIDGETS, "bars", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_progress("fuel", 0.62, "62%", accent),
            ],
        },
        ContainerSpec {
            id: cid(PANE_WIDGETS, "buttons"),
            title: "Buttons".into(),
            icon: "button",
            pods: vec![
                Pod::new(pid(PANE_WIDGETS, "buttons", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_button("Refuel", accent),
                Pod::new(pid(PANE_WIDGETS, "buttons", 1))
                    .with_separator(SeparatorStyle::None)
                    .with_card_button(
                        "star",
                        "Primary action",
                        "Two-line card button with glyph + subtitle",
                        accent,
                    ),
            ],
        },
        ContainerSpec {
            id: cid(PANE_WIDGETS, "anim"),
            title: "Animated".into(),
            icon: "animation",
            pods: vec![
                anim("Slide left",         FillStyle::SlideLeft,              SeparatorStyle::Line,  0),
                anim("Parallelogram",      FillStyle::Parallelogram,          SeparatorStyle::Line,  1),
                anim("Parallelogram meet", FillStyle::ParallelogramMeet,      SeparatorStyle::Line,  2),
                anim("Bowtie",             FillStyle::Bowtie,                 SeparatorStyle::Line,  3),
                anim("Bands meet",         FillStyle::BandsMeet,              SeparatorStyle::Line,  4),
                anim("Corner squares",     FillStyle::CornerSquares,          SeparatorStyle::Line,  5),
                anim("Diagonal triangles", FillStyle::DiagonalTriangles,      SeparatorStyle::Line,  6),
                anim("Circle grow",        FillStyle::CircleGrow,             SeparatorStyle::Line,  7),
                anim("Equalizer",          FillStyle::Equalizer,              SeparatorStyle::Line,  8),
                anim("Horizontal slide",   FillStyle::HorizontalSlide,        SeparatorStyle::Line,  9),
                anim("Horizontal delayed", FillStyle::HorizontalSlideDelayed, SeparatorStyle::Line, 10),
                anim("Vertical delayed",   FillStyle::VerticalSlideDelayed,   SeparatorStyle::Line, 11),
                anim("Criss cross",        FillStyle::CrissCross,             SeparatorStyle::None, 12),
            ],
        },
    ]);
}

/// **Containers pane** — Position + Rotation, axis-coloured drag values.
fn containers_pane(body: &mut egui::Ui, anchor: PaneAnchor, accent: egui::Color32) {
    let pane_id = egui::Id::new(PANE_CONTAINERS);
    render_containers(body, pane_id, anchor, accent, vec![
        ContainerSpec {
            id: cid(PANE_CONTAINERS, "pos"),
            title: "Position".into(),
            icon: "axis",
            pods: vec![
                Pod::new(pid(PANE_CONTAINERS, "pos", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("X", 0.0, 0.05, -1000.0..=1000.0, 3, " m"),
                Pod::new(pid(PANE_CONTAINERS, "pos", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("Y", 0.0, 0.05, -1000.0..=1000.0, 3, " m"),
                Pod::new(pid(PANE_CONTAINERS, "pos", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_drag_value("Z", 0.0, 0.05, -1000.0..=1000.0, 3, " m"),
            ],
        },
        ContainerSpec {
            id: cid(PANE_CONTAINERS, "rot"),
            title: "Rotation".into(),
            icon: "rotate",
            pods: vec![
                Pod::new(pid(PANE_CONTAINERS, "rot", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("X", 0.0, 1.0, -360.0..=360.0, 2, "°"),
                Pod::new(pid(PANE_CONTAINERS, "rot", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("Y", 0.0, 1.0, -360.0..=360.0, 2, "°"),
                Pod::new(pid(PANE_CONTAINERS, "rot", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_drag_value("Z", 0.0, 1.0, -360.0..=360.0, 2, "°"),
            ],
        },
    ]);
}

/// **Scene pane** — outliner tree + flat hybrid_select roster.
fn scene_pane(body: &mut egui::Ui, anchor: PaneAnchor, accent: egui::Color32) {
    let pane_id = egui::Id::new(PANE_SCENE);
    let tree_root = cid(PANE_SCENE, "tree_root");
    // Read the tree's currently-selected path so the trailing
    // readout shows it live (mirrors the legacy demo's `selected`
    // row at the bottom of the scene tree).
    let selected_path: String = body
        .ctx()
        .data(|d| {
            d.get_temp::<String>(tree_root.with("frost_demo_tree_selected"))
        })
        .unwrap_or_default();
    let selected_display = if selected_path.is_empty() {
        "—".to_string()
    } else {
        selected_path
    };

    let entities: Vec<String> = [
        "Planet", "Robot", "Sun", "Cloud Shell", "Camera",
        "Swatch[0]", "Swatch[1]", "Swatch[2]",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let trailing: Vec<String> = (0..entities.len()).map(|i| format!("#{i}")).collect();

    render_containers(body, pane_id, anchor, accent, vec![
        ContainerSpec {
            id: cid(PANE_SCENE, "scene"),
            title: "Scene".into(),
            icon: "folder",
            pods: vec![
                Pod::new(pid(PANE_SCENE, "scene", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_search("filter by name / path…", accent),
                Pod::new(pid(PANE_SCENE, "scene", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_dropdown(["all", "transforms", "lights", "meshes"], 0, accent),
                Pod::new(pid(PANE_SCENE, "scene", 2))
                    .with_separator(SeparatorStyle::Line)
                    .fill()
                    .with_custom_units(7, move |ui| demo_tree(ui, tree_root, accent)),
                Pod::new(pid(PANE_SCENE, "scene", 3))
                    .with_separator(SeparatorStyle::None)
                    .with_readout("selected", selected_display),
            ],
        },
        ContainerSpec {
            id: cid(PANE_SCENE, "flat"),
            title: "Flat list".into(),
            icon: "list",
            pods: vec![
                Pod::new(pid(PANE_SCENE, "flat", 0))
                    .with_separator(SeparatorStyle::LineDots)
                    .resizable()
                    .with_hybrid_select_list(entities, Some(trailing), accent),
            ],
        },
    ]);
}

/// **Theme pane** — Profile / Accent / Glass.
#[allow(clippy::too_many_arguments)]
fn theme_pane(
    body: &mut egui::Ui,
    anchor: PaneAnchor,
    accent: egui::Color32,
    accent_res: &mut AccentColor,
    glass: &mut GlassOpacity,
    family: &mut ThemeFamily,
    mode: &mut ThemeModeRes,
    pastel: &mut PastelToggle,
    tint: &mut TintRgba,
) {
    let pane_id = egui::Id::new(PANE_THEME);
    let profile_id = cid(PANE_THEME, "profile");
    let accent_id = cid(PANE_THEME, "accent");
    let glass_id = cid(PANE_THEME, "glass");
    let responses = render_containers(body, pane_id, anchor, accent, vec![
        ContainerSpec {
            id: profile_id,
            title: "Profile".into(),
            icon: "person",
            pods: vec![
                Pod::new(pid(PANE_THEME, "profile", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_dropdown(["PRO", "GAME"], family.0 as usize, accent),
                Pod::new(pid(PANE_THEME, "profile", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_dropdown(["Dark", "Light"], mode.0 as usize, accent),
                Pod::new(pid(PANE_THEME, "profile", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_toggle_initial("pastel accent", accent, pastel.0),
            ],
        },
        ContainerSpec {
            id: accent_id,
            title: "Accent".into(),
            icon: "color",
            pods: vec![
                Pod::new(pid(PANE_THEME, "accent", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_color_rgb(
                        "accent",
                        [
                            accent_res.0.r() as f32 / 255.0,
                            accent_res.0.g() as f32 / 255.0,
                            accent_res.0.b() as f32 / 255.0,
                        ],
                        accent,
                    ),
                Pod::new(pid(PANE_THEME, "accent", 1))
                    .with_separator(SeparatorStyle::None)
                    .with_color_rgba("tint", tint.0, accent),
            ],
        },
        ContainerSpec {
            id: glass_id,
            title: "Glass".into(),
            icon: "glasses",
            pods: vec![
                Pod::new(pid(PANE_THEME, "glass", 0))
                    .with_separator(SeparatorStyle::None)
                    .with_slider("opacity", glass.0 as f64, 1.0..=100.0, 0, "%", accent),
            ],
        },
    ]);
    // Wire response → mutable state.
    if let Some(pr) = responses.get(&profile_id) {
        if let Some(p0) = pr.first() {
            if let Some(d) = p0.dropdowns.first() {
                if d.changed { family.0 = d.selected as u8; }
            }
        }
        if let Some(p1) = pr.get(1) {
            if let Some(d) = p1.dropdowns.first() {
                if d.changed { mode.0 = d.selected as u8; }
            }
        }
        if let Some(p2) = pr.get(2) {
            if let Some(t) = p2.toggles.first() {
                if t.changed { pastel.0 = t.on; }
            }
        }
    }
    if let Some(pr) = responses.get(&accent_id) {
        if let Some(p0) = pr.first() {
            if let Some(c) = p0.colors.first() {
                if c.changed {
                    accent_res.0 = srgb_to_egui([c.rgba[0], c.rgba[1], c.rgba[2]]);
                }
            }
        }
        if let Some(p1) = pr.get(1) {
            if let Some(c) = p1.colors.first() {
                if c.changed { tint.0 = c.rgba; }
            }
        }
    }
    if let Some(pr) = responses.get(&glass_id) {
        if let Some(p0) = pr.first() {
            if let Some(s) = p0.sliders.first() {
                if s.changed {
                    glass.0 = s.value.round().clamp(1.0, 100.0) as u8;
                }
            }
        }
    }
}

/// **Keys pane** — keybinding readouts.
fn keys_pane(body: &mut egui::Ui, anchor: PaneAnchor, accent: egui::Color32) {
    let pane_id = egui::Id::new(PANE_KEYS);
    render_containers(body, pane_id, anchor, accent, vec![
        ContainerSpec {
            id: cid(PANE_KEYS, "mouse"),
            title: "Mouse".into(),
            icon: "cursor",
            pods: vec![
                Pod::new(pid(PANE_KEYS, "mouse", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_readout("MMB drag", "pan camera focus"),
                Pod::new(pid(PANE_KEYS, "mouse", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_readout("LMB+RMB drag", "orbit camera"),
                Pod::new(pid(PANE_KEYS, "mouse", 2))
                    .with_separator(SeparatorStyle::Line)
                    .with_readout("Scroll", "log-smooth zoom"),
                Pod::new(pid(PANE_KEYS, "mouse", 3))
                    .with_separator(SeparatorStyle::None)
                    .with_readout("LMB on cube", "re-tint UI accent"),
            ],
        },
        ContainerSpec {
            id: cid(PANE_KEYS, "layout"),
            title: "Layout".into(),
            icon: "grid",
            pods: vec![
                Pod::new(pid(PANE_KEYS, "layout", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_readout("Drag pane edge", "resize the pane"),
                Pod::new(pid(PANE_KEYS, "layout", 1))
                    .with_separator(SeparatorStyle::None)
                    .with_readout("Toggle ribbon btn", "open / close pane"),
            ],
        },
    ]);
}

/// **About pane** — version + dependency readouts.
fn about_pane(body: &mut egui::Ui, anchor: PaneAnchor, accent: egui::Color32) {
    let pane_id = egui::Id::new(PANE_ABOUT);
    render_containers(body, pane_id, anchor, accent, vec![
        ContainerSpec {
            id: cid(PANE_ABOUT, "info"),
            title: "bevy_frost".into(),
            icon: "info",
            pods: vec![
                Pod::new(pid(PANE_ABOUT, "info", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_readout("version", env!("CARGO_PKG_VERSION")),
                Pod::new(pid(PANE_ABOUT, "info", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_readout("bevy", "0.18"),
                Pod::new(pid(PANE_ABOUT, "info", 2))
                    .with_separator(SeparatorStyle::Line)
                    .with_readout("bevy_egui", "0.39"),
                Pod::new(pid(PANE_ABOUT, "info", 3))
                    .with_separator(SeparatorStyle::None)
                    .with_readout("egui", "0.33"),
            ],
        },
    ]);
}

/// **Editor pane** — node graph (top) + code editor (bottom),
/// each in its own container with a fill pod so they soak up the
/// pane's available space. Mirrors the legacy demo's Editor pane,
/// now driven by the vendored `bevy_frost::extras` wrappers.
fn editor_pane(body: &mut egui::Ui, anchor: PaneAnchor, accent: egui::Color32) {
    let pane_id = egui::Id::new(PANE_EDITOR);
    let graph_id = cid(PANE_EDITOR, "graph_state");
    let code_id = cid(PANE_EDITOR, "code_state");
    render_containers(body, pane_id, anchor, accent, vec![
        ContainerSpec {
            id: cid(PANE_EDITOR, "graph"),
            title: "Node graph".into(),
            icon: "flowchart",
            pods: vec![
                Pod::new(pid(PANE_EDITOR, "graph", 0))
                    .with_separator(SeparatorStyle::None)
                    .fill()
                    .with_custom_units(10, move |ui| {
                        let mut graph: Snarl<GraphNode> = ui
                            .ctx()
                            .data(|d| d.get_temp::<Snarl<GraphNode>>(graph_id))
                            .unwrap_or_else(default_graph);
                        let mut viewer = GraphViewer;
                        let avail = ui.available_size_before_wrap();
                        frost_snarl(
                            ui, graph_id, &mut graph, &mut viewer, accent, avail,
                        );
                        ui.ctx().data_mut(|d| d.insert_temp(graph_id, graph));
                    }),
            ],
        },
        ContainerSpec {
            id: cid(PANE_EDITOR, "code"),
            title: "Source".into(),
            icon: "code",
            pods: vec![
                Pod::new(pid(PANE_EDITOR, "code", 0))
                    .with_separator(SeparatorStyle::None)
                    .fill()
                    .with_custom_units(10, move |ui| {
                        let mut text: String = ui
                            .ctx()
                            .data(|d| d.get_temp::<String>(code_id))
                            .unwrap_or_else(|| DEFAULT_CODE.to_string());
                        let avail = ui.available_size_before_wrap();
                        frost_code_editor(
                            ui, code_id, &mut text, Syntax::rust(), accent, avail,
                        );
                        ui.ctx().data_mut(|d| d.insert_temp(code_id, text));
                    }),
            ],
        },
    ]);
}

// ─── Node-graph types (used by Editor pane) ────────────────────────

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
        .map(|r| eval_output(snarl, &snarl.out_pin(*r)))
        .sum()
}

#[derive(Default)]
struct GraphViewer;

impl SnarlViewer<GraphNode> for GraphViewer {
    fn title(&mut self, n: &GraphNode) -> String { n.title().into() }
    fn inputs(&mut self, n: &GraphNode) -> usize { n.inputs() }
    fn outputs(&mut self, n: &GraphNode) -> usize { n.outputs() }
    fn show_input(&mut self, pin: &InPin, ui: &mut egui::Ui, snarl: &mut Snarl<GraphNode>)
        -> impl SnarlPin + 'static
    {
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
    fn show_output(&mut self, pin: &OutPin, ui: &mut egui::Ui, snarl: &mut Snarl<GraphNode>)
        -> impl SnarlPin + 'static
    {
        if let Some(GraphNode::Number(v)) = snarl.get_node_mut(pin.id.node) {
            ui.add(egui::DragValue::new(v).speed(0.05).fixed_decimals(2));
        } else if let Some(GraphNode::Add) = snarl.get_node(pin.id.node) {
            let v = eval_output(snarl, pin);
            ui.label(format!("= {v:.3}"));
        }
        PinInfo::circle()
    }
    fn has_graph_menu(&mut self, _: egui::Pos2, _: &mut Snarl<GraphNode>) -> bool { true }
    fn show_graph_menu(&mut self, pos: egui::Pos2, ui: &mut egui::Ui, snarl: &mut Snarl<GraphNode>) {
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

// ─── Demo scene tree ───────────────────────────────────────────────

type DemoTreeRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
    egui::Color32,
);

const DEMO_TREE: &[DemoTreeRow] = &[
    ("/World", "World", "folder",
     &["/World/Robot", "/World/Lights"],
     egui::Color32::from_rgb(0x55, 0x6E, 0x9C)),
    ("/World/Robot", "Robot", "person",
     &["/World/Robot/base", "/World/Robot/arm"],
     egui::Color32::from_rgb(0xE0, 0x6C, 0x4F)),
    ("/World/Robot/base", "base", "code",
     &[], egui::Color32::from_rgb(0x4D, 0xA8, 0xDA)),
    ("/World/Robot/arm", "arm", "code",
     &["/World/Robot/arm/grip"],
     egui::Color32::from_rgb(0xE6, 0xB7, 0x3D)),
    ("/World/Robot/arm/grip", "grip", "code",
     &[], egui::Color32::from_rgb(0x9C, 0x55, 0xC0)),
    ("/World/Lights", "Lights", "image",
     &["/World/Lights/sun"],
     egui::Color32::from_rgb(0xF5, 0xC2, 0x42)),
    ("/World/Lights/sun", "sun", "image",
     &[], egui::Color32::from_rgb(0xFF, 0xE5, 0x6B)),
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
    let mut swatch_dummy = false;

    let mut slots = [
        TreeIconSlot::new(TreeIconKind::Eye, &mut eye_on)
            .with_tooltip("Toggle visibility"),
        TreeIconSlot::new(TreeIconKind::Lock, &mut lock_on)
            .with_tooltip("Toggle lock"),
        TreeIconSlot::new(TreeIconKind::Color(*material), &mut swatch_dummy)
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
