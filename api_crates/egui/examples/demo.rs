//! `egui_frost` widget gallery + layout showcase. Mirrors
//! `bevy_frost`'s `demo` example pane-for-pane — the only
//! difference is the host: `eframe` instead of Bevy. Every pane,
//! container, pod, and widget code path runs verbatim from the
//! frost_core side.
//!
//! Panes:
//!
//! * **Widgets** — Flags / Numbers / Bars / Buttons / Animated.
//! * **Containers** — Position + Rotation, axis-coloured drag values.
//! * **Elements** — scene tree (eye/lock/colour slots) + flat
//!   hybrid_select roster.
//! * **Editor** — node graph + code editor.
//! * **Theme** — Profile dropdowns + accent picker + glass slider.
//! * **Keys** — keybinding rows.
//! * **About** — version + dependency readouts + chip clusters.
//!
//! Run: `cargo run -p egui_frost --example demo`.

use eframe::egui;

use frost_core::container::SeparatorStyle;
use frost_core::pane::{Pane, PaneAnchor, PaneBody, RailZone};
use frost_core::pod::Pod;
use frost_core::ribbon::{
    RibbonCluster, RibbonDef, RibbonDrag, RibbonEdge, RibbonGlyph, RibbonItem, RibbonMode,
    RibbonOpen, RibbonPlacement, RibbonRole, draw_assembly, find_item, find_ribbon,
};
use frost_core::style::{AccentColor, GlassOpacity, Mode, srgb_to_egui};
use frost_core::widget::{FillStyle, TreeIconKind, TreeIconSlot};
// Vendored extras — node graph + code editor. Both live in frost_core
// so egui_frost can reach them without any Bevy dep.
use egui_frost::EframeNodeViewBackend;
use frost_core::extras::code::Syntax;
use frost_core::extras::graph::{
    Graph, InPin, InPinId, NodePin, NodeViewState, NodeViewer, OutPin, OutPinId, PinInfo,
};

// ─── Ribbon / pane ids ──────────────────────────────────────────────

const RIBBON_LEFT: &str = "demo_ribbon_left";
const RIBBON_RIGHT: &str = "demo_ribbon_right";
const RIBBON_TOP: &str = "demo_ribbon_top";
const RIBBON_BOTTOM: &str = "demo_ribbon_bottom";

const PANE_WIDGETS: &str = "demo_pane_widgets";
const PANE_CONTAINERS: &str = "demo_pane_containers";
const PANE_SCENE: &str = "demo_pane_scene";
const PANE_EDITOR: &str = "demo_pane_editor";
const PANE_THEME: &str = "demo_pane_theme";
const PANE_KEYS: &str = "demo_pane_keys";
const PANE_ABOUT: &str = "demo_pane_about";

const ACTION_PREV_CUBE: &str = "demo_action_prev_cube";
const ACTION_NEXT_CUBE: &str = "demo_action_next_cube";

const PANE_DEFS: &[(&str, &str, PaneAnchor, &str)] = &[
    (
        RIBBON_LEFT,
        PANE_WIDGETS,
        PaneAnchor::LeftRail(RailZone::Start),
        "Widgets",
    ),
    (
        RIBBON_LEFT,
        PANE_CONTAINERS,
        PaneAnchor::LeftRail(RailZone::Middle),
        "Containers",
    ),
    (
        RIBBON_LEFT,
        PANE_SCENE,
        PaneAnchor::LeftRail(RailZone::End),
        "Elements",
    ),
    (
        RIBBON_RIGHT,
        PANE_THEME,
        PaneAnchor::RightRail(RailZone::Start),
        "Theme",
    ),
    (
        RIBBON_RIGHT,
        PANE_KEYS,
        PaneAnchor::RightRail(RailZone::Middle),
        "Keys",
    ),
    (
        RIBBON_TOP,
        PANE_ABOUT,
        PaneAnchor::TopRail(RailZone::Start),
        "About",
    ),
    (
        RIBBON_BOTTOM,
        PANE_EDITOR,
        PaneAnchor::BottomRail(RailZone::Start),
        "Editor",
    ),
];

const RIBBONS: &[RibbonDef] = &[
    RibbonDef {
        id: RIBBON_LEFT,
        edge: RibbonEdge::Left,
        role: RibbonRole::Panel,
        mode: RibbonMode::ThreeSided,
        draggable: true,
        accepts: &[RIBBON_RIGHT, RIBBON_TOP, RIBBON_BOTTOM],
    },
    RibbonDef {
        id: RIBBON_RIGHT,
        edge: RibbonEdge::Right,
        role: RibbonRole::Panel,
        mode: RibbonMode::ThreeSided,
        draggable: true,
        accepts: &[RIBBON_LEFT, RIBBON_TOP, RIBBON_BOTTOM],
    },
    RibbonDef {
        id: RIBBON_TOP,
        edge: RibbonEdge::Top,
        role: RibbonRole::Panel,
        mode: RibbonMode::ThreeSided,
        draggable: true,
        accepts: &[RIBBON_LEFT, RIBBON_RIGHT, RIBBON_BOTTOM],
    },
    RibbonDef {
        id: RIBBON_BOTTOM,
        edge: RibbonEdge::Bottom,
        role: RibbonRole::Panel,
        mode: RibbonMode::ThreeSided,
        draggable: true,
        accepts: &[RIBBON_LEFT, RIBBON_RIGHT, RIBBON_TOP],
    },
];

const RIBBON_ITEMS: &[RibbonItem] = &[
    // LEFT rail — primary navigation cluster.
    RibbonItem {
        id: PANE_WIDGETS,
        ribbon: RIBBON_LEFT,
        cluster: RibbonCluster::Start,
        slot: 0,
        glyph: RibbonGlyph::Icon("apps"),
        tooltip: "Widgets gallery",
        child_ribbon: None,
        role: None,
    },
    RibbonItem {
        id: PANE_CONTAINERS,
        ribbon: RIBBON_LEFT,
        cluster: RibbonCluster::Start,
        slot: 1,
        glyph: RibbonGlyph::Icon("box"),
        tooltip: "Containers showcase",
        child_ribbon: None,
        role: None,
    },
    RibbonItem {
        id: PANE_SCENE,
        ribbon: RIBBON_LEFT,
        cluster: RibbonCluster::Start,
        slot: 2,
        glyph: RibbonGlyph::Icon("folder"),
        tooltip: "Scene outliner",
        child_ribbon: None,
        role: None,
    },
    // RIGHT rail — theme + input.
    RibbonItem {
        id: PANE_THEME,
        ribbon: RIBBON_RIGHT,
        cluster: RibbonCluster::Start,
        slot: 0,
        glyph: RibbonGlyph::Icon("color"),
        tooltip: "Theme & colour",
        child_ribbon: None,
        role: None,
    },
    RibbonItem {
        id: PANE_KEYS,
        ribbon: RIBBON_RIGHT,
        cluster: RibbonCluster::Start,
        slot: 1,
        glyph: RibbonGlyph::Icon("keyboard"),
        tooltip: "Keys & gestures",
        child_ribbon: None,
        role: None,
    },
    // TOP rail — meta.
    RibbonItem {
        id: PANE_ABOUT,
        ribbon: RIBBON_TOP,
        cluster: RibbonCluster::Start,
        slot: 0,
        glyph: RibbonGlyph::Icon("info"),
        tooltip: "About this demo",
        child_ribbon: None,
        role: None,
    },
    // BOTTOM rail — Editor (placeholder; the legacy graph + code
    // wrappers lived in `frostcore` which has been removed) and the
    // one-shot cube-cycle action buttons in the End cluster.
    RibbonItem {
        id: PANE_EDITOR,
        ribbon: RIBBON_BOTTOM,
        cluster: RibbonCluster::Start,
        slot: 0,
        glyph: RibbonGlyph::Icon("flowchart"),
        tooltip: "Editor",
        child_ribbon: None,
        role: None,
    },
    RibbonItem {
        id: ACTION_PREV_CUBE,
        ribbon: RIBBON_BOTTOM,
        cluster: RibbonCluster::End,
        slot: 0,
        glyph: RibbonGlyph::Icon("arrow-left"),
        tooltip: "Previous cube",
        child_ribbon: None,
        role: Some(RibbonRole::Icon),
    },
    RibbonItem {
        id: ACTION_NEXT_CUBE,
        ribbon: RIBBON_BOTTOM,
        cluster: RibbonCluster::End,
        slot: 1,
        glyph: RibbonGlyph::Icon("arrow-right"),
        tooltip: "Next cube",
        child_ribbon: None,
        role: Some(RibbonRole::Icon),
    },
];

// ─── Theme + app state ─────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default)]
struct ThemeFamily(u8);

#[derive(Clone, Copy, Debug, Default)]
struct ThemeModeRes(u8);

#[derive(Clone, Copy, Debug)]
struct PastelToggle(bool);
impl Default for PastelToggle {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Clone, Copy, Debug)]
struct TintRgba(pub [f32; 4]);
impl Default for TintRgba {
    fn default() -> Self {
        Self([0.5, 0.7, 0.9, 0.6])
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DemoRootView {
    #[default]
    FrostGallery,
    EframeHost,
}

/// All the per-frame mutable state the demo carries — accent
/// colour, glass opacity, ribbon state, and the various theme
/// toggles. Mirrors what `bevy_frost`'s demo holds across separate
/// `Resource`s; here it's one struct on the `eframe::App`.
struct FrostApp {
    accent: AccentColor,
    glass: GlassOpacity,
    open: RibbonOpen,
    placement: RibbonPlacement,
    drag: RibbonDrag,
    family: ThemeFamily,
    mode: ThemeModeRes,
    pastel: PastelToggle,
    tint: TintRgba,
    root_view: DemoRootView,
    /// Persistent secondary-context state for the node graph
    /// (sub egui::Context, pan, zoom, wgpu render target).
    /// Owns wgpu resources, hence App-owned and passed by &mut
    /// each frame into `frost_node_graph`.
    node_view: NodeViewState,
    /// The actual node-graph data — held on the App so the graph
    /// renderer can mutate it via &mut. Previously stashed in
    /// egui ctx data, but the v2 path needs a non-`'static`
    /// closure body to thread `&mut self.node_view`.
    graph: Graph<GraphNode>,
    /// The viewer (=node-painter trait impl) is a unit struct; it
    /// could live anywhere but pairing it with the graph on App
    /// keeps the callsite tidy.
    viewer: DemoViewer,
}

impl Default for FrostApp {
    fn default() -> Self {
        Self {
            accent: AccentColor::default(),
            glass: GlassOpacity::default(),
            open: RibbonOpen::default(),
            placement: RibbonPlacement::default(),
            drag: RibbonDrag::default(),
            family: ThemeFamily::default(),
            mode: ThemeModeRes::default(),
            pastel: PastelToggle::default(),
            tint: TintRgba::default(),
            root_view: DemoRootView::default(),
            node_view: NodeViewState::new(),
            graph: default_graph(),
            viewer: DemoViewer,
        }
    }
}

// ─── App entry ─────────────────────────────────────────────────────

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("egui_frost — demo"),
        ..Default::default()
    };
    eframe::run_native(
        "egui_frost demo",
        native_options,
        Box::new(|_cc| Ok(Box::new(FrostApp::default()))),
    )
}

// ─── eframe glue ───────────────────────────────────────────────────

impl eframe::App for FrostApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Match `bevy_frost`'s scene clear colour so the
        // see-through / glass surfaces in the panes look the same
        // against the background — egui_frost has no 3D scene
        // behind, so the clear colour is just the flat fill.
        [0.06, 0.08, 0.12, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Snapshot the wgpu RenderState eframe is using — this is
        // what `EframeNodeViewBackend` borrows from to drive the
        // sharp-zoom node graph's secondary-context render. Cheap
        // clone (Arc-counted handles) so we keep one local for
        // the whole frame.
        let render_state = frame
            .wgpu_render_state()
            .expect("wgpu backend required for frost_node_graph")
            .clone();
        let FrostApp {
            accent,
            glass,
            open,
            placement,
            drag,
            family,
            mode,
            pastel,
            tint,
            root_view,
            node_view,
            graph,
            viewer,
        } = self;
        let mut active_theme = match (family.0, mode.0) {
            (0, 0) => frost_core::style::theme_pro(Mode::Dark),
            (0, 1) => frost_core::style::theme_pro(Mode::Light),
            (1, 0) => frost_core::style::theme_game(Mode::Dark),
            (1, 1) => frost_core::style::theme_game(Mode::Light),
            (2, 0) => frost_core::style::theme_flat(Mode::Dark),
            (2, 1) => frost_core::style::theme_flat(Mode::Light),
            _ => frost_core::style::theme_pro(Mode::Dark),
        };
        active_theme.pastel_accent = pastel.0;
        frost_core::style::set_theme(active_theme);
        frost_core::style::apply_theme(ctx, *accent, *glass);

        let accent_col = frost_core::style::active_accent();
        // Ribbon assembly is rendered AFTER the pane loop below — see
        // the trailing `draw_assembly` call. The ribbon `Area`s share
        // `Order::Foreground` with the `embed` fullscreen overlay, so
        // they must register later to land on top of it.

        match *root_view {
            DemoRootView::FrostGallery => {
                let is_open = |id: &'static str| -> bool {
                    let Some(item) = find_item(RIBBON_ITEMS, id) else {
                        return false;
                    };
                    let (rid, _, _) = placement.resolve(item);
                    open.is_open(rid, id)
                };
                let live_anchor = |id: &'static str| -> Option<PaneAnchor> {
                    let item = find_item(RIBBON_ITEMS, id)?;
                    let (rid, cluster, _) = placement.resolve(item);
                    let def = find_ribbon(RIBBONS, rid)?;
                    let zone = match cluster {
                        RibbonCluster::Start => RailZone::Start,
                        RibbonCluster::Middle => RailZone::Middle,
                        RibbonCluster::End => RailZone::End,
                    };
                    Some(match def.edge {
                        RibbonEdge::Left => PaneAnchor::LeftRail(zone),
                        RibbonEdge::Right => PaneAnchor::RightRail(zone),
                        RibbonEdge::Top => PaneAnchor::TopRail(zone),
                        RibbonEdge::Bottom => PaneAnchor::BottomRail(zone),
                    })
                };

                for &(_, button_id, default_anchor, label) in PANE_DEFS {
                    if !is_open(button_id) {
                        continue;
                    }
                    let anchor = live_anchor(button_id).unwrap_or(default_anchor);
                    // Editor pane uses non-`'static` borrows that have
                    // to outlive `Pane::show` (see `editor_pane`'s
                    // 'spec-bound signature). Hoist `viewer` and the
                    // backend into the iteration scope so they live
                    // past the closure body.
                    if button_id == PANE_EDITOR {
                        let mut backend = EframeNodeViewBackend::new(&render_state);
                        Pane::new(button_id, label, anchor, accent_col)
                            .resize(frost_core::pane::PaneResize::SPAN)
                            .show(ctx, |body| {
                                editor_pane(
                                    body,
                                    &mut *node_view,
                                    &mut *graph,
                                    &mut *viewer,
                                    &mut backend,
                                );
                            });
                        continue;
                    }
                    Pane::new(button_id, label, anchor, accent_col)
                        .resize(frost_core::pane::PaneResize::SPAN)
                        .show(ctx, |body| match button_id {
                            PANE_WIDGETS => widgets_pane(body),
                            PANE_CONTAINERS => containers_pane(body),
                            PANE_SCENE => scene_pane(body),
                            PANE_EDITOR => unreachable!("handled above"),
                            PANE_THEME => {
                                theme_pane(body, accent, glass, family, mode, pastel, tint)
                            }
                            PANE_KEYS => keys_pane(body),
                            PANE_ABOUT => about_pane(body),
                            _ => {}
                        });
                }

                // Ribbon paint, AFTER the panes — registration order within
                // `Order::Foreground` lands the ribbon `Area`s on top of the
                // `embed` fullscreen overlay.
                let clicks = draw_assembly(
                    ctx,
                    accent_col,
                    RIBBONS,
                    RIBBON_ITEMS,
                    open,
                    placement,
                    drag,
                    |_| false,
                );
                const SWATCH_RGB: &[(u8, u8, u8)] = &[
                    (230, 76, 76),
                    (242, 166, 51),
                    (242, 230, 76),
                    (89, 217, 115),
                    (76, 153, 242),
                    (191, 115, 242),
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
            }
            DemoRootView::EframeHost => {
                eframe_host_view(ctx, accent_col);
            }
        }

        if let Some(next_view) = draw_demo_view_switcher(ctx, accent_col, *root_view) {
            *root_view = next_view;
        }
    }
}

// ─── Root view switcher ────────────────────────────────────────────

fn draw_demo_view_switcher(
    ctx: &egui::Context,
    accent: egui::Color32,
    active: DemoRootView,
) -> Option<DemoRootView> {
    let mut next = None;
    let screen = ctx.content_rect();
    let pos = egui::pos2(screen.center().x - 120.0, screen.top() + 8.0);

    egui::Area::new(egui::Id::new("demo.root_view_switcher.visible"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(frost_core::style::glass_fill(
                    frost_core::style::theme().palette.bg_panel,
                    accent,
                    frost_core::style::glass_alpha_card(),
                ))
                .stroke(frost_core::style::stroke_for(
                    frost_core::style::StrokeRole::WidgetBorder,
                    accent,
                ))
                .corner_radius(frost_core::style::radius_for(
                    frost_core::style::RadiusRole::Widget,
                ))
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if root_view_button(
                            ui,
                            "▦ Frost",
                            active == DemoRootView::FrostGallery,
                            accent,
                        ) {
                            next = Some(DemoRootView::FrostGallery);
                        }
                        if root_view_button(
                            ui,
                            "▣ Eframe",
                            active == DemoRootView::EframeHost,
                            accent,
                        ) {
                            next = Some(DemoRootView::EframeHost);
                        }
                    });
                });
        });

    next
}

fn root_view_button(
    ui: &mut egui::Ui,
    label: &'static str,
    active: bool,
    accent: egui::Color32,
) -> bool {
    let fill = if active {
        accent
    } else {
        frost_core::style::theme().palette.bg_raised
    };
    ui.add(
        egui::Button::new(egui::RichText::new(label).strong())
            .fill(fill)
            .stroke(frost_core::style::stroke_for(
                frost_core::style::StrokeRole::WidgetBorder,
                accent,
            ))
            .min_size(egui::vec2(104.0, 28.0)),
    )
    .on_hover_text("Switch root/L0 demo view")
    .clicked()
}

fn eframe_host_view(ctx: &egui::Context, accent: egui::Color32) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(96.0);
            ui.heading("Eframe host view");
            ui.label("This is a second root/L0 view inside the main demo.");
            ui.label(
                "Use the permanent top-center Frost / Eframe switcher to swap the whole canvas.",
            );
            ui.add_space(16.0);
            ui.group(|ui| {
                ui.label("Plain egui content can live here without the legacy pane gallery.");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.colored_label(accent, "accent");
                    ui.label(format!(
                        "#{:02X}{:02X}{:02X}",
                        accent.r(),
                        accent.g(),
                        accent.b()
                    ));
                });
                let mut demo_value = ui.ctx().data(|d| {
                    d.get_temp::<f32>(egui::Id::new("demo.eframe.slider"))
                        .unwrap_or(42.0)
                });
                if ui
                    .add(egui::Slider::new(&mut demo_value, 0.0..=100.0).text("host value"))
                    .changed()
                {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(egui::Id::new("demo.eframe.slider"), demo_value)
                    });
                }
            });
        });
    });
}

// ─── Per-pane content ──────────────────────────────────────────────

fn cid(pane: &str, suffix: &str) -> egui::Id {
    egui::Id::new((pane, suffix))
}
fn pid(pane: &str, container: &str, idx: usize) -> egui::Id {
    egui::Id::new((pane, container, "pod", idx))
}

/// **Widgets pane** — one container per widget category.
fn widgets_pane(body: &mut PaneBody) {
    let accent = body.accent();
    let anim = |name: &str, style: FillStyle, sep: SeparatorStyle, idx: usize| -> Pod {
        Pod::new(pid(PANE_WIDGETS, "anim", idx))
            .with_separator(sep)
            .with_button_animated(name, accent, style)
    };
    body.add_normal(
        cid(PANE_WIDGETS, "flags"),
        "Flags",
        "flag",
        vec![
            Pod::new(pid(PANE_WIDGETS, "flags", 0))
                .with_separator(SeparatorStyle::Line)
                .with_toggle_initial("power", accent, true),
            Pod::new(pid(PANE_WIDGETS, "flags", 1))
                .with_separator(SeparatorStyle::None)
                .with_toggle_initial("headlights", accent, false),
        ],
    );
    body.add_normal(
        cid(PANE_WIDGETS, "numbers"),
        "Numbers",
        "calculator",
        vec![
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
    );
    body.add_normal(
        cid(PANE_WIDGETS, "bars"),
        "Bars",
        "gauge",
        vec![
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
    );
    body.add_normal(
        cid(PANE_WIDGETS, "buttons"),
        "Buttons",
        "button",
        vec![
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
    );
    body.add_normal(
        cid(PANE_WIDGETS, "anim"),
        "Animated",
        "animation",
        vec![
            anim("Slide left", FillStyle::SlideLeft, SeparatorStyle::Line, 0),
            anim(
                "Parallelogram",
                FillStyle::Parallelogram,
                SeparatorStyle::Line,
                1,
            ),
            anim(
                "Parallelogram meet",
                FillStyle::ParallelogramMeet,
                SeparatorStyle::Line,
                2,
            ),
            anim("Bowtie", FillStyle::Bowtie, SeparatorStyle::Line, 3),
            anim("Bands meet", FillStyle::BandsMeet, SeparatorStyle::Line, 4),
            anim(
                "Corner squares",
                FillStyle::CornerSquares,
                SeparatorStyle::Line,
                5,
            ),
            anim(
                "Diagonal triangles",
                FillStyle::DiagonalTriangles,
                SeparatorStyle::Line,
                6,
            ),
            anim(
                "Circle grow",
                FillStyle::CircleGrow,
                SeparatorStyle::Line,
                7,
            ),
            anim("Equalizer", FillStyle::Equalizer, SeparatorStyle::Line, 8),
            anim(
                "Horizontal slide",
                FillStyle::HorizontalSlide,
                SeparatorStyle::Line,
                9,
            ),
            anim(
                "Horizontal delayed",
                FillStyle::HorizontalSlideDelayed,
                SeparatorStyle::Line,
                10,
            ),
            anim(
                "Vertical delayed",
                FillStyle::VerticalSlideDelayed,
                SeparatorStyle::Line,
                11,
            ),
            anim(
                "Criss cross",
                FillStyle::CrissCross,
                SeparatorStyle::None,
                12,
            ),
        ],
    );
}

/// **Containers pane** — two tabbed containers stacked: `Transform`
/// (Position / Rotation / Scale) and `Velocity` (Linear / Angular).
fn containers_pane(body: &mut PaneBody) {
    body.add_tabbed(
        cid(PANE_CONTAINERS, "xform"),
        "Transform",
        "cube",
        vec![
            frost_core::container::Tab::new("xform.position", "Position", "arrow-move").pods(vec![
                Pod::new(pid(PANE_CONTAINERS, "pos", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("X", 0.0, 0.05, -1000.0..=1000.0, 3, " m"),
                Pod::new(pid(PANE_CONTAINERS, "pos", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("Y", 0.0, 0.05, -1000.0..=1000.0, 3, " m"),
                Pod::new(pid(PANE_CONTAINERS, "pos", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_drag_value("Z", 0.0, 0.05, -1000.0..=1000.0, 3, " m"),
            ]),
            frost_core::container::Tab::new("xform.rotation", "Rotation", "arrow-rotate-clockwise")
                .pods(vec![
                    Pod::new(pid(PANE_CONTAINERS, "rot", 0))
                        .with_separator(SeparatorStyle::Line)
                        .with_drag_value("X", 0.0, 1.0, -360.0..=360.0, 2, "°"),
                    Pod::new(pid(PANE_CONTAINERS, "rot", 1))
                        .with_separator(SeparatorStyle::Line)
                        .with_drag_value("Y", 0.0, 1.0, -360.0..=360.0, 2, "°"),
                    Pod::new(pid(PANE_CONTAINERS, "rot", 2))
                        .with_separator(SeparatorStyle::None)
                        .with_drag_value("Z", 0.0, 1.0, -360.0..=360.0, 2, "°"),
                ]),
            frost_core::container::Tab::new("xform.scale", "Scale", "maximize").pods(vec![
                Pod::new(pid(PANE_CONTAINERS, "scl", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("X", 1.0, 0.01, 0.01..=100.0, 3, "×"),
                Pod::new(pid(PANE_CONTAINERS, "scl", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("Y", 1.0, 0.01, 0.01..=100.0, 3, "×"),
                Pod::new(pid(PANE_CONTAINERS, "scl", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_drag_value("Z", 1.0, 0.01, 0.01..=100.0, 3, "×"),
            ]),
        ],
    );
    body.add_tabbed(
        cid(PANE_CONTAINERS, "vel"),
        "Velocity",
        "flash",
        vec![
            frost_core::container::Tab::new("vel.linear", "Linear", "arrow-trending").pods(vec![
                Pod::new(pid(PANE_CONTAINERS, "vlin", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("X", 0.0, 0.05, -100.0..=100.0, 2, " m/s"),
                Pod::new(pid(PANE_CONTAINERS, "vlin", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("Y", 0.0, 0.05, -100.0..=100.0, 2, " m/s"),
                Pod::new(pid(PANE_CONTAINERS, "vlin", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_drag_value("Z", 0.0, 0.05, -100.0..=100.0, 2, " m/s"),
            ]),
            frost_core::container::Tab::new(
                "vel.angular",
                "Angular",
                "arrow-rotate-counterclockwise",
            )
            .pods(vec![
                Pod::new(pid(PANE_CONTAINERS, "vang", 0))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("X", 0.0, 0.1, -720.0..=720.0, 2, " °/s"),
                Pod::new(pid(PANE_CONTAINERS, "vang", 1))
                    .with_separator(SeparatorStyle::Line)
                    .with_drag_value("Y", 0.0, 0.1, -720.0..=720.0, 2, " °/s"),
                Pod::new(pid(PANE_CONTAINERS, "vang", 2))
                    .with_separator(SeparatorStyle::None)
                    .with_drag_value("Z", 0.0, 0.1, -720.0..=720.0, 2, " °/s"),
            ]),
        ],
    );
}

/// **Scene pane** — outliner tree + flat hybrid_select roster.
fn scene_pane(body: &mut PaneBody) {
    let accent = body.accent();
    let tree_root = cid(PANE_SCENE, "tree_root");
    let search_pod_id = pid(PANE_SCENE, "scene", 0);
    let tree_filter =
        frost_core::pod::Pod::search_query(body.ctx(), search_pod_id, 0).to_lowercase();
    let selected_path: String = body
        .ctx()
        .data(|d| d.get_temp::<String>(tree_root.with("frost_demo_tree_selected")))
        .unwrap_or_default();
    let selected_display = if selected_path.is_empty() {
        "—".to_string()
    } else {
        selected_path
    };

    let entities: Vec<String> = [
        "Planet",
        "Robot",
        "Sun",
        "Cloud Shell",
        "Camera",
        "Swatch[0]",
        "Swatch[1]",
        "Swatch[2]",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let trailing: Vec<String> = (0..entities.len()).map(|i| format!("#{i}")).collect();

    body.add_normal(
        cid(PANE_SCENE, "scene"),
        "Scene",
        "folder",
        vec![
            Pod::new(pid(PANE_SCENE, "scene", 0))
                .with_separator(SeparatorStyle::Line)
                .with_search("filter by name / path…", accent),
            Pod::new(pid(PANE_SCENE, "scene", 1))
                .with_separator(SeparatorStyle::Line)
                .with_dropdown(["all", "transforms", "lights", "meshes"], 0, accent),
            Pod::new(pid(PANE_SCENE, "scene", 2))
                .with_separator(SeparatorStyle::Line)
                .fill()
                .with_tree(7, move |tree| {
                    demo_tree(tree, tree_root, accent, &tree_filter)
                }),
            Pod::new(pid(PANE_SCENE, "scene", 3))
                .with_separator(SeparatorStyle::None)
                .with_readout("selected", selected_display),
        ],
    );
    body.add_normal(
        cid(PANE_SCENE, "flat"),
        "Flat list",
        "list",
        vec![
            Pod::new(pid(PANE_SCENE, "flat", 0))
                .with_separator(SeparatorStyle::LineDots)
                .resizable()
                .with_hybrid_select_list(entities, Some(trailing), accent),
        ],
    );
}

/// **Theme pane** — Profile / Accent / Glass.
#[allow(clippy::too_many_arguments)]
fn theme_pane(
    body: &mut PaneBody,
    accent_res: &mut AccentColor,
    glass: &mut GlassOpacity,
    family: &mut ThemeFamily,
    mode: &mut ThemeModeRes,
    pastel: &mut PastelToggle,
    tint: &mut TintRgba,
) {
    let accent = body.accent();
    let profile_id = cid(PANE_THEME, "profile");
    let accent_id = cid(PANE_THEME, "accent");
    let glass_id = cid(PANE_THEME, "glass");
    body.add_normal(
        profile_id,
        "Profile",
        "person",
        vec![
            Pod::new(pid(PANE_THEME, "profile", 0))
                .with_separator(SeparatorStyle::Line)
                .with_dropdown(["PRO", "GAME", "FLAT"], family.0 as usize, accent),
            Pod::new(pid(PANE_THEME, "profile", 1))
                .with_separator(SeparatorStyle::Line)
                .with_dropdown(["Dark", "Light"], mode.0 as usize, accent),
            Pod::new(pid(PANE_THEME, "profile", 2))
                .with_separator(SeparatorStyle::None)
                .with_toggle_initial("pastel accent", accent, pastel.0),
        ],
    );
    body.add_normal(
        accent_id,
        "Accent",
        "color",
        vec![
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
    );
    body.add_normal(
        glass_id,
        "Glass",
        "glasses",
        vec![
            Pod::new(pid(PANE_THEME, "glass", 0))
                .with_separator(SeparatorStyle::None)
                .with_slider("opacity", glass.0 as f64, 1.0..=100.0, 0, "%", accent),
        ],
    );
    let responses = body.render();
    // Wire response → mutable state.
    if let Some(pr) = responses.get(&profile_id) {
        if let Some(p0) = pr.first() {
            if let Some(d) = p0.dropdowns.first() {
                if d.changed {
                    family.0 = d.selected as u8;
                }
            }
        }
        if let Some(p1) = pr.get(1) {
            if let Some(d) = p1.dropdowns.first() {
                if d.changed {
                    mode.0 = d.selected as u8;
                }
            }
        }
        if let Some(p2) = pr.get(2) {
            if let Some(t) = p2.toggles.first() {
                if t.changed {
                    pastel.0 = t.on;
                }
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
                if c.changed {
                    tint.0 = c.rgba;
                }
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
fn keys_pane(body: &mut PaneBody) {
    body.add_normal(
        cid(PANE_KEYS, "mouse"),
        "Mouse",
        "cursor",
        vec![
            Pod::new(pid(PANE_KEYS, "mouse", 0))
                .with_separator(SeparatorStyle::None)
                .with_keybindings(vec![
                    ("MMB drag", "pan camera focus"),
                    ("LMB+RMB", "orbit camera"),
                    ("Scroll", "log-smooth zoom"),
                    ("LMB cube", "re-tint UI accent"),
                ]),
        ],
    );
    body.add_normal(
        cid(PANE_KEYS, "layout"),
        "Layout",
        "grid",
        vec![
            Pod::new(pid(PANE_KEYS, "layout", 0))
                .with_separator(SeparatorStyle::None)
                .with_keybindings(vec![
                    ("Drag edge", "resize the pane"),
                    ("Click btn", "open / close pane"),
                    ("Drag btn", "reorder ribbon"),
                    ("F12", "egui debug overlay"),
                ]),
        ],
    );
    body.add_normal(
        cid(PANE_KEYS, "global"),
        "Global",
        "keyboard",
        vec![
            Pod::new(pid(PANE_KEYS, "global", 0))
                .with_separator(SeparatorStyle::None)
                .with_keybindings(vec![
                    ("Ctrl+K", "command palette"),
                    ("Ctrl+P", "command palette"),
                    ("Esc", "close palette"),
                ]),
        ],
    );
}

/// **About pane** — version + dependency readouts plus a feature
/// chip cluster that demonstrates the auto-growing tags pod.
fn about_pane(body: &mut PaneBody) {
    let accent = body.accent();
    body.add_normal(
        cid(PANE_ABOUT, "info"),
        "bevy_frost",
        "info",
        vec![
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
    );
    body.add_normal(
        cid(PANE_ABOUT, "features"),
        "Features",
        "tag",
        vec![
            Pod::new(pid(PANE_ABOUT, "features", 0))
                .with_separator(SeparatorStyle::None)
                .with_tag_items(
                    vec![
                        frost_core::pod::TagItem::new("widgets"),
                        frost_core::pod::TagItem::new("ribbons"),
                        frost_core::pod::TagItem::new("panes"),
                        frost_core::pod::TagItem::new("pods"),
                        frost_core::pod::TagItem::new("graph-graph"),
                        frost_core::pod::TagItem::new("code-editor"),
                        frost_core::pod::TagItem::new("theme/PRO"),
                        frost_core::pod::TagItem::new("theme/GAME"),
                        frost_core::pod::TagItem::new("theme/FLAT"),
                        frost_core::pod::TagItem::colored(
                            "experimental",
                            frost_core::style::WARNING,
                        ),
                        frost_core::pod::TagItem::colored("stable-api", frost_core::style::SUCCESS),
                    ],
                    accent,
                ),
        ],
    );
    body.add_normal(
        cid(PANE_ABOUT, "stats"),
        "Stage stats",
        "info",
        vec![
            Pod::new(pid(PANE_ABOUT, "stats", 0))
                .with_separator(SeparatorStyle::Line)
                .with_badge_row("lights", vec!["12 dir", "4 pt", "2 spot", "1 dome"], accent),
            Pod::new(pid(PANE_ABOUT, "stats", 1))
                .with_separator(SeparatorStyle::Line)
                .with_badge_row("instances", vec!["3 proto", "128 inst", "anim"], accent),
            Pod::new(pid(PANE_ABOUT, "stats", 2))
                .with_separator(SeparatorStyle::Line)
                .with_badge_row("skel", vec!["6 skel", "1 root", "84 bind"], accent),
            Pod::new(pid(PANE_ABOUT, "stats", 3))
                .with_separator(SeparatorStyle::Line)
                .with_badge_row("render", vec!["1 settings", "2 product", "3 var"], accent),
            Pod::new(pid(PANE_ABOUT, "stats", 4))
                .with_separator(SeparatorStyle::None)
                .with_badge_row_items(
                    "physics",
                    vec![
                        frost_core::pod::TagItem::new("1 scene"),
                        frost_core::pod::TagItem::new("12 rb"),
                        frost_core::pod::TagItem::colored("broken", frost_core::style::WARNING),
                    ],
                    accent,
                ),
        ],
    );
}

/// **Editor pane** — node graph (top) + code editor (bottom),
/// each in its own container with a fill pod so they soak up the
/// pane's available space. Mirrors the legacy demo's Editor pane,
/// now driven by the vendored `bevy_frost::extras` wrappers.
fn editor_pane<'spec>(
    body: &mut PaneBody<'_, 'spec>,
    node_view: &'spec mut NodeViewState,
    graph: &'spec mut Graph<GraphNode>,
    viewer: &'spec mut DemoViewer,
    backend: &'spec mut EframeNodeViewBackend<'_>,
) {
    let cid_graph = cid(PANE_EDITOR, "graph");
    let code_id = cid(PANE_EDITOR, "code_state");
    body.add_node_graph(
        cid_graph,
        "Node graph",
        "flowchart",
        node_view,
        graph,
        viewer,
        backend,
    );
    body.add_normal(
        cid(PANE_EDITOR, "code"),
        "Source",
        "code",
        vec![
            Pod::new(pid(PANE_EDITOR, "code", 0))
                .with_separator(SeparatorStyle::None)
                .fill()
                .with_code_editor(code_id, Syntax::rust(), DEFAULT_CODE),
        ],
    );
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

fn eval_output(graph: &Graph<GraphNode>, pin: &OutPin) -> f64 {
    match graph.get_node(pin.id.node) {
        Some(GraphNode::Number(v)) => *v,
        Some(GraphNode::Add) => {
            let mut sum = 0.0;
            for i in 0..2 {
                let in_pin = graph.in_pin(InPinId {
                    node: pin.id.node,
                    input: i,
                });
                for remote in &in_pin.remotes {
                    let out_pin = graph.out_pin(*remote);
                    sum += eval_output(graph, &out_pin);
                }
            }
            sum
        }
        _ => 0.0,
    }
}

fn eval_input(graph: &Graph<GraphNode>, pin: &InPin) -> f64 {
    pin.remotes
        .iter()
        .map(|r| eval_output(graph, &graph.out_pin(*r)))
        .sum()
}

#[derive(Default)]
struct DemoViewer;

impl NodeViewer<GraphNode> for DemoViewer {
    fn title(&mut self, n: &GraphNode) -> String {
        n.title().into()
    }
    fn inputs(&mut self, n: &GraphNode) -> usize {
        n.inputs()
    }
    fn outputs(&mut self, n: &GraphNode) -> usize {
        n.outputs()
    }
    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut egui::Ui,
        graph: &mut Graph<GraphNode>,
    ) -> impl NodePin + 'static {
        match graph.get_node(pin.id.node) {
            Some(GraphNode::Add) => {
                let name = if pin.id.input == 0 { "a" } else { "b" };
                if pin.remotes.is_empty() {
                    ui.label(format!("{name} = 0"));
                } else {
                    ui.label(format!("{name} = {:.2}", eval_input(graph, pin)));
                }
            }
            Some(GraphNode::Output) => {
                let v = eval_input(graph, pin);
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
        graph: &mut Graph<GraphNode>,
    ) -> impl NodePin + 'static {
        if let Some(GraphNode::Number(v)) = graph.get_node_mut(pin.id.node) {
            ui.add(egui::DragValue::new(v).speed(0.05).fixed_decimals(2));
        } else if let Some(GraphNode::Add) = graph.get_node(pin.id.node) {
            let v = eval_output(graph, pin);
            ui.label(format!("= {v:.3}"));
        }
        PinInfo::circle()
    }
    fn has_graph_menu(&mut self, _: egui::Pos2, _: &mut Graph<GraphNode>) -> bool {
        true
    }
    fn show_graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        graph: &mut Graph<GraphNode>,
    ) {
        ui.label("Add node");
        if ui.button("Number").clicked() {
            graph.insert_node(pos, GraphNode::Number(0.0));
            ui.close();
        }
        if ui.button("Add").clicked() {
            graph.insert_node(pos, GraphNode::Add);
            ui.close();
        }
        if ui.button("Output").clicked() {
            graph.insert_node(pos, GraphNode::Output);
            ui.close();
        }
    }
}

fn default_graph() -> Graph<GraphNode> {
    let mut g = Graph::new();
    let a = g.insert_node(egui::pos2(30.0, 40.0), GraphNode::Number(2.0));
    let b = g.insert_node(egui::pos2(30.0, 130.0), GraphNode::Number(3.0));
    let add = g.insert_node(egui::pos2(220.0, 80.0), GraphNode::Add);
    let out = g.insert_node(egui::pos2(420.0, 80.0), GraphNode::Output);
    g.connect(
        OutPinId { node: a, output: 0 },
        InPinId {
            node: add,
            input: 0,
        },
    );
    g.connect(
        OutPinId { node: b, output: 0 },
        InPinId {
            node: add,
            input: 1,
        },
    );
    g.connect(
        OutPinId {
            node: add,
            output: 0,
        },
        InPinId {
            node: out,
            input: 0,
        },
    );
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
    (
        "/World",
        "World",
        "folder",
        &["/World/Robot", "/World/Lights"],
        egui::Color32::from_rgb(0x55, 0x6E, 0x9C),
    ),
    (
        "/World/Robot",
        "Robot",
        "person",
        &["/World/Robot/base", "/World/Robot/arm"],
        egui::Color32::from_rgb(0xE0, 0x6C, 0x4F),
    ),
    (
        "/World/Robot/base",
        "base",
        "code",
        &[],
        egui::Color32::from_rgb(0x4D, 0xA8, 0xDA),
    ),
    (
        "/World/Robot/arm",
        "arm",
        "code",
        &["/World/Robot/arm/grip"],
        egui::Color32::from_rgb(0xE6, 0xB7, 0x3D),
    ),
    (
        "/World/Robot/arm/grip",
        "grip",
        "code",
        &[],
        egui::Color32::from_rgb(0x9C, 0x55, 0xC0),
    ),
    (
        "/World/Lights",
        "Lights",
        "image",
        &["/World/Lights/sun"],
        egui::Color32::from_rgb(0xF5, 0xC2, 0x42),
    ),
    (
        "/World/Lights/sun",
        "sun",
        "image",
        &[],
        egui::Color32::from_rgb(0xFF, 0xE5, 0x6B),
    ),
];

fn demo_tree_node(path: &str) -> Option<&'static DemoTreeRow> {
    DEMO_TREE.iter().find(|(p, _, _, _, _)| *p == path)
}

fn demo_tree(
    tree: &mut frost_core::widget::TreeBody,
    root_id: egui::Id,
    accent: egui::Color32,
    filter: &str,
) {
    let sel_key = root_id.with("frost_demo_tree_selected");
    let mut selected: String = tree
        .ctx()
        .data(|d| d.get_temp::<String>(sel_key))
        .unwrap_or_default();
    let initial_selected = selected.clone();
    let mut frame_clicked: Option<String> = None;
    walk_demo_tree(
        tree,
        root_id,
        "/World",
        0,
        &selected,
        accent,
        filter,
        &mut frame_clicked,
    );
    if let Some(p) = frame_clicked {
        selected = p;
    }
    if selected != initial_selected {
        tree.ctx().data_mut(|d| d.insert_temp(sel_key, selected));
    }
}

/// Does this node — or any descendant — match the (lowercase)
/// substring `filter`? Branches stay visible when any child passes
/// so the path to a matching leaf never gets hidden by the parent
/// chain. Empty filter passes everything.
fn demo_tree_passes(path: &'static str, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let Some((p, name, _, children, _)) = demo_tree_node(path) else {
        return false;
    };
    if name.to_lowercase().contains(filter) || p.to_lowercase().contains(filter) {
        return true;
    }
    children.iter().any(|c| demo_tree_passes(c, filter))
}

fn walk_demo_tree(
    tree: &mut frost_core::widget::TreeBody,
    root_id: egui::Id,
    path: &'static str,
    depth: u32,
    selected: &str,
    accent: egui::Color32,
    filter: &str,
    clicked: &mut Option<String>,
) {
    let Some((p, name, icon, children, material)) = demo_tree_node(path) else {
        return;
    };
    if !demo_tree_passes(path, filter) {
        return;
    }
    let is_branch = !children.is_empty();
    let exp_key = root_id.with(("frost_demo_tree_expanded", *p));
    let eye_key = root_id.with(("frost_demo_tree_eye", *p));
    let lock_key = root_id.with(("frost_demo_tree_lock", *p));
    let mut expanded: bool = tree
        .ctx()
        .data_mut(|d| d.get_persisted::<bool>(exp_key))
        .unwrap_or(true);
    let mut eye_on: bool = tree
        .ctx()
        .data_mut(|d| d.get_persisted::<bool>(eye_key))
        .unwrap_or(true);
    let mut lock_on: bool = tree
        .ctx()
        .data_mut(|d| d.get_persisted::<bool>(lock_key))
        .unwrap_or(false);
    let mut swatch_dummy = false;

    let mut slots = [
        TreeIconSlot::new(TreeIconKind::Eye, &mut eye_on).with_tooltip("Toggle visibility"),
        TreeIconSlot::new(TreeIconKind::Lock, &mut lock_on).with_tooltip("Toggle lock"),
        TreeIconSlot::new(TreeIconKind::Color(*material), &mut swatch_dummy)
            .with_tooltip("Material colour"),
    ];
    let resp = tree.row(
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

    tree.ctx().data_mut(|d| {
        d.insert_persisted(exp_key, expanded);
        d.insert_persisted(eye_key, eye_on);
        d.insert_persisted(lock_key, lock_on);
    });

    if is_branch && expanded {
        for child in *children {
            walk_demo_tree(
                tree,
                root_id,
                child,
                depth + 1,
                selected,
                accent,
                filter,
                clicked,
            );
        }
    }
}
