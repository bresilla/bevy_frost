//! Plain-egui showcase for `egui_frost` — an `eframe` app that
//! mirrors `bevy_frost`'s `demo` example without any Bevy
//! dependency. Every pane, widget, interaction, and keyboard
//! shortcut is wired the same way — the only differences are:
//!
//! * No 3D viewport (eframe + wgpu is just drawing egui; there's
//!   no Bevy scene to render behind the panels).
//! * State lives in an `impl eframe::App` struct instead of Bevy
//!   `Resource`s.
//! * `frostcore::apply_theme` is called manually from `update`
//!   (Bevy's `ThemePlugin` did this transparently).
//!
//! Run with:
//! ```text
//! cargo run -p egui_frost --example simple
//! ```
//!
//! Features exercised: floating panes + ribbon, the full widget
//! gallery (toggles, drag values, sliders, progress bars,
//! colour pickers, buttons, trees, dropdowns, chips, badge rows,
//! search fields, context menus), the node graph (`frost_snarl`),
//! the code editor (`frost_code_editor`), the command palette
//! (Ctrl+K), and the status bar — every one reads from
//! `egui_frost` (which is itself just a re-export of `frostcore`).

use eframe::egui;

use egui_frost::code::{frost_code_editor, Syntax};
use egui_frost::prelude::*;
use egui_frost::snarl::{
    frost_snarl, InPin, InPinId, OutPin, OutPinId, PinInfo, Snarl, SnarlPin, SnarlViewer,
};
use egui_frost::style::srgb_to_egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("egui_frost demo"),
        ..Default::default()
    };
    eframe::run_native(
        "egui_frost demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(FrostApp::default()))),
    )
}

// ─── Command-palette item slices ───────────────────────────────
//
// Context picks one of these based on `is_maximized`. Identical
// in shape to `bevy_frost`'s demo — the only difference is this
// side uses `egui_frost::...` imports instead of `bevy_frost::...`.

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

// ─── Ribbon + menu ids ─────────────────────────────────────────────

const RIBBON_LEFT: &str = "demo_ribbon_left";
const RIBBON_RIGHT: &str = "demo_ribbon_right";

const MENU_WIDGETS: &str = "demo_menu_widgets";
const MENU_CONTAINERS: &str = "demo_menu_containers";
const MENU_SCENE: &str = "demo_menu_scene";
const MENU_GRAPH: &str = "demo_menu_graph";
const MENU_ICONS: &str = "demo_menu_icons";
const MENU_THEME: &str = "demo_menu_theme";
const MENU_KEYS: &str = "demo_menu_keys";
const MENU_ABOUT: &str = "demo_menu_about";

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
        id: MENU_ICONS,
        ribbon: RIBBON_LEFT,
        cluster: RibbonCluster::End,
        slot: 1,
        glyph: "I",
        tooltip: "Fluent UI icon grid",
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

// ─── App state ─────────────────────────────────────────────────────

struct FrostApp {
    // Theme values. In a Bevy app these would be `Resource`s; here
    // they're just fields.
    accent: AccentColor,
    glass: GlassOpacity,

    // Widgets panel values.
    power: bool,
    headlights: bool,
    gravity: f64,
    speed_limit: f64,
    engine_power: f64,
    throttle: f64,
    brake: f64,
    fuel: f32,
    tint_rgba: [f32; 4],

    // Containers panel values.
    position: [f64; 3],
    rotation_deg: [f64; 3],

    // Flat list state — hybrid_select demo.
    scene_entities: Vec<String>,
    scene_selected: Option<usize>,
    scene_following: Option<usize>,
    scene_double_click_count: u32,

    // Scene-graph outliner + filters.
    scene_tree: Vec<SceneNode>,
    scene_tree_selected: Option<String>,
    scene_filter: usize,
    scene_query: String,
    copied_path: Option<String>,

    // Scroll heights for resizable scroll areas in the Elements pane.
    scene_scroll_h: f32,
    flat_scroll_h: f32,

    // Graph + code editor state.
    graph: Snarl<GraphNode>,
    graph_viewer: GraphViewer,
    code: String,

    // Ribbon state — identical types to the Bevy version. Plain
    // structs here because we don't enable the `bevy` feature.
    open: RibbonOpen,
    placement: RibbonPlacement,
    drag: RibbonDrag,

    // Command palette (Ctrl+K).
    palette: CommandPaletteState,
    last_picked: Option<&'static str>,
}

impl Default for FrostApp {
    fn default() -> Self {
        Self {
            accent: AccentColor::default(),
            glass: GlassOpacity::default(),
            power: true,
            headlights: false,
            gravity: 9.81,
            speed_limit: 24.0,
            engine_power: 180.0,
            throttle: 0.35,
            brake: 0.0,
            fuel: 0.72,
            tint_rgba: [0.30, 0.70, 0.95, 0.60],
            position: [0.0, 1.5, 0.0],
            rotation_deg: [0.0, 45.0, 0.0],
            scene_entities: vec![
                "Planet".into(),
                "CloudShell".into(),
                "Sun".into(),
                "Grid[L0]".into(),
                "Grid[L1]".into(),
                "Grid[L2]".into(),
                "OriginCube".into(),
            ],
            scene_selected: Some(0),
            scene_following: None,
            scene_double_click_count: 0,
            scene_tree: default_scene_tree(),
            scene_tree_selected: Some("/World/Robot/base_link".into()),
            scene_filter: 0,
            scene_query: String::new(),
            copied_path: None,
            scene_scroll_h: TREE_ROW_H * 8.0,
            flat_scroll_h: HYBRID_SELECT_ROW_H * 8.0,
            graph: default_graph(),
            graph_viewer: GraphViewer,
            code: default_code(),
            open: RibbonOpen::default(),
            placement: RibbonPlacement::default(),
            drag: RibbonDrag::default(),
            palette: CommandPaletteState::default(),
            last_picked: None,
        }
    }
}

impl eframe::App for FrostApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Theme must be re-applied every frame for accent / opacity
        // changes to take effect. The function de-dupes internally,
        // so stable values cost nothing.
        apply_theme(ctx, self.accent, self.glass);
        set_glass_opacity(self.glass.0);

        // Central "scene" stand-in — the Bevy demo renders a 3D
        // scene here; we just paint the window fill and a title so
        // the floating panes have a backdrop.
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(0x1A, 0x1A, 0x1C)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.heading("egui_frost");
                    ui.label("Plain egui (no Bevy) · every ribbon button opens a pane");
                    ui.add_space(8.0);
                    ui.label("Ctrl+K  opens the command palette");
                });
            });

        let accent_col = self.accent.0;
        // Destructure so each closure below only borrows the state
        // it actually touches — the placement ref + panel-body
        // closures would conflict through `self` otherwise.
        let FrostApp {
            accent,
            glass,
            power,
            headlights,
            gravity,
            speed_limit,
            engine_power,
            throttle,
            brake,
            fuel,
            tint_rgba,
            position,
            rotation_deg,
            scene_entities,
            scene_selected,
            scene_following,
            scene_double_click_count,
            scene_tree,
            scene_tree_selected,
            scene_filter,
            scene_query,
            copied_path,
            scene_scroll_h,
            flat_scroll_h,
            graph,
            graph_viewer,
            code,
            open,
            placement,
            drag,
            palette,
            last_picked,
        } = self;

        let _clicks = draw_assembly(
            ctx,
            accent_col,
            RIBBONS,
            RIBBON_ITEMS,
            open,
            placement,
            drag,
            |_| false,
        );

        let is_open = |id: &'static str| -> bool {
            let Some(item) = find_item(RIBBON_ITEMS, id) else {
                return false;
            };
            let (rid, _, _) = placement.resolve(item);
            open.is_open(rid, id)
        };

        let mut keep_open = true;

        if is_open(MENU_WIDGETS) {
            floating_window_for_item(
                ctx, RIBBONS, RIBBON_ITEMS, placement,
                MENU_WIDGETS, "Widgets", egui::vec2(320.0, 600.0),
                &mut keep_open, accent_col,
                |pane| widgets_panel(
                    pane, power, headlights, gravity, speed_limit, engine_power,
                    throttle, brake, fuel,
                ),
            );
        }
        if is_open(MENU_CONTAINERS) {
            floating_window_for_item(
                ctx, RIBBONS, RIBBON_ITEMS, placement,
                MENU_CONTAINERS, "Containers", egui::vec2(320.0, 400.0),
                &mut keep_open, accent_col,
                |pane| containers_panel(pane, position, rotation_deg),
            );
        }
        if is_open(MENU_SCENE) {
            floating_window_for_item(
                ctx, RIBBONS, RIBBON_ITEMS, placement,
                MENU_SCENE, "Elements", egui::vec2(340.0, 560.0),
                &mut keep_open, accent_col,
                |pane| elements_panel(
                    pane,
                    scene_tree, scene_tree_selected, scene_filter, scene_query,
                    copied_path, scene_scroll_h, flat_scroll_h,
                    scene_entities, scene_selected, scene_following,
                    scene_double_click_count,
                ),
            );
        }
        if is_open(MENU_GRAPH) {
            floating_window_for_item(
                ctx, RIBBONS, RIBBON_ITEMS, placement,
                MENU_GRAPH, "Editor", egui::vec2(560.0, 720.0),
                &mut keep_open, accent_col,
                |pane| editor_panel(pane, graph, graph_viewer, code),
            );
        }
        if is_open(MENU_ICONS) {
            floating_window_for_item(
                ctx, RIBBONS, RIBBON_ITEMS, placement,
                MENU_ICONS, "Icons", egui::vec2(420.0, 560.0),
                &mut keep_open, accent_col,
                |pane| icons_panel(pane),
            );
        }
        if is_open(MENU_THEME) {
            floating_window_for_item(
                ctx, RIBBONS, RIBBON_ITEMS, placement,
                MENU_THEME, "Theme", egui::vec2(300.0, 280.0),
                &mut keep_open, accent_col,
                |pane| theme_panel(pane, accent, glass, tint_rgba),
            );
        }
        if is_open(MENU_KEYS) {
            floating_window_for_item(
                ctx, RIBBONS, RIBBON_ITEMS, placement,
                MENU_KEYS, "Keys", egui::vec2(300.0, 220.0),
                &mut keep_open, accent_col,
                |pane| keys_panel(pane),
            );
        }
        if is_open(MENU_ABOUT) {
            floating_window_for_item(
                ctx, RIBBONS, RIBBON_ITEMS, placement,
                MENU_ABOUT, "About", egui::vec2(300.0, 220.0),
                &mut keep_open, accent_col,
                |pane| about_panel(pane),
            );
        }

        // ── Context-aware command palette (Ctrl+K) ─────────
        //
        // Same pattern as bevy_frost's demo: the palette items
        // slice is picked based on `is_maximized(ctx, id_salt)`
        // for each special widget. When no widget is maximised,
        // picking an "open_*" item closes every other open pane
        // first via `open.close_all()` so the user always lands
        // on a single-pane layout.
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::K) {
                palette.open = !palette.open;
                palette.query.clear();
                palette.selected = 0;
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
        if let Some(picked) = command_palette(ctx, palette, items, accent_col) {
            *last_picked = Some(picked);
            let is_general = !graph_maxed && !code_maxed;
            if is_general && picked.starts_with("open_") {
                open.close_all();
            }
            match picked {
                "open_widgets"    => { open.toggle(RIBBON_LEFT, MENU_WIDGETS); }
                "open_containers" => { open.toggle(RIBBON_LEFT, MENU_CONTAINERS); }
                "open_scene"      => { open.toggle(RIBBON_LEFT, MENU_SCENE); }
                "open_editor"     => { open.toggle(RIBBON_LEFT, MENU_GRAPH); }
                "open_theme"      => { open.toggle(RIBBON_RIGHT, MENU_THEME); }
                "open_keys"       => { open.toggle(RIBBON_RIGHT, MENU_KEYS); }
                "open_about"      => { open.toggle(RIBBON_RIGHT, MENU_ABOUT); }
                "close_all"       => { open.close_all(); }
                "reset_accent"    => accent.0 = egui_frost::style::ACCENT_NEUTRAL,
                "full_glass"      => glass.0 = 100,
                "half_glass"      => glass.0 = 50,
                "graph_add_number" => {
                    graph.insert_node(egui::pos2(40.0, 40.0), GraphNode::Number(0.0));
                }
                "graph_add_add" => {
                    graph.insert_node(egui::pos2(40.0, 40.0), GraphNode::Add);
                }
                "graph_add_output" => {
                    graph.insert_node(egui::pos2(40.0, 40.0), GraphNode::Output);
                }
                "code_wipe" => { code.clear(); }
                "code_reset" => { *code = default_code(); }
                _ => {}
            }
        }

        // ── Status bar ──────────────────────────────────────
        let sel = scene_tree_selected
            .clone()
            .unwrap_or_else(|| "—".into());
        let copied = copied_path.clone();
        let picked = *last_picked;
        statusbar(
            ctx,
            "demo_statusbar",
            egui::Align2::LEFT_BOTTOM,
            accent_col,
            |ui| {
                ui.label(
                    egui::RichText::new(format!("selected: {sel}"))
                        .color(egui_frost::style::TEXT_PRIMARY),
                );
                if let Some(p) = copied {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("copied: {p}"))
                            .color(egui_frost::style::TEXT_SECONDARY),
                    );
                }
                if let Some(id) = picked {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("last: {id}"))
                            .color(egui_frost::style::TEXT_SECONDARY),
                    );
                }
                ui.separator();
                chip(ui, "Ctrl+K palette", accent_col);
            },
        );
    }
}

// ─── Panels ────────────────────────────────────────────────────────

fn widgets_panel(
    pane: &mut PaneBuilder,
    power: &mut bool,
    headlights: &mut bool,
    gravity: &mut f64,
    speed_limit: &mut f64,
    engine_power: &mut f64,
    throttle: &mut f64,
    brake: &mut f64,
    fuel: &mut f32,
) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_flags", "demo_numbers", "demo_bars", "demo_buttons"]);
    for id in &order {
        match id.as_str() {
            "demo_flags" => pane.section_with("demo_flags", "Flags", true, Some("flag"), 0, |_| {}, |ui| {
                toggle(ui, "power", power, accent);
                toggle(ui, "headlights", headlights, accent);
            }),
            "demo_numbers" => pane.section_with("demo_numbers", "Numbers", true, Some("calculator"), 0, |_| {}, |ui| {
                drag_value(ui, "gravity (m/s²)", gravity, 0.05, 0.0..=30.0, 2, "");
                drag_value(ui, "speed limit (m/s)", speed_limit, 0.1, 0.0..=100.0, 1, "");
                drag_value(ui, "engine power (kW)", engine_power, 1.0, 0.0..=2_000.0, 0, "");
            }),
            "demo_bars" => pane.section_with("demo_bars", "Bars", true, Some("gauge"), 0, |_| {}, |ui| {
                pretty_slider(ui, "throttle", throttle, 0.0..=1.0, 2, "", accent);
                pretty_slider(ui, "brake", brake, 0.0..=1.0, 2, "", accent);
                pretty_progressbar_text(
                    ui,
                    "fuel",
                    *fuel,
                    &format!("{:.0}%", *fuel * 100.0),
                    accent,
                );
            }),
            "demo_buttons" => pane.section_with("demo_buttons", "Buttons", false, Some("button"), 0, |_| {}, |ui| {
                if wide_button(ui, "Refuel", accent).clicked() {
                    *fuel = 1.0;
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

fn containers_panel(
    pane: &mut PaneBuilder,
    position: &mut [f64; 3],
    rotation_deg: &mut [f64; 3],
) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_transform"]);
    for id in &order {
        match id.as_str() {
            "demo_transform" => pane.section_with("demo_transform", "Transform", true, Some("resize"), 0, |_| {}, |ui| {
                subsection(
                    ui,
                    "demo_tr_pos",
                    "Position",
                    Some("drag, double-click to type"),
                    accent,
                    true,
                    |ui| {
                        axis_drag(ui, "X", egui::Color32::from_rgb(0xE0, 0x43, 0x3B),
                            &mut position[0], 0.05, " m", 3);
                        axis_drag(ui, "Y", egui::Color32::from_rgb(0x7F, 0xB4, 0x35),
                            &mut position[1], 0.05, " m", 3);
                        axis_drag(ui, "Z", egui::Color32::from_rgb(0x2E, 0x83, 0xE6),
                            &mut position[2], 0.05, " m", 3);
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
                            &mut rotation_deg[0], 1.0, "°", 2);
                        axis_drag(ui, "Y", egui::Color32::from_rgb(0x7F, 0xB4, 0x35),
                            &mut rotation_deg[1], 1.0, "°", 2);
                        axis_drag(ui, "Z", egui::Color32::from_rgb(0x2E, 0x83, 0xE6),
                            &mut rotation_deg[2], 1.0, "°", 2);
                    },
                );
            }),
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn elements_panel(
    pane: &mut PaneBuilder,
    scene_tree: &mut Vec<SceneNode>,
    scene_tree_selected: &mut Option<String>,
    scene_filter: &mut usize,
    scene_query: &mut String,
    copied_path: &mut Option<String>,
    scene_scroll_h: &mut f32,
    flat_scroll_h: &mut f32,
    scene_entities: &mut Vec<String>,
    scene_selected: &mut Option<usize>,
    scene_following: &mut Option<usize>,
    scene_double_click_count: &mut u32,
) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_scene_tree", "demo_elements"]);
    for id in &order {
        match id.as_str() {
            "demo_scene_tree" => pane.section_with("demo_scene_tree", "Scene", true, Some("folder"), 0, |_| {}, |ui| {
                search_field(ui, scene_query, "filter by name / path…", accent);
                dropdown(ui, "kind", scene_filter, SCENE_FILTERS, accent);

                let scroll_w = ui.available_width();
                let query_lc = scene_query.to_lowercase();
                let scroll_out = ui.allocate_ui_with_layout(
                    egui::vec2(scroll_w, *scene_scroll_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("demo_scene_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                draw_scene_tree(
                                    ui, scene_tree, 0, scene_tree_selected,
                                    *scene_filter, &query_lc, accent, copied_path,
                                );
                            })
                    },
                );
                let content_h = scroll_out.inner.content_size.y;
                let min_h = TREE_ROW_H * 3.0;
                let max_h = content_h.max(min_h);
                row_separator_resize(ui, "scene_scroll_grip", scene_scroll_h, min_h, max_h, accent);

                readout_row(ui, "selected", scene_tree_selected.as_deref().unwrap_or("—"));
                let sel_flags = scene_tree_selected
                    .as_deref()
                    .and_then(|p| find_node(scene_tree, p))
                    .map(|n| n.flags)
                    .unwrap_or(&[]);
                if !sel_flags.is_empty() {
                    badge_row(ui, "flags", sel_flags, accent);
                }
            }),
            "demo_elements" => pane.section_with("demo_elements", "Flat list", true, Some("list"), 0, |_| {}, |ui| {
                let scroll_w = ui.available_width();
                let scroll_out = ui.allocate_ui_with_layout(
                    egui::vec2(scroll_w, *flat_scroll_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("demo_flat_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                for (idx, name) in scene_entities.iter().enumerate() {
                                    let selected = *scene_selected == Some(idx);
                                    let pinned = *scene_following == Some(idx);
                                    let trailing = format!("#{idx}");
                                    let resp = hybrid_select_row(
                                        ui, idx, name, Some(&trailing), selected, pinned, accent,
                                    );
                                    if resp.body.clicked() {
                                        *scene_selected = Some(idx);
                                    }
                                    if resp.body.double_clicked() {
                                        *scene_double_click_count =
                                            scene_double_click_count.wrapping_add(1);
                                    }
                                    if resp.radio.clicked() {
                                        *scene_following = if pinned { None } else { Some(idx) };
                                    }
                                }
                            })
                    },
                );
                let content_h = scroll_out.inner.content_size.y;
                let min_h = HYBRID_SELECT_ROW_H * 3.0;
                let max_h = content_h.max(min_h);
                row_separator_resize(ui, "flat_scroll_grip", flat_scroll_h, min_h, max_h, accent);
                let sel_name = scene_selected
                    .and_then(|i| scene_entities.get(i))
                    .cloned()
                    .unwrap_or_else(|| "—".into());
                let pinned_name = scene_following
                    .and_then(|i| scene_entities.get(i))
                    .cloned()
                    .unwrap_or_else(|| "—".into());
                readout_row(ui, "selected (transient)", &sel_name);
                readout_row(ui, "pinned (durable)", &pinned_name);
                readout_row(ui, "double-clicks", &scene_double_click_count.to_string());
            }),
            _ => {}
        }
    }
}

fn editor_panel(
    pane: &mut PaneBuilder,
    graph: &mut Snarl<GraphNode>,
    viewer: &mut GraphViewer,
    code: &mut String,
) {
    let accent = pane.accent();
    let order = pane.section_order(["demo_graph", "demo_code"]);
    for id in &order {
        match id.as_str() {
            "demo_graph" => pane.section_with(
                "demo_graph",
                "Node graph",
                true,
                Some("flowchart"),
                1,
                |ui| header_action_maximize(ui, "demo_editor_snarl", accent),
                |ui| {
                    sub_caption(ui, "right-click to add nodes · click ▢ to maximise");
                    let w = ui.available_width();
                    frost_snarl(
                        ui,
                        "demo_editor_snarl",
                        graph,
                        viewer,
                        accent,
                        egui::vec2(w, 260.0),
                    );
                },
            ),
            "demo_code" => pane.section_with(
                "demo_code",
                "Source",
                true,
                Some("code"),
                1,
                |ui| header_action_maximize(ui, "demo_editor_code", accent),
                |ui| {
                    sub_caption(ui, "rust syntax · click ▢ to maximise");
                    let w = ui.available_width();
                    frost_code_editor(
                        ui,
                        "demo_editor_code",
                        code,
                        Syntax::rust(),
                        accent,
                        egui::vec2(w, 260.0),
                    );
                },
            ),
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
    let order = pane.section_order(["demo_theme_profile", "demo_theme_colour", "demo_theme_glass"]);
    for id in &order {
        match id.as_str() {
            "demo_theme_profile" => pane.section_with("demo_theme_profile", "Profile", true, Some("person"), 0, |_| {}, |ui| {
                // PRO and GAME are built-in; users can drop in a third
                // by writing their own `Theme { ..theme_game() }` and
                // calling `set_theme`. Stored selection is just an
                // index into the list below — egui temp data keyed by
                // a stable id.
                let key = ui.id().with("frost_theme_profile_idx");
                let mut idx: usize = ui.ctx().data(|d| d.get_temp(key).unwrap_or(0));
                let prev_idx = idx;
                if dropdown(ui, "profile", &mut idx, &["PRO", "GAME"], accent).changed()
                    || prev_idx != idx
                {
                    let chosen = if idx == 1 { theme_game() } else { theme_pro() };
                    set_theme(chosen);
                    ui.ctx().data_mut(|d| d.insert_temp(key, idx));
                }
                sub_caption(
                    ui,
                    "PRO = soft glass, rounded, subtle borders. GAME = square, no borders, full-accent click.",
                );
            }),
            "demo_theme_colour" => pane.section_with("demo_theme_colour", "Accent", true, Some("color"), 0, |_| {}, |ui| {
                let c = accent_res.0;
                let mut rgb = [
                    c.r() as f32 / 255.0,
                    c.g() as f32 / 255.0,
                    c.b() as f32 / 255.0,
                ];
                if color_rgb(ui, "accent", &mut rgb, accent).changed() {
                    accent_res.0 = srgb_to_egui(rgb);
                }
                color_rgba(ui, "tint (rgba)", tint_rgba, accent);
                sub_caption(
                    ui,
                    "Changing accent recolours every widget — one resource, one brush.",
                );
            }),
            "demo_theme_glass" => pane.section_with("demo_theme_glass", "Glass", true, Some("glasses"), 0, |_| {}, |ui| {
                let mut v = glass.0 as f64;
                if pretty_slider(ui, "opacity", &mut v, 1.0..=100.0, 0, "%", accent).changed() {
                    glass.0 = v.round().clamp(1.0, 100.0) as u8;
                }
                sub_caption(ui, "Lower values let the backdrop peek through every pane.");
            }),
            _ => {}
        }
    }
}

fn keys_panel(pane: &mut PaneBuilder) {
    let order = pane.section_order(["demo_keys_app", "demo_keys_layout"]);
    for id in &order {
        match id.as_str() {
            "demo_keys_app" => pane.section_with("demo_keys_app", "App", true, Some("keyboard"), 0, |_| {}, |ui| {
                keybinding_row(ui, "Ctrl+K", "toggle the command palette");
                keybinding_row(ui, "Shift + chevron", "expand / collapse subtree");
                keybinding_row(ui, "Right-click row", "frost-styled context menu");
            }),
            "demo_keys_layout" => pane.section_with("demo_keys_layout", "Layout", false, Some("grid"), 0, |_| {}, |ui| {
                keybinding_row(ui, "Drag panel edge", "resize its cluster's width");
                keybinding_row(ui, "Toggle ribbon btn", "open / close the pane");
            }),
            _ => {}
        }
    }
}

/// Demo pane for the iconflow integration — renders a deterministic
/// sample of ~100 filled Fluent UI System Icons in a grid so the
/// user can eyeball how they read against the active theme.
///
/// `iconflow::list(Pack::Fluentui)` returns thousands of icon
/// names; we pick every Nth so the sample is well-distributed
/// alphabetically and stays the same across runs (no `rand` dep).
fn icons_panel(pane: &mut PaneBuilder) {
    use egui_frost::iconflow::{list, Pack};
    let accent = pane.accent();
    let order = pane.section_order(["demo_icons_grid"]);
    for id in &order {
        match id.as_str() {
            "demo_icons_grid" => pane.section_with("demo_icons_grid", "Fluent icons", true, Some("icons"), 0, |_| {}, |ui| {
                let names: &[&'static str] = list(Pack::Fluentui);
                const SAMPLE_COUNT: usize = 100;
                let stride = (names.len() / SAMPLE_COUNT).max(1);
                let sample: Vec<&'static str> = names
                    .iter()
                    .step_by(stride)
                    .take(SAMPLE_COUNT)
                    .copied()
                    .collect();
                sub_caption(ui, &format!("{} of {} icons (every {}th)", sample.len(), names.len(), stride));

                const COLS: usize = 8;
                const CELL: f32 = 36.0;
                let icon_color = on_section();
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    for name in &sample {
                        let (rect, resp) = ui.allocate_exact_size(
                            egui::vec2(CELL, CELL),
                            egui::Sense::hover(),
                        );
                        // Hover tints the cell so the user can pick
                        // out individual icons in the grid.
                        if resp.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(theme().radius_compact),
                                row_hover_fill(accent),
                            );
                        }
                        if let Some((glyph, family)) = egui_frost::icons::icon(name) {
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                glyph.to_string(),
                                egui::FontId::new(20.0, family),
                                if resp.hovered() { accent } else { icon_color },
                            );
                        }
                        resp.on_hover_text(*name);
                    }
                    // Suppress the unused-COLS warning — the
                    // wrap-layout above honours `available_width`,
                    // not a fixed column count, so COLS just
                    // documents the visual cadence we aim for.
                    let _ = COLS;
                });
            }),
            _ => {}
        }
    }
}

fn about_panel(pane: &mut PaneBuilder) {
    let order = pane.section_order(["demo_about_intro"]);
    for id in &order {
        match id.as_str() {
            "demo_about_intro" => pane.section_with("demo_about_intro", "egui_frost", true, Some("info"), 0, |_| {}, |ui| {
                sub_caption(ui, "Reusable glass-themed editor UI kit for plain egui / eframe.");
                readout_row(ui, "version", env!("CARGO_PKG_VERSION"));
                readout_row(ui, "egui", "0.33");
                readout_row(ui, "eframe", "0.33");
            }),
            _ => {}
        }
    }
}

// ─── Scene tree types / helpers ────────────────────────────────────

#[derive(Clone)]
struct SceneNode {
    path: String,
    name: String,
    kind: NodeKind,
    visible: bool,
    locked: bool,
    expanded: bool,
    children: Vec<SceneNode>,
    material: egui::Color32,
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
            NodeKind::Group => "folder",
            NodeKind::Mesh => "cube",
            NodeKind::Light => "lightbulb",
            NodeKind::Camera => "camera",
        }
    }
}

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
            path: path.into(), name: name.into(), kind,
            visible: true, locked: false, expanded, children, material, flags,
        }
    }
    let red = egui::Color32::from_rgb(0xE0, 0x43, 0x3B);
    let green = egui::Color32::from_rgb(0x7F, 0xB4, 0x35);
    let blue = egui::Color32::from_rgb(0x2E, 0x83, 0xE6);
    let yellow = egui::Color32::from_rgb(0xF5, 0xA5, 0x24);
    let grey = egui::Color32::from_rgb(0x70, 0x70, 0x70);
    vec![node(
        "/World", "World", NodeKind::Group, true, grey, &[],
        vec![
            node("/World/Robot", "Robot", NodeKind::Group, true, red, &["rig"], vec![
                node("/World/Robot/base_link", "base_link", NodeKind::Mesh, false, red, &[], vec![]),
                node("/World/Robot/arm", "arm", NodeKind::Group, false, red, &["anim"], vec![
                    node("/World/Robot/arm/shoulder", "shoulder", NodeKind::Mesh, false, red, &["anim"], vec![]),
                    node("/World/Robot/arm/elbow", "elbow", NodeKind::Mesh, false, red, &["anim"], vec![]),
                    node("/World/Robot/arm/gripper", "gripper", NodeKind::Mesh, false, red, &["anim", "inst"], vec![]),
                ]),
            ]),
            node("/World/Environment", "Environment", NodeKind::Group, true, grey, &[], vec![
                node("/World/Environment/Ground", "Ground", NodeKind::Mesh, false, green, &["subdiv"], vec![]),
                node("/World/Environment/Lights", "Lights", NodeKind::Group, false, yellow, &[], vec![
                    node("/World/Environment/Lights/Key", "KeyLight", NodeKind::Light, false, yellow, &["anim"], vec![]),
                    node("/World/Environment/Lights/Fill", "FillLight", NodeKind::Light, false, yellow, &[], vec![]),
                ]),
                node("/World/Environment/SkyDome", "SkyDome", NodeKind::Mesh, false, blue, &["var"], vec![]),
            ]),
            node("/World/Camera", "Camera", NodeKind::Camera, false, grey, &["linked"], vec![]),
        ],
    )]
}

#[allow(clippy::too_many_arguments)]
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
    for node in nodes.iter_mut() {
        if !node_passes_filters(node, filter, query) {
            continue;
        }
        let is_branch = !node.children.is_empty();
        let is_selected = selected.as_deref() == Some(node.path.as_str());
        let path_for_click = node.path.clone();
        let mat = node.material;
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
            ui, node.path.as_str(), depth,
            if is_branch { Some(&mut node.expanded) } else { None },
            Some(node.kind.glyph()), &node.name, is_selected, accent, &mut slots,
        );
        if resp.body.clicked() {
            *selected = Some(path_for_click.clone());
        }
        if resp.chevron_shift_clicked {
            let new_state = !node.expanded;
            node.expanded = new_state;
            set_subtree_expanded(&mut node.children, new_state);
        }
        let path_for_menu = node.path.clone();
        context_menu_frost(&resp.body, accent, |ui| {
            if wide_button(ui, "Copy path", accent).clicked() {
                *copied_path = Some(path_for_menu.clone());
                ui.close();
            }
        });
        if is_branch && node.expanded {
            draw_scene_tree(
                ui, &mut node.children, depth + 1, selected, filter, query, accent, copied_path,
            );
        }
    }
}

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
    if matches!(node.kind, NodeKind::Group)
        && node.children.iter().any(|c| node_passes_filters(c, filter, query))
    {
        return true;
    }
    false
}

fn set_subtree_expanded(nodes: &mut [SceneNode], open: bool) {
    for n in nodes.iter_mut() {
        n.expanded = open;
        set_subtree_expanded(&mut n.children, open);
    }
}

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

// ─── Graph viewer + seed ───────────────────────────────────────────

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

fn default_code() -> String {
    r#"// Frost code editor — Rust syntax highlighting.
fn fibonacci(n: u64) -> u64 {
    if n < 2 { return n; }
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
