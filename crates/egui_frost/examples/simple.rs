//! Plain-egui showcase for `egui_frost` — an `eframe` app that
//! demonstrates the frost theme + widgets WITHOUT any Bevy
//! dependency. Compare with `bevy_frost`'s `demo` example, which
//! does the same against a Bevy App.
//!
//! Run with:
//! ```text
//! cargo run -p egui_frost --example demo
//! ```
//!
//! What it exercises:
//!
//! * [`apply_theme`] — called every frame from the app's `update`.
//!   De-dupes internally, so re-applying on a stable theme is
//!   cheap.
//! * [`set_glass_opacity`] — pushes the current opacity value into
//!   the shared atomic so `glass_alpha_*` helpers pick it up.
//! * Widgets: [`toggle`], [`pretty_slider`], [`drag_value`],
//!   [`wide_button`], [`section`], [`color_rgb`].
//! * Floating ribbon + panel with a right-rail button (the
//!   [`floating_window_for_item`] helper works identically to the
//!   Bevy version — same state types, same drawing code).
//!
//! State lives inside the `FrostApp` struct; this is the plain
//! egui pattern. When driving the same widgets from Bevy, the same
//! state lives in `Resource`s (bevy_frost adds the derive via the
//! crate's `bevy` feature).

use eframe::egui;

use egui_frost::prelude::*;
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

// ─── Ribbon + panel ids ────────────────────────────────────────────

const RIBBON_RIGHT: &str = "egui_frost_right";
const MENU_WIDGETS: &str = "egui_frost_widgets";
const MENU_THEME: &str = "egui_frost_theme";

const RIBBONS: &[RibbonDef] = &[RibbonDef {
    id: RIBBON_RIGHT,
    edge: RibbonEdge::Right,
    role: RibbonRole::Panel,
    mode: RibbonMode::TwoSided,
    draggable: true,
    accepts: &[],
}];

const RIBBON_ITEMS: &[RibbonItem] = &[
    RibbonItem {
        id: MENU_WIDGETS,
        ribbon: RIBBON_RIGHT,
        cluster: RibbonCluster::Start,
        slot: 0,
        glyph: "W",
        tooltip: "Widgets",
        child_ribbon: None,
    },
    RibbonItem {
        id: MENU_THEME,
        ribbon: RIBBON_RIGHT,
        cluster: RibbonCluster::Start,
        slot: 1,
        glyph: "T",
        tooltip: "Theme",
        child_ribbon: None,
    },
];

// ─── App state ─────────────────────────────────────────────────────

struct FrostApp {
    // Theme values — in a Bevy app these would be `Resource`s; in
    // a plain egui app they're just fields.
    accent: AccentColor,
    glass: GlassOpacity,

    // Demo widget values.
    power: bool,
    headlights: bool,
    throttle: f64,
    brake: f64,
    fuel: f32,

    // Ribbon state — identical types to the Bevy version. Without
    // the `bevy` feature they're plain structs; with it they also
    // derive `Resource`.
    open: RibbonOpen,
    placement: RibbonPlacement,
    drag: RibbonDrag,
    // `layout` + `ghost` aren't used by the static ribbon flow
    // this demo uses; `draw_assembly` covers everything.
}

impl Default for FrostApp {
    fn default() -> Self {
        Self {
            accent: AccentColor::default(),
            glass: GlassOpacity::default(),
            power: true,
            headlights: false,
            throttle: 0.35,
            brake: 0.0,
            fuel: 0.72,
            open: RibbonOpen::default(),
            placement: RibbonPlacement::default(),
            drag: RibbonDrag::default(),
        }
    }
}

impl eframe::App for FrostApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Theme must be re-applied every frame so changes to the
        // accent / opacity take effect. The function de-dupes
        // internally, so stable values cost nothing.
        apply_theme(ctx, self.accent, self.glass);
        set_glass_opacity(self.glass.0);

        // The central panel is our "scene" — bevy_frost's demo
        // renders a 3D viewport here; we just fill with the glass
        // panel colour so the floating windows have something to
        // sit on.
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(0x1A, 0x1A, 0x1C)))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.heading("egui_frost");
                    ui.label("Plain egui (no Bevy) · click the ribbon button on the right ➜");
                });
            });

        let accent_col = self.accent.0;
        // Destructure `self` into disjoint field borrows so the
        // panel closures (which mutate widget values) don't
        // conflict with the immutable `placement` borrow
        // `floating_window_for_item` needs. Each closure below
        // only captures the fields it actually uses.
        let FrostApp {
            accent,
            glass,
            power,
            headlights,
            throttle,
            brake,
            fuel,
            open,
            placement,
            drag,
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
                ctx,
                RIBBONS,
                RIBBON_ITEMS,
                placement,
                MENU_WIDGETS,
                "Widgets",
                egui::vec2(320.0, 480.0),
                &mut keep_open,
                accent_col,
                |pane| widgets_panel(pane, power, headlights, throttle, brake, fuel),
            );
        }
        if is_open(MENU_THEME) {
            floating_window_for_item(
                ctx,
                RIBBONS,
                RIBBON_ITEMS,
                placement,
                MENU_THEME,
                "Theme",
                egui::vec2(300.0, 280.0),
                &mut keep_open,
                accent_col,
                |pane| theme_panel(pane, accent, glass),
            );
        }
    }
}

// ─── Panels ────────────────────────────────────────────────────────
//
// Both panels must use `pane.section(...)` rather than raw `ui` —
// the `PaneBuilder` enforces the "every widget lives inside a
// container" rule at the type level, same as bevy_frost's demo.

fn widgets_panel(
    pane: &mut PaneBuilder,
    power: &mut bool,
    headlights: &mut bool,
    throttle: &mut f64,
    brake: &mut f64,
    fuel: &mut f32,
) {
    let accent = pane.accent();
    pane.section("demo_flags", "Flags", true, |ui| {
        toggle(ui, "power", power, accent);
        toggle(ui, "headlights", headlights, accent);
    });
    pane.section("demo_bars", "Bars", true, |ui| {
        pretty_slider(ui, "throttle", throttle, 0.0..=1.0, 2, "", accent);
        pretty_slider(ui, "brake", brake, 0.0..=1.0, 2, "", accent);
        pretty_progressbar_text(
            ui,
            "fuel",
            *fuel,
            &format!("{:.0}%", *fuel * 100.0),
            accent,
        );
    });
    pane.section("demo_buttons", "Buttons", false, |ui| {
        if wide_button(ui, "Refuel", accent).clicked() {
            *fuel = 1.0;
        }
    });
}

fn theme_panel(
    pane: &mut PaneBuilder,
    accent_res: &mut AccentColor,
    glass: &mut GlassOpacity,
) {
    let accent = pane.accent();
    pane.section("demo_accent", "Accent", true, |ui| {
        let c = accent_res.0;
        let mut rgb = [
            c.r() as f32 / 255.0,
            c.g() as f32 / 255.0,
            c.b() as f32 / 255.0,
        ];
        if color_rgb(ui, "accent", &mut rgb, accent).changed() {
            accent_res.0 = srgb_to_egui(rgb);
        }
        sub_caption(
            ui,
            "Changing this recolours every widget in the app.",
        );
    });
    pane.section("demo_glass", "Glass", true, |ui| {
        let mut v = glass.0 as f64;
        if pretty_slider(ui, "opacity", &mut v, 1.0..=100.0, 0, "%", accent).changed() {
            glass.0 = v.round().clamp(1.0, 100.0) as u8;
        }
        sub_caption(
            ui,
            "Lower values let the content behind each panel peek through.",
        );
    });
}
