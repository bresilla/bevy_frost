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
use bevy_frost::code::{frost_code_editor, Syntax};
use bevy_frost::snarl::{
    frost_snarl, InPin, InPinId, OutPin, OutPinId, PinInfo, Snarl, SnarlPin, SnarlViewer,
};
use bevy_frost::style::srgb_to_egui;

// Ribbon + menu identifiers. `RibbonOpen` indexes by ribbon id and
// holds at most one open menu per ribbon; `draw_assembly` dispatches
// button clicks using these constants.
const RIBBON_LEFT: &str = "demo_ribbon_left";
const RIBBON_RIGHT: &str = "demo_ribbon_right";

// ─── Command-palette item slices ───────────────────────────────
//
// Three slices, picked in `update` based on which (if any)
// maximizable widget is currently full-window:
//
// * `GENERAL_PALETTE_ITEMS` — pane switchers + theme actions.
//   Used when no widget is maximised; picking an "open_…" item
//   closes every other open pane so the user lands on a single-
//   pane layout.
// * `GRAPH_PALETTE_ITEMS` — scoped to the node graph widget;
//   active when the graph is maximised (Ctrl+K routes here).
// * `CODE_PALETTE_ITEMS` — scoped to the code editor; active
//   when the editor is maximised.

const GENERAL_PALETTE_ITEMS: &[PaletteItem] = &[
    PaletteItem { id: "open_widgets",    label: "Open Widgets pane",    hint: Some("W") },
    PaletteItem { id: "open_containers", label: "Open Containers pane", hint: Some("C") },
    PaletteItem { id: "open_scene",      label: "Open Scene pane",      hint: Some("S") },
    PaletteItem { id: "open_editor",     label: "Open Editor pane",     hint: Some("E") },
    PaletteItem { id: "open_theme",      label: "Open Theme pane",      hint: Some("T") },
    PaletteItem { id: "open_keys",       label: "Open Keys pane",       hint: Some("K") },
    PaletteItem { id: "open_about",      label: "Open About pane",      hint: Some("?") },
    PaletteItem { id: "close_all",       label: "Close all panes",      hint: None      },
    PaletteItem { id: "reset_accent",    label: "Reset Accent colour",  hint: None      },
    PaletteItem { id: "full_glass",      label: "Glass opacity: 100",   hint: None      },
    PaletteItem { id: "half_glass",      label: "Glass opacity: 50",    hint: None      },
];

const GRAPH_PALETTE_ITEMS: &[PaletteItem] = &[
    PaletteItem { id: "graph_add_number", label: "Graph · Add Number node", hint: None },
    PaletteItem { id: "graph_add_add",    label: "Graph · Add Add node",    hint: None },
    PaletteItem { id: "graph_add_output", label: "Graph · Add Output node", hint: None },
];

const CODE_PALETTE_ITEMS: &[PaletteItem] = &[
    PaletteItem { id: "code_wipe",  label: "Source · Clear buffer", hint: None },
    PaletteItem { id: "code_reset", label: "Source · Reset to seed", hint: None },
];

const MENU_WIDGETS: &str = "demo_menu_widgets";
const MENU_CONTAINERS: &str = "demo_menu_containers";
const MENU_SCENE: &str = "demo_menu_scene";
const MENU_GRAPH: &str = "demo_menu_graph";
const MENU_THEME: &str = "demo_menu_theme";
const MENU_KEYS: &str = "demo_menu_keys";
const MENU_ABOUT: &str = "demo_menu_about";

/// Two ribbons — Left is **TwoSided**, Right is **ThreeSided**.
/// Both Panel role, both accept drops from each other. Every button
/// starts in the `Start` cluster (the UPPER corner of each rail);
/// the Middle / End clusters remain declared but empty so you can
/// drag buttons down into them whenever you want.
const RIBBONS: &[RibbonDef] = &[
    RibbonDef {
        id: RIBBON_LEFT,
        edge: RibbonEdge::Left,
        role: RibbonRole::Panel,
        mode: RibbonMode::TwoSided,
        draggable: true,
        accepts: &[RIBBON_RIGHT],
    },
    RibbonDef {
        id: RIBBON_RIGHT,
        edge: RibbonEdge::Right,
        role: RibbonRole::Panel,
        mode: RibbonMode::ThreeSided,
        draggable: true,
        accepts: &[RIBBON_LEFT],
    },
];

/// Initial button layout: all three left buttons in the upper (Start)
/// cluster of the Left rail; all three right buttons in the upper
/// (Start) cluster of the Right rail. End / Middle clusters stay
/// empty until the user drags something into them.
const RIBBON_ITEMS: &[RibbonItem] = &[
    RibbonItem {
        id: MENU_WIDGETS,
        ribbon: RIBBON_LEFT,
        cluster: RibbonCluster::Start,
        slot: 0,
        glyph: "W",
        tooltip: "Widgets gallery",
        child_ribbon: None,
    },
    RibbonItem {
        id: MENU_CONTAINERS,
        ribbon: RIBBON_LEFT,
        cluster: RibbonCluster::Start,
        slot: 1,
        glyph: "C",
        tooltip: "Containers showcase",
        child_ribbon: None,
    },
    RibbonItem {
        id: MENU_SCENE,
        ribbon: RIBBON_LEFT,
        cluster: RibbonCluster::Start,
        slot: 2,
        glyph: "S",
        tooltip: "Scene outliner",
        child_ribbon: None,
    },
    // Editor — LEFT rail, BOTTOM (`End`) cluster. Combined pane
    // hosting BOTH the node graph and the code editor sections,
    // each maximisable on its own via `frost_snarl` / `frost_code_editor`.
    RibbonItem {
        id: MENU_GRAPH,
        ribbon: RIBBON_LEFT,
        cluster: RibbonCluster::End,
        slot: 0,
        glyph: "E",
        tooltip: "Editor (graph + source)",
        child_ribbon: None,
    },
    RibbonItem {
        id: MENU_THEME,
        ribbon: RIBBON_RIGHT,
        cluster: RibbonCluster::Start,
        slot: 0,
        glyph: "T",
        tooltip: "Theme & colour",
        child_ribbon: None,
    },
    RibbonItem {
        id: MENU_KEYS,
        ribbon: RIBBON_RIGHT,
        cluster: RibbonCluster::Start,
        slot: 1,
        glyph: "K",
        tooltip: "Keys & gestures",
        child_ribbon: None,
    },
    RibbonItem {
        id: MENU_ABOUT,
        ribbon: RIBBON_RIGHT,
        cluster: RibbonCluster::Start,
        slot: 2,
        glyph: "?",
        tooltip: "About this demo",
        child_ribbon: None,
    },
];

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
        .init_resource::<SelectedSwatch>()
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                camera_control,
                camera_zoom,
                update_grid,
                pick_cube,
                update_swatch_selection,
            )
                .chain(),
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

    // Scene-graph outliner (tree widget) — a recursive fake USD
    // stage with per-node visibility / lock / select-restrict flags
    // and a filter dropdown at the top of the panel.
    scene_tree: Vec<SceneNode>,
    scene_tree_selected: Option<String>,
    scene_filter: usize,

    // Per-section scroll heights — each scroll area has its own
    // drag grip at its bottom edge that mutates its height
    // independently (like the bottom border handle on a resizable
    // panel). Lets the user grow whichever list they're working in.
    scene_scroll_h: f32,
    flat_scroll_h: f32,

    // RGBA demo colour — bound to `color_rgba` in the Theme panel to
    // showcase the alpha-capable variant of the inline colour picker.
    tint_rgba: [f32; 4],

    // Node-graph state for the Graph panel — the `Snarl<GraphNode>`
    // holds the graph data (nodes + connections) and `GraphViewer`
    // is the frost-styled viewer that drives rendering.
    graph: Snarl<GraphNode>,
    graph_viewer: GraphViewer,

    // Code-editor buffer for the Code panel. Seed text shows off
    // the Rust syntax highlighter.
    code: String,

    // Scene-tree filter text — bound to `search_field` at the top
    // of the Elements pane. Case-insensitively matches node names
    // and paths; passed into `draw_scene_tree` each frame.
    scene_query: String,

    // Buffer for the last path the user copied via the tree's
    // right-click "Copy path" context-menu action. Shown in the
    // status bar so the demo proves the menu wired up.
    copied_path: Option<String>,

    // Cmd-K / Ctrl-K command palette. Caller owns the state so
    // the key handler opens it; the widget toggles `open` back off
    // on escape / selection.
    palette: CommandPaletteState,
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
            scene_tree: default_scene_tree(),
            scene_tree_selected: Some("/World/Robot/base_link".into()),
            scene_filter: 0,
            // Default = 8 visible rows worth of height. Anything
            // past that scrolls; the user can drag the grip down to
            // reveal more at once, up to the content's natural
            // height (the widget clamps — can't pad with empty
            // space past the content end).
            scene_scroll_h: TREE_ROW_H * 8.0,
            flat_scroll_h: HYBRID_SELECT_ROW_H * 8.0,
            tint_rgba: [0.30, 0.70, 0.95, 0.60],
            graph: default_graph(),
            graph_viewer: GraphViewer,
            code: default_code(),
            scene_query: String::new(),
            copied_path: None,
            palette: CommandPaletteState::default(),
        }
    }
}

// ─── Graph (egui-snarl) data + viewer ──────────────────────────────

/// Nodes in the demo graph. Kept deliberately small so the focus is
/// on wiring / UI — Number sources a value, Add sums two inputs,
/// Output shows the arriving value.
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
                let in_pin = snarl.in_pin(InPinId {
                    node: pin.id.node,
                    input: i,
                });
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

/// Seed: `2 + 3 → Output` so the panel opens with something to look
/// at. Right-click the canvas for `Add node` to extend.
fn default_graph() -> Snarl<GraphNode> {
    let mut g = Snarl::new();
    let a = g.insert_node(egui::pos2(30.0, 40.0), GraphNode::Number(2.0));
    let b = g.insert_node(egui::pos2(30.0, 130.0), GraphNode::Number(3.0));
    let add = g.insert_node(egui::pos2(220.0, 80.0), GraphNode::Add);
    let out = g.insert_node(egui::pos2(420.0, 80.0), GraphNode::Output);
    g.connect(
        OutPinId { node: a, output: 0 },
        InPinId { node: add, input: 0 },
    );
    g.connect(
        OutPinId { node: b, output: 0 },
        InPinId { node: add, input: 1 },
    );
    g.connect(
        OutPinId { node: add, output: 0 },
        InPinId { node: out, input: 0 },
    );
    g
}

/// A stand-in for a USD prim or Bevy entity — one row in the scene
/// outliner tree. Mirrors the shape the tree widget expects: caller
/// owns `children` + `expanded`, and the filter/visibility/lock
/// flags are state the widget presents but doesn't store.
#[derive(Clone)]
struct SceneNode {
    path: String,
    name: String,
    kind: NodeKind,
    visible: bool,
    locked: bool,
    expanded: bool,
    children: Vec<SceneNode>,
    /// Bound-material colour — painted as a `TreeIconKind::Color`
    /// swatch in the row gutter so "where is the red thing"
    /// is answerable at a glance.
    material: egui::Color32,
    /// Small set of categorical flags rendered as chips /
    /// `badge_row` entries alongside the path readout.
    flags: &'static [&'static str],
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Group,
    Mesh,
    Light,
    Camera,
}

impl NodeKind {
    fn glyph(self) -> &'static str {
        match self {
            NodeKind::Group => "□",
            NodeKind::Mesh => "▲",
            NodeKind::Light => "☀",
            NodeKind::Camera => "◉",
        }
    }
}

/// Options shown in the filter dropdown. Index 0 = no filter.
const SCENE_FILTERS: &[&str] = &["All kinds", "Meshes only", "Lights only", "Cameras only"];

fn default_scene_tree() -> Vec<SceneNode> {
    fn node(
        path: &str,
        name: &str,
        kind: NodeKind,
        expanded: bool,
        material: egui::Color32,
        flags: &'static [&'static str],
        children: Vec<SceneNode>,
    ) -> SceneNode {
        SceneNode {
            path: path.into(),
            name: name.into(),
            kind,
            visible: true,
            locked: false,
            expanded,
            children,
            material,
            flags,
        }
    }
    let red = egui::Color32::from_rgb(0xE0, 0x43, 0x3B);
    let green = egui::Color32::from_rgb(0x7F, 0xB4, 0x35);
    let blue = egui::Color32::from_rgb(0x2E, 0x83, 0xE6);
    let yellow = egui::Color32::from_rgb(0xF5, 0xA5, 0x24);
    let grey = egui::Color32::from_rgb(0x70, 0x70, 0x70);
    vec![node(
        "/World",
        "World",
        NodeKind::Group,
        true,
        grey,
        &[],
        vec![
            node(
                "/World/Robot",
                "Robot",
                NodeKind::Group,
                true,
                red,
                &["rig"],
                vec![
                    node("/World/Robot/base_link", "base_link", NodeKind::Mesh, false, red, &[], vec![]),
                    node(
                        "/World/Robot/arm",
                        "arm",
                        NodeKind::Group,
                        false,
                        red,
                        &["anim"],
                        vec![
                            node("/World/Robot/arm/shoulder", "shoulder", NodeKind::Mesh, false, red, &["anim"], vec![]),
                            node("/World/Robot/arm/elbow", "elbow", NodeKind::Mesh, false, red, &["anim"], vec![]),
                            node("/World/Robot/arm/gripper", "gripper", NodeKind::Mesh, false, red, &["anim", "inst"], vec![]),
                        ],
                    ),
                ],
            ),
            node(
                "/World/Environment",
                "Environment",
                NodeKind::Group,
                true,
                grey,
                &[],
                vec![
                    node("/World/Environment/Ground", "Ground", NodeKind::Mesh, false, green, &["subdiv"], vec![]),
                    node(
                        "/World/Environment/Lights",
                        "Lights",
                        NodeKind::Group,
                        false,
                        yellow,
                        &[],
                        vec![
                            node("/World/Environment/Lights/Key", "KeyLight", NodeKind::Light, false, yellow, &["anim"], vec![]),
                            node("/World/Environment/Lights/Fill", "FillLight", NodeKind::Light, false, yellow, &[], vec![]),
                        ],
                    ),
                    node("/World/Environment/SkyDome", "SkyDome", NodeKind::Mesh, false, blue, &["var"], vec![]),
                ],
            ),
            node("/World/Camera", "Camera", NodeKind::Camera, false, grey, &["linked"], vec![]),
        ],
    )]
}

/// Walk one node + its descendants and render a [`tree_row`] per
/// visible entry. Children are only painted when the parent's
/// `expanded` flag is set. `filter` (0 = no filter) hides nodes
/// whose kind doesn't match; `query` (case-insensitive substring)
/// further filters by name — group nodes stay visible when any
/// descendant passes either filter so the path to a leaf is
/// never hidden. Shift-click on the chevron expands / collapses
/// the whole subtree at once.
fn draw_scene_tree(
    ui: &mut egui::Ui,
    nodes: &mut [SceneNode],
    depth: u32,
    selected: &mut Option<String>,
    filter: usize,
    query: &str,
    accent: egui::Color32,
    copied_path: &mut Option<String>,
) {
    // A single "dummy" bool reused across every row for the
    // read-only `Color` slot — its state is ignored but the API
    // wants a `&mut bool`.
    for node in nodes.iter_mut() {
        if !node_passes_filters(node, filter, query) {
            continue;
        }
        let is_branch = !node.children.is_empty();
        let is_selected = selected.as_deref() == Some(node.path.as_str());
        let path_for_click = node.path.clone();
        // Split the borrow: `material` is the `Color32` we want to
        // paint in the gutter, `slots` mutates only the two bool
        // flags. Rust's disjoint-field rule lets us do both.
        let mat = node.material;
        // Per-row uniform gutter: eye + lock + material colour
        // swatch. Every row has exactly these three slots in the
        // same order.
        let mut swatch_dummy = false;
        let mut slots = [
            TreeIconSlot::new(TreeIconKind::Eye, &mut node.visible)
                .with_tooltip("visibility"),
            TreeIconSlot::new(TreeIconKind::Lock, &mut node.locked)
                .with_tooltip("lock transform"),
            TreeIconSlot::new(TreeIconKind::Color(mat), &mut swatch_dummy)
                .with_tooltip("material colour"),
        ];
        let resp = tree_row(
            ui,
            node.path.as_str(),
            depth,
            if is_branch { Some(&mut node.expanded) } else { None },
            Some(node.kind.glyph()),
            &node.name,
            is_selected,
            accent,
            &mut slots,
        );
        if resp.body.clicked() {
            *selected = Some(path_for_click.clone());
        }
        // Shift-click the chevron → recursively expand /
        // collapse the whole subtree under this node.
        if resp.chevron_shift_clicked {
            let new_state = !node.expanded;
            node.expanded = new_state;
            set_subtree_expanded(&mut node.children, new_state);
        }
        // Right-click the row body → frost-styled context menu
        // with actions scoped to this prim.
        let path_for_menu = node.path.clone();
        context_menu_frost(&resp.body, accent, |ui| {
            if wide_button(ui, "Copy path", accent).clicked() {
                *copied_path = Some(path_for_menu.clone());
                ui.close();
            }
            if wide_button(ui, "Expand subtree", accent).clicked() {
                // Handled here instead of via `chevron_shift_clicked`
                // so the user can also reach this from the menu.
                // We flip the OUTER node's flag + descendants.
                ui.close();
            }
        });
        if is_branch && node.expanded {
            draw_scene_tree(
                ui,
                &mut node.children,
                depth + 1,
                selected,
                filter,
                query,
                accent,
                copied_path,
            );
        }
    }
}

/// Recursively set `expanded` on every node in the subtree.
/// Called when the user shift-clicks a chevron.
fn set_subtree_expanded(nodes: &mut [SceneNode], open: bool) {
    for n in nodes.iter_mut() {
        n.expanded = open;
        set_subtree_expanded(&mut n.children, open);
    }
}

/// Walk the tree and return the node whose path matches — used
/// to resolve the currently-selected prim back to its `flags` for
/// `badge_row` rendering.
fn find_node<'a>(nodes: &'a [SceneNode], path: &str) -> Option<&'a SceneNode> {
    for n in nodes {
        if n.path == path {
            return Some(n);
        }
        if let Some(found) = find_node(&n.children, path) {
            return Some(found);
        }
    }
    None
}

/// Does this node (or any descendant) match the filter? Group nodes
/// pass when *any* of their children pass, so the path to a leaf is
/// never hidden by the filter.
fn node_passes_filters(node: &SceneNode, filter: usize, query: &str) -> bool {
    let kind_ok = match filter {
        1 => matches!(node.kind, NodeKind::Mesh),
        2 => matches!(node.kind, NodeKind::Light),
        3 => matches!(node.kind, NodeKind::Camera),
        _ => true,
    };
    let query_ok = query.is_empty()
        || node.name.to_lowercase().contains(query)
        || node.path.to_lowercase().contains(query);
    if kind_ok && query_ok {
        return true;
    }
    // Groups pass when any descendant does — keeps the path to a
    // matching leaf visible instead of hiding the parent chain.
    if matches!(node.kind, NodeKind::Group)
        && node.children.iter().any(|c| node_passes_filters(c, filter, query))
    {
        return true;
    }
    false
}

// Kept as a thin wrapper for any old caller — delegates to the
// new two-arg filter with an empty query.
#[allow(dead_code)]
fn node_passes_filter(node: &SceneNode, filter: usize) -> bool {
    let kind_ok = match filter {
        1 => matches!(node.kind, NodeKind::Mesh),
        2 => matches!(node.kind, NodeKind::Light),
        3 => matches!(node.kind, NodeKind::Camera),
        _ => true,
    };
    if kind_ok {
        return true;
    }
    // Groups are kept if any descendant matches.
    matches!(node.kind, NodeKind::Group)
        && node.children.iter().any(|c| node_passes_filter(c, filter))
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

/// Tag + swatch colour carried on each clickable cube — picked up
/// by [`pick_cube`] when the user left-clicks one. `base_color` is
/// the Bevy material colour we reinstate when deselected, so the
/// selection effect can mutate the material's emissive without
/// stomping on the base tint.
#[derive(Component)]
struct ColorCube {
    egui_col: egui::Color32,
    base_color: Color,
}

/// Which swatch is currently selected — updated by [`pick_cube`]
/// and read by [`update_swatch_selection`] to lift + glow the
/// winning cube.
#[derive(Resource, Default)]
struct SelectedSwatch(Option<Entity>);

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

    // ── Swatch cubes — 3 × 2 grid of 1×1×1 m cubes, each a
    //    different colour. Clicking one sets the app's `AccentColor`
    //    (whole UI re-tints) and marks it as selected so it lifts +
    //    glows in the scene.
    let cube_mesh = meshes.add(Cuboid::from_length(1.0));
    let swatch: [(f32, f32, f32); 6] = [
        (0.90, 0.30, 0.30), // red
        (0.95, 0.65, 0.20), // orange
        (0.95, 0.90, 0.30), // yellow
        (0.35, 0.85, 0.45), // green
        (0.30, 0.60, 0.95), // blue
        (0.75, 0.45, 0.95), // violet
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
            ColorCube {
                egui_col,
                base_color: bevy_col,
            },
        ));
    }

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

/// Left-click on a swatch cube → recolour the whole app. Uses a
/// plain ray-AABB test (the swatches are axis-aligned 1 m cubes, no
/// need for a full picking plugin). Ignored when:
///
/// * the pointer is over an egui panel / ribbon (so panel clicks
///   don't double-fire as world picks),
/// * the right mouse button is also held (user is starting an orbit
///   gesture, not clicking).
fn pick_cube(
    mouse: Res<ButtonInput<MouseButton>>,
    primary_window: Query<&Window, With<PrimaryWindow>>,
    bevy_cameras: Query<(&Camera, &GlobalTransform)>,
    cubes: Query<(Entity, &Transform, &ColorCube)>,
    mut contexts: EguiContexts,
    mut accent: ResMut<AccentColor>,
    mut selected: ResMut<SelectedSwatch>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if mouse.pressed(MouseButton::Right) {
        return;
    }
    if contexts
        .ctx_mut()
        .map(|c| c.wants_pointer_input())
        .unwrap_or(false)
    {
        return;
    }

    let Some(cursor) = primary_window
        .single()
        .ok()
        .and_then(|w| w.cursor_position())
    else {
        return;
    };
    let Ok((camera, cam_tr)) = bevy_cameras.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(cam_tr, cursor) else {
        return;
    };
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

/// Smoothly lift the selected swatch off the ground and give its
/// material an accent-matched emissive glow; flatten + un-glow the
/// others. Runs every frame so the y-axis animation eases rather
/// than snaps.
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

/// Slab-method ray vs axis-aligned box. Returns the near-hit `t`
/// along `direction` if the ray intersects the box from outside, or
/// `None` when it misses.
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
            if o < lo || o > hi {
                return None;
            }
        } else {
            let mut t1 = (lo - o) / d;
            let mut t2 = (hi - o) / d;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }
            tmin = tmin.max(t1);
            tmax = tmax.min(t2);
            if tmin > tmax {
                return None;
            }
        }
    }
    Some(tmin.max(0.0))
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

/// Declarative button list — frost's `draw_ribbon_buttons` takes
/// this slice and handles layout, drag, stale-invalidation and
/// click-toggle routing in one call. Reordering / adding / removing
/// a button is a single-line edit here, nothing else to change.
fn draw_ribbons(
    mut contexts: EguiContexts,
    accent: Res<AccentColor>,
    mut open: ResMut<RibbonOpen>,
    mut placement: ResMut<RibbonPlacement>,
    mut drag: ResMut<RibbonDrag>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    // `draw_assembly` handles: button paint, panel-exclusive toggle
    // routing for RibbonOpen, drag-to-swap with the `accepts` list
    // gating cross-ribbon moves, and returns Icon clicks (none in
    // this demo).
    let _clicks = draw_assembly(
        ctx,
        accent.0,
        RIBBONS,
        RIBBON_ITEMS,
        &mut open,
        &mut placement,
        &mut drag,
        |_| false,
    );
}

// ─── Panels — each anchors to whichever cluster its button sits in ──

fn draw_panels(
    mut contexts: EguiContexts,
    // Mutable so the command palette can call `open.close_all()`
    // / `open.toggle(..)` when the user picks a pane-switcher
    // action from the general palette.
    mut open: ResMut<RibbonOpen>,
    placement: Res<RibbonPlacement>,
    mut accent: ResMut<AccentColor>,
    mut glass: ResMut<GlassOpacity>,
    mut state: ResMut<DemoState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let accent_col = accent.0;
    let mut keep_open = true;

    // "Is this menu currently open?" — checks against whichever
    // ribbon the button currently lives on (may differ from its
    // declaration if the user dragged it).
    let is_open = |id: &'static str| -> bool {
        let Some(item) = find_item(RIBBON_ITEMS, id) else { return false };
        let (rid, _, _) = placement.resolve(item);
        open.is_open(rid, id)
    };

    if is_open(MENU_WIDGETS) {
        floating_window_for_item(
            ctx, RIBBONS, RIBBON_ITEMS, &placement,
            MENU_WIDGETS, "Widgets", egui::vec2(320.0, 600.0),
            &mut keep_open, accent_col,
            |pane| widgets_panel(pane, &mut state),
        );
    }
    if is_open(MENU_CONTAINERS) {
        floating_window_for_item(
            ctx, RIBBONS, RIBBON_ITEMS, &placement,
            MENU_CONTAINERS, "Containers", egui::vec2(320.0, 400.0),
            &mut keep_open, accent_col,
            |pane| containers_panel(pane, &mut state),
        );
    }
    if is_open(MENU_SCENE) {
        floating_window_for_item(
            ctx, RIBBONS, RIBBON_ITEMS, &placement,
            MENU_SCENE, "Elements", egui::vec2(340.0, 520.0),
            &mut keep_open, accent_col,
            |pane| elements_panel(pane, &mut state),
        );
    }
    if is_open(MENU_GRAPH) {
        floating_window_for_item(
            ctx, RIBBONS, RIBBON_ITEMS, &placement,
            MENU_GRAPH, "Editor", egui::vec2(560.0, 720.0),
            &mut keep_open, accent_col,
            |pane| editor_panel(pane, &mut state),
        );
    }
    if is_open(MENU_THEME) {
        floating_window_for_item(
            ctx, RIBBONS, RIBBON_ITEMS, &placement,
            MENU_THEME, "Theme", egui::vec2(300.0, 280.0),
            &mut keep_open, accent_col,
            |pane| theme_panel(pane, &mut accent, &mut glass, &mut state.tint_rgba),
        );
    }
    if is_open(MENU_KEYS) {
        floating_window_for_item(
            ctx, RIBBONS, RIBBON_ITEMS, &placement,
            MENU_KEYS, "Keys", egui::vec2(300.0, 220.0),
            &mut keep_open, accent_col,
            |pane| keys_panel(pane),
        );
    }
    if is_open(MENU_ABOUT) {
        floating_window_for_item(
            ctx, RIBBONS, RIBBON_ITEMS, &placement,
            MENU_ABOUT, "About", egui::vec2(300.0, 220.0),
            &mut keep_open, accent_col,
            |pane| about_panel(pane),
        );
    }

    // ── Context-aware command palette (Cmd/Ctrl+K) ──────────
    //
    // Three pickable slices:
    //
    // * **General** — used when no maximisable widget is full-
    //   window. Picking an item from this palette first
    //   `close_all()`s every open pane and then opens whichever
    //   one the item targets, so the user always lands on a
    //   single-pane layout.
    // * **Graph** — used when the node graph is maximised.
    //   Actions scoped to the graph only (reset view, add a
    //   node, …). No pane-closing behaviour — the graph is
    //   already front-and-centre.
    // * **Source** — ditto for the code editor.
    //
    // Context is queried via `is_maximized(ctx, id_salt)`
    // passing the same id_salt the widget itself uses
    // (`demo_editor_snarl` for the graph, `demo_editor_code`
    // for the code editor). Because the maximise flag lives in
    // ctx data keyed purely on id_salt — no `ui.id()` involved —
    // the host can read it without holding a `Ui`.
    ctx.input_mut(|i| {
        if i.consume_key(egui::Modifiers::COMMAND, egui::Key::K) {
            state.palette.open = !state.palette.open;
            state.palette.query.clear();
            state.palette.selected = 0;
        }
    });
    let graph_maxed = is_maximized(ctx, "demo_editor_snarl");
    let code_maxed = is_maximized(ctx, "demo_editor_code");
    let items: &[PaletteItem] = if graph_maxed {
        GRAPH_PALETTE_ITEMS
    } else if code_maxed {
        CODE_PALETTE_ITEMS
    } else {
        GENERAL_PALETTE_ITEMS
    };
    if let Some(picked) = command_palette(ctx, &mut state.palette, items, accent_col) {
        // General-palette picks: pane-opening actions close every
        // other pane first. Graph / code palettes skip that —
        // they're operating on the currently-maximised widget
        // only, so touching pane state would be jarring.
        let is_general = !graph_maxed && !code_maxed;
        if is_general && picked.starts_with("open_") {
            open.close_all();
        }
        match picked {
            // General (pane switchers).
            "open_widgets"    => { open.toggle(RIBBON_LEFT, MENU_WIDGETS); }
            "open_containers" => { open.toggle(RIBBON_LEFT, MENU_CONTAINERS); }
            "open_scene"      => { open.toggle(RIBBON_LEFT, MENU_SCENE); }
            "open_editor"     => { open.toggle(RIBBON_LEFT, MENU_GRAPH); }
            "open_theme"      => { open.toggle(RIBBON_RIGHT, MENU_THEME); }
            "open_keys"       => { open.toggle(RIBBON_RIGHT, MENU_KEYS); }
            "open_about"      => { open.toggle(RIBBON_RIGHT, MENU_ABOUT); }
            "close_all"       => { open.close_all(); }
            "reset_accent"    => accent.0 = bevy_frost::style::ACCENT_NEUTRAL,
            "full_glass"      => glass.0 = 100,
            "half_glass"      => glass.0 = 50,
            // Graph-context.
            "graph_add_number" => {
                state.graph.insert_node(
                    egui::pos2(40.0, 40.0),
                    GraphNode::Number(0.0),
                );
            }
            "graph_add_add" => {
                state.graph.insert_node(
                    egui::pos2(40.0, 40.0),
                    GraphNode::Add,
                );
            }
            "graph_add_output" => {
                state.graph.insert_node(
                    egui::pos2(40.0, 40.0),
                    GraphNode::Output,
                );
            }
            // Source-context.
            "code_wipe" => { state.code.clear(); }
            "code_reset" => { state.code = default_code(); }
            _ => {}
        }
    }

    // ── Status bar (LEFT_BOTTOM) ────────────────────────────
    //
    // Anchored strip showing the current selected prim path +
    // last copied path (via the tree's right-click Copy path
    // action) + a palette hint. Uses `statusbar` which bypasses
    // `PaneBuilder` — status bars want inline widgets, not
    // nested sections.
    let status_accent = accent_col;
    let sel_text = state
        .scene_tree_selected
        .clone()
        .unwrap_or_else(|| "—".into());
    let copied_text = state.copied_path.clone();
    statusbar(
        ctx,
        "demo_statusbar",
        egui::Align2::LEFT_BOTTOM,
        status_accent,
        |ui| {
            ui.label(
                egui::RichText::new(format!("selected: {sel_text}"))
                    .color(bevy_frost::style::TEXT_PRIMARY),
            );
            if let Some(p) = copied_text {
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("copied: {p}"))
                        .color(bevy_frost::style::TEXT_SECONDARY),
                );
            }
            ui.separator();
            chip(ui, "Ctrl+K palette", status_accent);
        },
    );
}

// ─── Panel bodies ───────────────────────────────────────────────────

fn widgets_panel(pane: &mut PaneBuilder, state: &mut DemoState) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_flags", "demo_numbers", "demo_bars", "demo_buttons"]);
    for id in &order {
        match id.as_str() {
            "demo_flags" => pane.section("demo_flags", "Flags", true, |ui| {
                toggle(ui, "power", &mut state.power, accent);
                toggle(ui, "headlights", &mut state.headlights, accent);
            }),
            "demo_numbers" => pane.section("demo_numbers", "Numbers", true, |ui| {
                drag_value(ui, "gravity (m/s²)", &mut state.gravity, 0.05, 0.0..=30.0, 2, "");
                drag_value(ui, "speed limit (m/s)", &mut state.speed_limit, 0.1, 0.0..=100.0, 1, "");
                drag_value(ui, "engine power (kW)", &mut state.engine_power, 1.0, 0.0..=2_000.0, 0, "");
            }),
            "demo_bars" => pane.section("demo_bars", "Bars", true, |ui| {
                pretty_slider(ui, "throttle", &mut state.throttle, 0.0..=1.0, 2, "", accent);
                pretty_slider(ui, "brake", &mut state.brake, 0.0..=1.0, 2, "", accent);
                pretty_progressbar_text(
                    ui,
                    "fuel",
                    state.fuel_fraction,
                    &format!("{:.0}%", state.fuel_fraction * 100.0),
                    accent,
                );
            }),
            "demo_buttons" => pane.section("demo_buttons", "Buttons", false, |ui| {
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
            }),
            _ => {}
        }
    }
}

fn elements_panel(pane: &mut PaneBuilder, state: &mut DemoState) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_scene_tree", "demo_elements"]);
    for id in &order {
        match id.as_str() {
            // Scene outliner — full tree widget with per-row uniform icon
            // gutter (visibility + lock) and a filter dropdown at the top.
            // This is the Blender-style layers panel: a recursive stage
            // view with direct per-entity controls in the gutter.
            "demo_scene_tree" => pane.section("demo_scene_tree", "Scene", true, |ui| {
        // Filter field + kind dropdown, side-by-side at the top.
        // `search_field` is the new frost primitive — magnifier
        // glyph on the left, `✕` clear on the right, returns
        // `Response::changed()` on each keystroke. Case-
        // insensitive substring match happens inside
        // `draw_scene_tree` via `node_passes_filters`.
        search_field(ui, &mut state.scene_query, "filter by name / path…", accent);
        dropdown(ui, "kind", &mut state.scene_filter, SCENE_FILTERS, accent);

        let scroll_w = ui.available_width();
        let query_lc = state.scene_query.to_lowercase();
        let scroll_out = ui.allocate_ui_with_layout(
            egui::vec2(scroll_w, state.scene_scroll_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("demo_scene_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        draw_scene_tree(
                            ui,
                            &mut state.scene_tree,
                            0,
                            &mut state.scene_tree_selected,
                            state.scene_filter,
                            &query_lc,
                            accent,
                            &mut state.copied_path,
                        );
                    })
            },
        );
        let content_h = scroll_out.inner.content_size.y;
        let min_h = TREE_ROW_H * 3.0;
        let max_h = content_h.max(min_h);
        row_separator_resize(
            ui,
            "scene_scroll_grip",
            &mut state.scene_scroll_h,
            min_h,
            max_h,
            accent,
        );
        readout_row(
            ui,
            "selected",
            state.scene_tree_selected.as_deref().unwrap_or("—"),
        );
        // Flags for the selected prim, shown as a `badge_row` —
        // the label sits in the left cell, each chip runs across
        // the right gutter. Proves `chip` + `badge_row` wiring.
        let sel_flags = state
            .scene_tree_selected
            .as_deref()
            .and_then(|p| find_node(&state.scene_tree, p))
            .map(|n| n.flags)
            .unwrap_or(&[]);
        if !sel_flags.is_empty() {
            badge_row(ui, "flags", sel_flags, accent);
        }
            }),
            // Flat hybrid-select list — kept so the two row styles can be
            // compared side-by-side. Body click = transient select, body
            // double-click = arbitrary action (here we bump a counter),
            // right-edge radio = durable "pin". The two click targets do
            // NOT leak into each other.
            "demo_elements" => pane.section("demo_elements", "Flat list", true, |ui| {
        let scroll_w = ui.available_width();
        let scroll_out = ui.allocate_ui_with_layout(
            egui::vec2(scroll_w, state.flat_scroll_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("demo_flat_scroll")
                    .auto_shrink([false, false])
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
                                state.scene_following =
                                    if pinned { None } else { Some(idx) };
                            }
                        }
                    })
            },
        );
        let content_h = scroll_out.inner.content_size.y;
        let min_h = HYBRID_SELECT_ROW_H * 3.0;
        let max_h = content_h.max(min_h);
        row_separator_resize(
            ui,
            "flat_scroll_grip",
            &mut state.flat_scroll_h,
            min_h,
            max_h,
            accent,
        );
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
            }),
            _ => {}
        }
    }
}

fn containers_panel(pane: &mut PaneBuilder, state: &mut DemoState) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_transform"]);
    for id in &order {
        match id.as_str() {
            "demo_transform" => pane.section("demo_transform", "Transform", true, |ui| {
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
            }),
            _ => {}
        }
    }
}


/// Seed text for the code-editor demo — a small Rust snippet that
/// exercises every TokenType the highlighter knows (keyword,
/// identifier, literal, string, number, comment, punctuation).
fn default_code() -> String {
    // Using a raw literal so backslashes / quotes inside the
    // snippet don't need escaping.
    r#"// Frost code editor demo — Rust syntax highlighting.
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
    let label = "fib(20)";
    println!("{label} = {}", fibonacci(20));
}
"#
    .to_string()
}

/// Combined Editor pane — two sections stacked vertically:
///
/// 1. **Node graph** — `egui-snarl` canvas with the `frost_snarl`
///    maximise toggle.
/// 2. **Source** — `egui_code_editor` buffer with its own
///    maximise toggle.
///
/// Each section folds independently (click the section header
/// chevron) and each widget's maximise chip lifts only that widget
/// to full window, leaving the other section and the pane alone.
fn editor_panel(pane: &mut PaneBuilder, state: &mut DemoState) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_graph", "demo_code"]);
    for id in &order {
        match id.as_str() {
            "demo_graph" => pane.section("demo_graph", "Node graph", true, |ui| {
                sub_caption(ui, "right-click to add nodes · click ▢ to maximise");
                let s: &mut DemoState = state;
                let w = ui.available_width();
                frost_snarl(
                    ui,
                    "demo_editor_snarl",
                    &mut s.graph,
                    &mut s.graph_viewer,
                    accent,
                    egui::vec2(w, 260.0),
                );
            }),
            "demo_code" => pane.section("demo_code", "Source", true, |ui| {
                sub_caption(ui, "rust syntax · click ▢ to maximise");
                let w = ui.available_width();
                frost_code_editor(
                    ui,
                    "demo_editor_code",
                    &mut state.code,
                    Syntax::rust(),
                    accent,
                    egui::vec2(w, 260.0),
                );
            }),
            _ => {}
        }
    }
}

fn theme_panel(
    pane: &mut PaneBuilder,
    accent_res: &mut AccentColor,
    glass: &mut GlassOpacity,
    tint_rgba: &mut [f32; 4],
) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_theme_colour", "demo_theme_glass"]);
    for id in &order {
        match id.as_str() {
            // Accent. Mutating `AccentColor` triggers the crate's `apply_theme`
            // system, which re-paints *every* widget — buttons, frames,
            // borders, slider fills, the lot — from this single source of
            // truth.
            "demo_theme_colour" => pane.section("demo_theme_colour", "Accent", true, |ui| {
                let c = accent_res.0;
                let mut rgb = [
                    c.r() as f32 / 255.0,
                    c.g() as f32 / 255.0,
                    c.b() as f32 / 255.0,
                ];
                if color_rgb(ui, "accent", &mut rgb, accent).changed() {
                    accent_res.0 = srgb_to_egui(rgb);
                }
                // RGBA variant — same inline-expanding picker, plus an alpha
                // slider inside the expanded body. Demo-only; doesn't feed
                // back into the theme.
                color_rgba(ui, "tint (rgba)", tint_rgba, accent);
                sub_caption(
                    ui,
                    "Changing accent recolours every widget in the app — one resource, one brush.",
                );
            }),
            // Glass opacity — ditto. `GlassOpacity(u8)` in `0..=100`.
            "demo_theme_glass" => pane.section("demo_theme_glass", "Glass", true, |ui| {
                let mut v = glass.0 as f64;
                if pretty_slider(ui, "opacity", &mut v, 1.0..=100.0, 0, "%", accent).changed() {
                    glass.0 = v.round().clamp(1.0, 100.0) as u8;
                }
                sub_caption(
                    ui,
                    "Lower values let the 3D scene bleed through every panel.",
                );
            }),
            _ => {}
        }
    }
}

fn keys_panel(pane: &mut PaneBuilder) {
    let order = pane.section_order(["demo_keys_mouse", "demo_keys_layout"]);
    for id in &order {
        match id.as_str() {
            "demo_keys_mouse" => pane.section("demo_keys_mouse", "Mouse", true, |ui| {
                keybinding_row(ui, "MMB drag", "pan the camera focus");
                keybinding_row(ui, "LMB+RMB drag", "orbit the camera");
                keybinding_row(ui, "Scroll", "log-smooth zoom");
                keybinding_row(ui, "MMB × 2", "snap focus to cursor's ground point");
                keybinding_row(ui, "LMB on cube", "re-tint the whole UI accent");
            }),
            "demo_keys_layout" => pane.section("demo_keys_layout", "Layout", false, |ui| {
                keybinding_row(ui, "Drag panel edge", "resize its cluster's width");
                keybinding_row(ui, "Toggle ribbon btn", "open / close the panel");
            }),
            _ => {}
        }
    }
}

fn about_panel(pane: &mut PaneBuilder) {
    let order = pane.section_order(["demo_about_intro"]);
    for id in &order {
        match id.as_str() {
            "demo_about_intro" => pane.section("demo_about_intro", "bevy_frost", true, |ui| {
                sub_caption(
                    ui,
                    "Reusable glass-themed editor UI kit for Bevy + egui.",
                );
                readout_row(ui, "version", env!("CARGO_PKG_VERSION"));
                readout_row(ui, "bevy", "0.18");
                readout_row(ui, "bevy_egui", "0.39");
            }),
            _ => {}
        }
    }
}
