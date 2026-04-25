//! One-shot egui theme setup — palette, typography, and a single
//! `apply_theme` function. **Framework-agnostic** — no bevy or
//! bevy_egui imports here. The `bevy_frost` crate wraps these in
//! Bevy `Resource`s + a `Plugin` that runs `apply_theme` every
//! frame; plain-egui callers call `apply_theme(ctx, accent,
//! opacity)` directly from their UI body.
//!
//! Palette + typography follow the 2024-2026 editor convergence
//! (Blender 4, UE5.4, Godot 4, Unity 6, Fleet). All values are
//! centralised here so individual panels never hard-code colours —
//! the full palette is published even if not every token has a
//! current caller, so new UI pulls from the same reference set.

#![allow(dead_code)]

// ─── Neutrals ───────────────────────────────────────────────────────
pub const BG_0_WINDOW: egui::Color32 = egui::Color32::from_rgb(0x1A, 0x1A, 0x1C);
pub const BG_1_PANEL:  egui::Color32 = egui::Color32::from_rgb(0x24, 0x24, 0x28);
pub const BG_2_RAISED: egui::Color32 = egui::Color32::from_rgb(0x2D, 0x2D, 0x32);
pub const BG_3_HOVER:  egui::Color32 = egui::Color32::from_rgb(0x38, 0x38, 0x3F);
pub const BG_4_INPUT:  egui::Color32 = egui::Color32::from_rgb(0x18, 0x18, 0x1A);

// ─── Glass opacity (slider-driven) ──────────────────────────────────
//
// One user-facing opacity knob, range 1..=100. Internally mapped to
// window opacity 80..=100 % so the UI never becomes so transparent
// it stops being readable. Card + group alphas scale proportionally
// via `CARD_FACTOR` / `GROUP_FACTOR` below.

use core::sync::atomic::{AtomicU8, Ordering};

/// Shadow copy of the current opacity value. Plain helper functions
/// (`section`, `floating_window`, etc.) read this to derive glass
/// alphas without plumbing state through every UI call. Hosts
/// (bevy_frost, egui_frost) are responsible for keeping it in sync
/// with their chosen source of truth — either call
/// [`set_glass_opacity`] every frame or on change.
static GLASS_OPACITY: AtomicU8 = AtomicU8::new(100);

/// Plain-data opacity value, range `1..=100`. With the `bevy`
/// crate feature enabled, this derives `Resource` so it slots
/// directly into a Bevy `App`. Without the feature, it's just a
/// plain struct.
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GlassOpacity(pub u8);

impl Default for GlassOpacity {
    fn default() -> Self { Self(100) }
}

/// Push a new opacity value into the shared atomic. Call every
/// frame before laying out UI (bevy_frost does this via a Bevy
/// system; egui_frost callers do it manually from their app's
/// update loop).
pub fn set_glass_opacity(value: u8) {
    GLASS_OPACITY.store(value.clamp(1, 100), Ordering::Relaxed);
}

/// Map the slider's `1..=100` onto a window opacity fraction in
/// `0.80..=1.00`. `1 → 0.80`, `100 → 1.00`, linear in between.
fn opacity_frac() -> f32 {
    let t = (GLASS_OPACITY.load(Ordering::Relaxed).max(1) as f32 - 1.0) / 99.0;
    0.80 + 0.20 * t
}

pub fn glass_alpha_window() -> u8 {
    (opacity_frac() * 255.0).round().clamp(0.0, 255.0) as u8
}
pub fn glass_alpha_card() -> u8 {
    let f = theme().glass_card_factor;
    (opacity_frac() * f * 255.0).round().clamp(0.0, 255.0) as u8
}
pub fn glass_alpha_group() -> u8 {
    let f = theme().glass_group_factor;
    (opacity_frac() * f * 255.0).round().clamp(0.0, 255.0) as u8
}

// ─── Glassy variants ────────────────────────────────────────────────
//
// Panel / card / group surfaces get progressive transparency so the
// 3D scene peeks through the stack, plus a very faint accent tint
// that shifts hue with the selection.
//
// Alphas are DECREASING with depth on purpose: the outermost panel
// holds almost all the opacity; each deeper layer only adds a small
// extra veil so overlap doesn't compound into "effectively solid".
// Opacity stacks as `1 − (1-a)·(1-b)·(1-c)`, so card+group ≈ 16 %
// on top of the panel — just enough to read as "another surface".
// Alphas are computed each frame from `GLASS_OPACITY` so the
// single UI slider (General Properties → Theme → opacity) drives
// every glass surface proportionally. See `glass_alpha_*()` below.
//
/// How much of the accent colour to blend into each glass fill,
/// kept as a fallback for callers reading the const directly.
/// `theme().glass_accent_tint` is the active value; this constant
/// matches the PRO profile's value so older code paths keep working
/// when the theme is PRO.
pub const GLASS_ACCENT_TINT:  f32 = 0.03;

/// Produce a glass-style fill: base RGB lightly tinted toward
/// `accent`, with the given alpha. Use with any `egui::Frame::fill`.
/// Uses *unmultiplied* alpha so the painted surface blends at
/// `alpha/255` opacity over the scene. The accent-tint fraction is
/// read from the active [`Theme`] — GAME-style themes can set it to
/// `0.0` to flatten the fill into a pure neutral tone.
pub fn glass_fill(base: egui::Color32, accent: egui::Color32, alpha: u8) -> egui::Color32 {
    let f = theme().glass_accent_tint;
    let blend = |a: u8, b: u8| ((a as f32) * (1.0 - f) + (b as f32) * f).round() as u8;
    egui::Color32::from_rgba_unmultiplied(
        blend(base.r(), accent.r()),
        blend(base.g(), accent.g()),
        blend(base.b(), accent.b()),
        alpha,
    )
}

pub const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(0x0E, 0x0E, 0x10);
pub const BORDER_INNER:  egui::Color32 = egui::Color32::from_rgb(0x3A, 0x3A, 0x42);

/// The canonical border colour used by **every** frost surface —
/// foldable cards, sub-section frames, inputs, toggles, buttons.
/// The base colour, accent-tint fraction, and alpha all come from
/// the active [`Theme`]; PRO blends 6 % of accent over a near-black
/// at α 230, GAME pushes alpha to 0 so no border is drawn at all.
pub fn widget_border(accent: egui::Color32) -> egui::Color32 {
    let th = theme();
    let t = th.border_accent_tint;
    let blend = |base: u8, acc: u8| {
        ((base as f32) * (1.0 - t) + (acc as f32) * t).round() as u8
    };
    egui::Color32::from_rgba_unmultiplied(
        blend(th.border_subtle.r(), accent.r()),
        blend(th.border_subtle.g(), accent.g()),
        blend(th.border_subtle.b(), accent.b()),
        th.border_alpha,
    )
}

// ─── Text ───────────────────────────────────────────────────────────
pub const TEXT_PRIMARY:   egui::Color32 = egui::Color32::from_rgb(0xE6, 0xE6, 0xE8);
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(0x9A, 0x9A, 0xA2);
pub const TEXT_DISABLED:  egui::Color32 = egui::Color32::from_rgb(0x5A, 0x5A, 0x62);

// ─── Accent (selection / focus) — violet / purple ──────────────────
pub const ACCENT:         egui::Color32 = egui::Color32::from_rgb(0xA7, 0x8B, 0xFA);
pub const ACCENT_HOVER:   egui::Color32 = egui::Color32::from_rgb(0xC4, 0xB5, 0xFD);
pub const ACCENT_PRESSED: egui::Color32 = egui::Color32::from_rgb(0x8B, 0x5C, 0xF6);
/// Subtle purple-tinted surface for the active side button and the
/// selected outliner row. 18 % of `ACCENT` over `BG_2_RAISED`.
pub const ACCENT_TINT:    egui::Color32 = egui::Color32::from_rgb(0x42, 0x3A, 0x5A);
pub const SELECTION_ROW:  egui::Color32 = egui::Color32::from_rgb(0x4A, 0x3C, 0x72);

// ─── Axes (vivid: gizmos + inspector labels) ────────────────────────
pub const AXIS_X: egui::Color32 = egui::Color32::from_rgb(0xE0, 0x43, 0x3B);
pub const AXIS_Y: egui::Color32 = egui::Color32::from_rgb(0x7F, 0xB4, 0x35);
pub const AXIS_Z: egui::Color32 = egui::Color32::from_rgb(0x2E, 0x83, 0xE6);

// ─── Status ─────────────────────────────────────────────────────────
pub const SUCCESS: egui::Color32 = egui::Color32::from_rgb(0x34, 0xC7, 0x59);
pub const WARNING: egui::Color32 = egui::Color32::from_rgb(0xF5, 0xA5, 0x24);
pub const DANGER:  egui::Color32 = egui::Color32::from_rgb(0xEF, 0x44, 0x44);

/// Plain-data accent colour. With the `bevy` crate feature
/// enabled, this derives `Resource` so it can be used directly as
/// a Bevy state type.
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AccentColor(pub egui::Color32);

/// Neutral accent used when no vehicle is selected.
pub const ACCENT_NEUTRAL: egui::Color32 = egui::Color32::from_rgb(0xE6, 0xE6, 0xE8);

impl Default for AccentColor {
    fn default() -> Self { Self(ACCENT_NEUTRAL) }
}

// ─── Embedded UI font ───────────────────────────────────────────────
//
// Iosevka Term Light baked into the binary via `include_bytes!` — no
// `assets/` directory needs to ship alongside the executable. Face 0
// of the upstream `SGr-IosevkaTerm-Light.ttc`, subset to Latin +
// common symbol blocks (~1.3 MB).
//
// We deliberately stick with the stock egui font families
// (`Proportional` + `Monospace`) and do NOT register `FontFamily::Name`
// variants: `ctx.set_fonts` only takes effect on the NEXT `begin_pass`,
// and bevy_egui 0.39 spawns the primary egui context entity late
// enough that we can't race ahead of frame 0's draw. Looking up an
// unbound `FontFamily::Name("…")` on frame 0 is a hard panic in
// epaint, so we give up per-text weight selection and use size +
// colour + `.strong()` for hierarchy instead.

const IOSEVKA_LIGHT_TTF: &[u8] = include_bytes!("fonts/iosevka-light.ttf");

/// Replace egui's stock body fonts with Iosevka Light **and**
/// register every iconflow Fluent UI font variant as a named
/// family. After this call, widgets can paint Fluent icons via
/// `crate::icons::icon_text(name, size, color)` or look up the
/// glyph + family with `crate::icons::icon(name)`.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "iosevka-light".into(),
        std::sync::Arc::new(egui::FontData::from_static(IOSEVKA_LIGHT_TTF)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "iosevka-light".into());
    }
    crate::icons::install_iconflow_fonts(&mut fonts);
    ctx.set_fonts(fonts);
}

/// Apply the frost theme to the given egui context. Pure egui —
/// no framework deps. Hosts call this once per frame (bevy_frost
/// does it from a system; egui_frost callers call it from their
/// `update` / `show` body). The function de-dupes internally via a
/// static cache so re-calling with the same `(accent, opacity)`
/// skips the `ctx.set_style` / `ctx.set_fonts` work.
pub fn apply_theme(ctx: &egui::Context, accent: AccentColor, opacity: GlassOpacity) {
    use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize};

    // Packed (r, g, b, a) cache. `u32::MAX` is used as the
    // "never-applied" sentinel — no real colour hashes to that,
    // so the first call always passes the dedup check.
    static LAST_ACCENT: AtomicU32 = AtomicU32::new(u32::MAX);
    static LAST_OPACITY: AtomicU8 = AtomicU8::new(0);
    static LAST_THEME_NAME_PTR: AtomicUsize = AtomicUsize::new(0);
    static FONTS_INSTALLED: AtomicBool = AtomicBool::new(false);

    let th = theme();
    let accent_col = accent.0;
    if !FONTS_INSTALLED.load(Ordering::Relaxed) {
        install_fonts(ctx);
        FONTS_INSTALLED.store(true, Ordering::Relaxed);
    }

    // Pack the accent Color32 as u32: (r << 24) | (g << 16) | (b << 8) | a.
    let packed = ((accent_col.r() as u32) << 24)
        | ((accent_col.g() as u32) << 16)
        | ((accent_col.b() as u32) << 8)
        | (accent_col.a() as u32);
    // Use the `&'static str` pointer as the theme identity — names
    // are interned `&'static str`s built from string literals, so
    // pointer equality matches name equality for built-ins and any
    // user theme using a literal.
    let theme_ptr = th.name.as_ptr() as usize;
    if LAST_ACCENT.load(Ordering::Relaxed) == packed
        && LAST_OPACITY.load(Ordering::Relaxed) == opacity.0
        && LAST_THEME_NAME_PTR.load(Ordering::Relaxed) == theme_ptr
    {
        return;
    }
    LAST_ACCENT.store(packed, Ordering::Relaxed);
    LAST_OPACITY.store(opacity.0, Ordering::Relaxed);
    LAST_THEME_NAME_PTR.store(theme_ptr, Ordering::Relaxed);
    // Push into the shared atomics so glass-alpha + contrast-text
    // helpers can read these without callers having to thread them
    // through every widget signature.
    set_glass_opacity(opacity.0);
    set_active_accent(accent_col);

    // Glass variants of every neutral bg, so EVERY egui widget that
    // pulls from `Visuals` (buttons, inputs, sliders, text fields,
    // combo boxes, progress bars, ...) inherits the look from the
    // active theme automatically. `pane_fill` / `section_fill`
    // resolve the panel/section ColorMode so the GAME profile's
    // accent-derived panel actually flows into Visuals.panel_fill.
    let glass_panel = glass_fill(pane_fill(accent_col), accent_col, glass_alpha_window());
    let glass_card  = glass_fill(section_fill(accent_col), accent_col, glass_alpha_card());
    let glass_hover = glass_fill(th.bg_hover, accent_col, glass_alpha_card());

    let unified_border = widget_border(accent_col);
    let stroke_w = th.border_width;

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill          = glass_panel;
    visuals.window_fill         = glass_panel;
    visuals.window_stroke       = egui::Stroke::new(stroke_w, unified_border);
    // `extreme_bg_color` is the egui visual every native input
    // (DragValue, TextEdit, ScrollArea track, …) pulls from. Route
    // it through `track_fill` so PRO keeps the dark sunken look and
    // GAME blends into the accent panel.
    visuals.extreme_bg_color    = track_fill(accent_col);
    visuals.faint_bg_color      = glass_card;
    visuals.code_bg_color       = glass_card;
    visuals.override_text_color = Some(th.text_primary);
    visuals.selection.bg_fill   = tinted_surface(accent_col);
    visuals.selection.stroke    = egui::Stroke::new(stroke_w.max(1.0), accent_col);
    visuals.hyperlink_color     = accent_col;

    let r = egui::CornerRadius::same(th.radius_widget);
    let widget = |bg: egui::Color32, fg_stroke: egui::Color32, bg_stroke: egui::Color32| {
        egui::style::WidgetVisuals {
            bg_fill: bg,
            weak_bg_fill: bg,
            bg_stroke: egui::Stroke::new(stroke_w, bg_stroke),
            fg_stroke: egui::Stroke::new(1.0, fg_stroke),
            corner_radius: r,
            expansion: 0.0,
        }
    };
    // Native egui interactive widgets (Button, DragValue,
    // Checkbox, RadioButton, ComboBox header, …) all paint their
    // background from `widgets.inactive.bg_fill` / `weak_bg_fill`.
    // Routing it through `track_fill` keeps these inputs at the
    // same brightness tier as the frost search field / dropdown
    // trigger / slider track instead of dropping to the dark
    // `bg_raised` panel colour. PRO unchanged (track_fill returns
    // `bg_input`); GAME now lifts inputs to `panel + 10 % white`.
    let input_bg = track_fill(accent_col);
    let glass_input = glass_fill(input_bg, accent_col, glass_alpha_card());
    visuals.widgets.noninteractive = widget(glass_panel, th.text_secondary, unified_border);
    visuals.widgets.inactive       = widget(glass_input, th.text_primary,   unified_border);
    visuals.widgets.hovered        = widget(glass_hover, th.text_primary,   th.border_inner);
    visuals.widgets.active         = widget(accent_col,  th.text_primary,   accent_col);
    visuals.widgets.open           = widget(glass_hover, th.text_primary,   th.border_inner);

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;

    // Slightly roomier controls — interacts at 20 px (was 18) and
    // buttons get 8×4 padding (was 6×2) so rows don't feel cramped
    // against each other.
    style.spacing.item_spacing      = egui::vec2(6.0, 3.0);
    style.spacing.button_padding    = egui::vec2(8.0, 4.0);
    style.spacing.indent            = 14.0;
    style.spacing.window_margin     = egui::Margin::ZERO;
    style.spacing.interact_size.y   = 20.0;
    // Tight slider track. Combined with no inline `.text(...)` label
    // and no `.show_value()` suffix, this leaves enough right-cell
    // space for the slider PLUS the current value without pushing
    // the section card wider than its pinned inner width.
    style.spacing.slider_width      = 90.0;
    style.spacing.icon_width        = 14.0;
    style.spacing.icon_spacing      = 6.0;

    // Scrollbar tinting — flip to `foreground_color` mode so the
    // handle picks up each state's `fg_stroke.color` instead of its
    // `bg_fill`. That lets us paint scrollbars in accent variants
    // (rest / hover / drag) without touching every OTHER widget's
    // `bg_fill` (which would re-tint buttons, inputs, frames, etc.
    // at the same time).
    style.spacing.scroll.foreground_color = true;
    // Rest: a dimmed-accent track handle that still belongs to the
    // accent family. Hover: full ACCENT_HOVER. Drag: ACCENT_PRESSED.
    // `fg_stroke` is also used for fine foreground elements
    // (checkmarks, focus rings) — re-tinting them to accent reads as
    // an improvement, not a regression.
    let accent_dim = egui::Color32::from_rgba_unmultiplied(
        accent_col.r(),
        accent_col.g(),
        accent_col.b(),
        160,
    );
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, accent_dim);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, accent_hover());
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, accent_pressed());
    style.text_styles = [
        (egui::TextStyle::Heading,   egui::FontId::new(16.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Body,      egui::FontId::new(13.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Monospace, egui::FontId::new(13.0, egui::FontFamily::Monospace)),
        (egui::TextStyle::Button,    egui::FontId::new(13.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Small,     egui::FontId::new(12.0, egui::FontFamily::Proportional)),
    ]
    .into();

    ctx.set_style(style);
}

/// Darker/muted version of an accent colour — used for "selected" row
/// fills where the full-strength accent would be too loud.
fn tinted_surface(c: egui::Color32) -> egui::Color32 {
    // 35 % of accent over the active theme's raised background.
    let bg = theme().bg_raised;
    let f = 0.35;
    let lerp = |a: u8, b: u8| ((a as f32) * (1.0 - f) + (b as f32) * f).round() as u8;
    egui::Color32::from_rgb(
        lerp(bg.r(), c.r()),
        lerp(bg.g(), c.g()),
        lerp(bg.b(), c.b()),
    )
}

/// Convert a linear-sRGB `[f32;3]` to an egui [`egui::Color32`].
/// Matches the visual tone a wgpu 3D view renders — handy when you
/// want the egui UI accent to match a material colour from the scene.
pub fn srgb_to_egui(rgb: [f32; 3]) -> egui::Color32 {
    let to_u8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    egui::Color32::from_rgb(to_u8(rgb[0]), to_u8(rgb[1]), to_u8(rgb[2]))
}

/// Uppercase accent section header. Used both by left panels
/// (`CollapsingHeader::new(section_caps(…))`) and by the right
/// inspector — keeps the visual language identical on both sides.
///
/// Pops against the Light body font via `.strong()` (darker render)
/// + caps + accent colour. Per-weight font selection isn't available:
/// see the comment on the embedded-font block above for why.
///
/// Size: 12 pt body baseline + 15 % bump so section titles read
/// clearly larger than body copy inside the same card.
pub fn section_caps(label: &str, accent: egui::Color32) -> egui::RichText {
    egui::RichText::new(label.to_uppercase())
        .strong()
        .size(12.0 * 1.15)
        .color(accent)
}

pub fn fg_dim() -> egui::Color32 { TEXT_SECONDARY }

// ─── Design-system tokens ────────────────────────────────────────────
//
// Every panel should lay out against THESE instead of ad-hoc `add_space`
// calls. Keeps rhythm consistent and lets the whole UI be re-tuned
// from one place. Scale is a 4 px grid; sizes are named by use, not
// by pixel count, so the numbers can evolve without a find-and-replace.

pub mod space {
    /// Between tightly-related items inside one row (label↔chip, glyph↔text).
    pub const TIGHT: f32 = 2.0;
    /// Between adjacent rows inside one section (label rows, slider rows).
    pub const ROW: f32 = 2.0;
    /// Between a row and a sub-block inside one section.
    pub const BLOCK: f32 = 4.0;
    /// Between distinct section cards in a panel. Slight gap so the
    /// rounded frames don't kiss each other edge-to-edge.
    pub const SECTION: f32 = 1.0;
}

pub mod radius {
    /// Default radius for every in-panel control — sliders,
    /// progress bars, buttons, number inputs, colour pickers,
    /// combo boxes, key chips. Tuned against a *long* widget
    /// (progress bar / slider) where 2 px reads as subtly
    /// rounded.
    pub const WIDGET: u8 = 2;
    /// Radius for **compact** controls — toggles and anything
    /// else whose footprint is close to square or very short.
    /// At short widths a 2 px corner looks square-cut; a slightly
    /// larger radius compensates so the perceived roundness
    /// matches the wider widgets'.
    pub const COMPACT: u8 = 3;
    /// Progress bars, chips, bars-within-rows.
    /// *(Legacy — prefer `WIDGET` for new code.)*
    pub const SM: u8 = 3;
    /// Foldable container cards. Larger than `WIDGET` so the
    /// container reads as a surface and the widgets inside read
    /// as controls on top of it.
    pub const MD: u8 = 6;
    /// Panels, pop-overs, the biggest floating surfaces.
    pub const LG: u8 = 8;
}

// ─── Theme — pluggable visual profile ───────────────────────────────
//
// Every value that varies between visual profiles (PRO ↔ GAME ↔ user
// custom) lives in this struct. Widgets read from `theme()`; users
// switch with `set_theme(...)`. Composing a third profile is one
// struct-update expression — no widget edits needed.
//
// Fields are deliberately concrete (not `Option`s) so `Theme` is
// `Copy` and `theme()` can hand out a value rather than a borrow.
// That keeps every widget call a plain field access with no lock /
// reference threading.

/// How a surface fill is computed. Either pulled straight out of
/// the theme palette ([`ColorMode::FromBg`] — the PRO behaviour) or
/// derived from the runtime accent colour by lerping from black
/// ([`ColorMode::FromAccent`] — the GAME behaviour, where the panel
/// itself takes on whatever colour the user set as accent).
#[derive(Copy, Clone, Debug)]
pub enum ColorMode {
    /// Use the corresponding `bg_*` field from the theme directly.
    FromBg,
    /// Compute as `lerp(Color32::BLACK, accent, lerp_factor)`. A
    /// factor of `0.65` produces a richly accent-toned panel, `0.85`
    /// is close to full accent, `0.0` is pure black.
    FromAccent { lerp_factor: f32 },
}

/// How the title text colour is picked. Lets a theme flip the
/// "accent text on dark panel" PRO recipe to "dark text on accent
/// panel" without per-panel code.
#[derive(Copy, Clone, Debug)]
pub enum TextColorMode {
    Accent,
    Primary,
    Secondary,
    /// Pick the colour that best contrasts whatever the active theme
    /// produces for the panel fill (luma-based via
    /// [`contrast_text_for`]). Use when the panel itself is bright
    /// (GAME's accent panel) so the title text stays readable.
    ContrastWithPanel,
    /// Same idea but contrasts against the section fill.
    ContrastWithSection,
}

/// A complete visual profile for the frost UI kit. Built-in
/// profiles: [`theme_pro`] (the default — soft glass, rounded
/// corners, accent-tinted titles on a dark panel) and [`theme_game`]
/// (square corners, no borders, no padding, accent-coloured panel
/// with contrasting dark titles, full-accent click fills).
#[derive(Copy, Clone, Debug)]
pub struct Theme {
    /// Identifier used by the de-dup cache in [`apply_theme`] — pick
    /// distinct names for distinct themes or the egui style won't
    /// re-apply on switch.
    pub name: &'static str,

    // ── Surfaces — palette ──
    pub bg_window:  egui::Color32,
    pub bg_panel:   egui::Color32,
    pub bg_raised:  egui::Color32,
    pub bg_hover:   egui::Color32,
    pub bg_input:   egui::Color32,

    // ── Surfaces — fill mode ──
    /// How [`pane_fill`] resolves. PRO uses `FromBg` (dark panel);
    /// GAME uses `FromAccent` so the entire pane takes the user's
    /// accent colour.
    pub panel_fill_mode:   ColorMode,
    /// How [`section_fill`] resolves. Only consulted when
    /// `section_show_frame` is true.
    pub section_fill_mode: ColorMode,
    /// `true` → sections paint a visible frame (fill + border + corner
    /// rounding) the way they always have. `false` → sections render
    /// no frame at all and the body content sits directly on the
    /// panel — the "no container" GAME look.
    pub section_show_frame: bool,
    /// PRO: 1 px hairline under the section header. GAME: false.
    pub section_show_title_divider: bool,
    /// Inner padding inside section / subsection frames.
    /// PRO: (4, 3); GAME: (0, 0).
    pub section_pad_x: i8,
    pub section_pad_y: i8,
    /// Horizontal indent applied to the section body content so it
    /// visually nests under the title rather than sitting flush at
    /// the same X. PRO ≈ 8 px, GAME ≈ 6 px — both themes get a
    /// distinct "body is inside" cue without the title needing to
    /// be cramped against the frame edge.
    pub section_body_indent: f32,

    // ── Text ──
    pub text_primary:   egui::Color32,
    pub text_secondary: egui::Color32,
    pub text_disabled:  egui::Color32,
    /// How the section / pane title colour is resolved.
    pub title_color_mode: TextColorMode,

    // ── Borders / strokes ──
    /// Base border colour (before the accent tint blend).
    pub border_subtle:      egui::Color32,
    /// Inner-frame stroke colour for hover / active states.
    pub border_inner:       egui::Color32,
    /// Alpha applied to [`widget_border`] strokes.
    pub border_alpha:       u8,
    /// Fraction of the accent colour blended into [`widget_border`].
    pub border_accent_tint: f32,
    /// Stroke width used for every frost surface (sections,
    /// subsections, group frames, inputs, …). `0.0` paints no border
    /// at all — handy for the GAME profile.
    pub border_width:       f32,

    // ── Glass ──
    /// Card alpha as a fraction of window alpha. PRO ≈ 0.76; GAME
    /// can flatten this to 1.0 + a flat fill to drop the glass effect.
    pub glass_card_factor:  f32,
    /// Group alpha as a fraction of window alpha.
    pub glass_group_factor: f32,
    /// Fraction of the accent colour blended into glass surfaces.
    /// `0.0` produces a pure neutral fill — matches the flat,
    /// posterised look of game UIs.
    pub glass_accent_tint:  f32,

    // ── Shape ──
    pub radius_widget:  u8,
    pub radius_compact: u8,
    pub radius_sm:      u8,
    pub radius_md:      u8,
    pub radius_lg:      u8,

    // ── Body row visuals ──
    /// PRO: false. GAME: true → reserved for an alternating-fill
    /// pattern on row-level widgets so a borderless stack still reads
    /// as a list (helper landing in a follow-up).
    pub row_alternation: bool,

    // ── Click visuals ──
    /// `false` → press-state uses the subtle accent lerp the PRO
    /// theme has always done. `true` → pressed buttons fill solid
    /// with `accent`, no halftone, for the chunky GAME look.
    pub button_full_accent_on_press: bool,
    /// Accent-blend fraction for buttons at rest. PRO `0.08`, GAME
    /// `0.0` (flat panel under the button).
    pub button_tint_rest:  f32,
    /// Accent-blend fraction for buttons on hover. PRO `0.16`, GAME
    /// `0.18` (a touch more pop on the bright accent panel).
    pub button_tint_hover: f32,
    /// Accent-blend fraction for buttons while pressed (when
    /// `button_full_accent_on_press` is `false`). PRO `0.30`.
    pub button_tint_press: f32,

    // ── Pane chrome ──
    /// Shadow blur radius for the floating pane window. PRO `24`,
    /// GAME `0` (hard-edge no-shadow look).
    pub pane_shadow_blur:  u8,
    /// Shadow vertical offset. PRO `8`, GAME `0`.
    pub pane_shadow_y:     i8,
    /// Whether the pane title strip paints a 1 px hairline divider
    /// under the title. PRO true, GAME false.
    pub pane_show_title_divider: bool,

    // ── Tree / list visuals ──
    /// Width of the indent-guide line painted at each depth level
    /// of a tree. PRO `1.0`, GAME `0.0` (guides off — flat list).
    pub tree_guide_width: f32,
    /// Snarl graph pin stroke width. PRO `1.0`, GAME `0.0`.
    pub snarl_pin_width:  f32,

    // ── Drag-reorder ghost ──
    /// Alpha applied to the accent fill of the section/ribbon-button
    /// drag-ghost rect. PRO `28` (faint), GAME `90` (visible against
    /// the accent panel).
    pub ghost_fill_alpha:    u8,
    /// Stroke width on the drag-ghost rect's accent border. PRO
    /// `1.5`, GAME `0.0` (no stroke — fill alone reads).
    pub ghost_stroke_width:  f32,
}

/// Built-in PRO profile — the look this kit shipped with: glass
/// surfaces, rounded corners, subtle accent-tinted borders, dimmed
/// click halftone. Every value here matches the constants the kit
/// used before the theme system landed.
pub const fn theme_pro() -> Theme {
    Theme {
        name: "PRO",
        bg_window:  BG_0_WINDOW,
        bg_panel:   BG_1_PANEL,
        bg_raised:  BG_2_RAISED,
        bg_hover:   BG_3_HOVER,
        bg_input:   BG_4_INPUT,
        panel_fill_mode:    ColorMode::FromBg,
        section_fill_mode:  ColorMode::FromBg,
        section_show_frame: true,
        section_show_title_divider: true,
        section_pad_x: 4,
        section_pad_y: 3,
        section_body_indent: 8.0,
        text_primary:   TEXT_PRIMARY,
        text_secondary: TEXT_SECONDARY,
        text_disabled:  TEXT_DISABLED,
        title_color_mode: TextColorMode::Accent,
        border_subtle:      BORDER_SUBTLE,
        border_inner:       BORDER_INNER,
        border_alpha:       230,
        border_accent_tint: 0.06,
        border_width:       1.0,
        glass_card_factor:  0.76,
        glass_group_factor: 0.57,
        glass_accent_tint:  0.03,
        radius_widget:  radius::WIDGET,
        radius_compact: radius::COMPACT,
        radius_sm:      radius::SM,
        radius_md:      radius::MD,
        radius_lg:      radius::LG,
        row_alternation: false,
        button_full_accent_on_press: false,
        button_tint_rest:  0.08,
        button_tint_hover: 0.16,
        button_tint_press: 0.30,
        pane_shadow_blur:  24,
        pane_shadow_y:     8,
        pane_show_title_divider: true,
        tree_guide_width: 1.0,
        snarl_pin_width:  1.0,
        ghost_fill_alpha:   28,
        ghost_stroke_width: 1.5,
    }
}

/// Built-in GAME profile — the inverse of PRO: the **panel** itself
/// takes the user's accent colour (lerped 65 % toward black so it
/// stays readable), sections render with **no frame** at all so
/// content sits flush on the accent panel, the section title flips
/// to a luma-contrasted near-black/near-white, every corner is
/// square, every border is gone, padding is zero, and pressed
/// buttons fill solid with full accent. Palette colours are kept
/// (used by hover / input states) but most surfaces derive from
/// accent at runtime.
pub const fn theme_game() -> Theme {
    Theme {
        name: "GAME",
        bg_window:  egui::Color32::from_rgb(0x08, 0x0A, 0x12),
        bg_panel:   egui::Color32::from_rgb(0x10, 0x14, 0x1F),
        bg_raised:  egui::Color32::from_rgb(0x16, 0x1B, 0x29),
        bg_hover:   egui::Color32::from_rgb(0x1F, 0x26, 0x38),
        bg_input:   egui::Color32::from_rgb(0x06, 0x08, 0x0E),
        panel_fill_mode:   ColorMode::FromAccent { lerp_factor: 0.65 },
        section_fill_mode: ColorMode::FromBg,
        section_show_frame: false,
        section_show_title_divider: false,
        section_pad_x: 0,
        section_pad_y: 0,
        section_body_indent: 6.0,
        text_primary:   egui::Color32::from_rgb(0xF0, 0xF4, 0xFF),
        text_secondary: egui::Color32::from_rgb(0x9E, 0xA8, 0xC0),
        text_disabled:  egui::Color32::from_rgb(0x4A, 0x52, 0x66),
        title_color_mode: TextColorMode::ContrastWithPanel,
        border_subtle:      egui::Color32::from_rgb(0x06, 0x08, 0x0E),
        border_inner:       egui::Color32::from_rgb(0x1F, 0x26, 0x38),
        border_alpha:       0,
        border_accent_tint: 0.0,
        border_width:       0.0,
        glass_card_factor:  1.0,
        glass_group_factor: 1.0,
        glass_accent_tint:  0.0,
        radius_widget:  0,
        radius_compact: 0,
        radius_sm:      0,
        radius_md:      0,
        radius_lg:      0,
        row_alternation: true,
        button_full_accent_on_press: true,
        button_tint_rest:  0.0,
        button_tint_hover: 0.18,
        button_tint_press: 0.40,
        pane_shadow_blur:  0,
        pane_shadow_y:     0,
        pane_show_title_divider: false,
        tree_guide_width: 0.0,
        snarl_pin_width:  0.0,
        ghost_fill_alpha:   90,
        ghost_stroke_width: 0.0,
    }
}

/// Packed `(r, g, b, a)` snapshot of the active accent colour.
/// `apply_theme` writes this so widget paints can call
/// [`active_accent`] without threading the colour through every API.
static ACTIVE_ACCENT: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0xE6E6E8FF);

fn set_active_accent(c: egui::Color32) {
    let p = ((c.r() as u32) << 24)
        | ((c.g() as u32) << 16)
        | ((c.b() as u32) << 8)
        | (c.a() as u32);
    ACTIVE_ACCENT.store(p, Ordering::Relaxed);
}

/// Read the current accent colour. Hosts call [`apply_theme`] each
/// frame, which keeps this in sync. Widget code that already has
/// `accent` in scope should keep using it; this exists for helpers
/// (text-contrast pickers, theme-aware fills) called from sites
/// that don't thread accent through their signatures.
pub fn active_accent() -> egui::Color32 {
    let p = ACTIVE_ACCENT.load(Ordering::Relaxed);
    egui::Color32::from_rgba_premultiplied(
        ((p >> 24) & 0xff) as u8,
        ((p >> 16) & 0xff) as u8,
        ((p >> 8) & 0xff) as u8,
        (p & 0xff) as u8,
    )
}

/// Lazily-initialised storage for the active theme. Single-process
/// singleton, read on every widget paint.
static ACTIVE_THEME: std::sync::OnceLock<std::sync::RwLock<Theme>> =
    std::sync::OnceLock::new();

fn theme_lock() -> &'static std::sync::RwLock<Theme> {
    ACTIVE_THEME.get_or_init(|| std::sync::RwLock::new(theme_pro()))
}

/// Replace the active theme. Takes effect on the next paint —
/// frostcore's de-dup cache in [`apply_theme`] uses `theme.name` to
/// detect the switch and re-push the egui style. Call this when the
/// user picks a profile from a settings UI.
pub fn set_theme(t: Theme) {
    *theme_lock().write().unwrap() = t;
}

/// Return a copy of the active theme. `Theme` is `Copy`, so widgets
/// can call this freely — no lifetimes, no allocation. Reads are
/// `RwLock::read`; under typical UI contention (none) the cost is a
/// single relaxed atomic.
pub fn theme() -> Theme {
    *theme_lock().read().unwrap()
}

/// Resolve the active theme's [`ColorMode`] for a fill against the
/// runtime accent colour. Used by [`pane_fill`] / [`section_fill`].
fn resolve_color(mode: ColorMode, fallback: egui::Color32, accent: egui::Color32) -> egui::Color32 {
    match mode {
        ColorMode::FromBg => fallback,
        ColorMode::FromAccent { lerp_factor } => {
            let f = lerp_factor.clamp(0.0, 1.0);
            let lerp = |a: u8, b: u8| {
                ((a as f32) * (1.0 - f) + (b as f32) * f).round() as u8
            };
            egui::Color32::from_rgb(
                lerp(0, accent.r()),
                lerp(0, accent.g()),
                lerp(0, accent.b()),
            )
        }
    }
}

/// The opaque base fill colour for the floating pane window — what
/// `egui::Frame::fill` ultimately gets, modulo the glass alpha.
/// PRO returns `theme().bg_panel`; GAME returns an accent-derived
/// dark colour so the entire pane reads as "the user's accent".
pub fn pane_fill(accent: egui::Color32) -> egui::Color32 {
    let th = theme();
    resolve_color(th.panel_fill_mode, th.bg_panel, accent)
}

/// The opaque base fill colour for a section card. Only consulted
/// when `theme().section_show_frame` is `true`. PRO returns
/// `theme().bg_raised`; GAME falls through to its `bg_raised` when
/// frame paint is enabled at all.
pub fn section_fill(accent: egui::Color32) -> egui::Color32 {
    let th = theme();
    resolve_color(th.section_fill_mode, th.bg_raised, accent)
}

/// Resolve the active theme's title colour against the runtime
/// accent. PRO maps to `accent` (the title literally tints with the
/// user's chosen accent); GAME maps via [`contrast_text_for`] over
/// the resolved panel fill, so a bright accent panel shows
/// near-black titles and a dark panel shows near-white.
pub fn section_title_color(accent: egui::Color32) -> egui::Color32 {
    let th = theme();
    match th.title_color_mode {
        TextColorMode::Accent => accent,
        TextColorMode::Primary => th.text_primary,
        TextColorMode::Secondary => th.text_secondary,
        TextColorMode::ContrastWithPanel => contrast_text_for(pane_fill(accent)),
        TextColorMode::ContrastWithSection => contrast_text_for(section_fill(accent)),
    }
}

/// `egui::Margin` used by section / subsection / group inner frames,
/// driven by the theme's `section_pad_x/y`. GAME → `Margin::ZERO`.
pub fn section_padding() -> egui::Margin {
    let th = theme();
    egui::Margin::symmetric(th.section_pad_x, th.section_pad_y)
}

/// Whether the section header should paint a 1 px hairline divider
/// between its title and body. Mirrors `theme().section_show_title_divider`.
pub fn section_show_title_divider() -> bool {
    theme().section_show_title_divider
}

/// Whether sections should paint their own frame (fill + border +
/// corner rounding). When `false`, the section's `egui::Frame` paint
/// is skipped entirely and the body content renders directly on the
/// pane background — the GAME "no card" look.
pub fn section_show_frame() -> bool {
    theme().section_show_frame
}

/// Linear RGB blend of two colours by `t` in `[0, 1]`. Internal
/// helper for theme-aware fill resolvers.
fn lerp_rgb(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let f = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| ((x as f32) * (1.0 - f) + (y as f32) * f).round() as u8;
    egui::Color32::from_rgb(
        lerp(a.r(), b.r()),
        lerp(a.g(), b.g()),
        lerp(a.b(), b.b()),
    )
}

/// The neutral fill used for "track" surfaces — slider / progress
/// bar unfilled tracks, toggle pill OFF state, search field
/// background, dropdown trigger, popup fill, and any DragValue /
/// TextEdit input (via `Visuals.extreme_bg_color`).
///
/// PRO returns the dark `bg_input` palette colour (a sunken-input
/// look against the lighter card / panel surfaces). GAME picks a
/// shade slightly DARKER than the panel itself — `lerp(BLACK,
/// accent, lerp_factor - 0.20)` where `lerp_factor` is the panel's
/// own — so tracks read as a consistent "input on the accent
/// panel" tier rather than a near-black block sitting on a bright
/// accent.
pub fn track_fill(accent: egui::Color32) -> egui::Color32 {
    let th = theme();
    match th.panel_fill_mode {
        ColorMode::FromAccent { lerp_factor } => {
            // Track sits one tier ABOVE the panel — `lerp(panel,
            // WHITE, 0.10)` so it always reads as a slightly
            // raised input regardless of how dark the user's
            // accent is. Going darker (the previous attempt)
            // produced near-black inputs on dark accents; lighter
            // is uniformly readable.
            let panel_color = lerp_rgb(egui::Color32::BLACK, accent, lerp_factor);
            lerp_rgb(panel_color, egui::Color32::WHITE, 0.10)
        }
        ColorMode::FromBg => th.bg_input,
    }
}

/// Fill colour for floating popup surfaces — dropdown lists, the
/// command palette, context menus. Sits above the panel as a
/// "raised" tier:
/// - PRO returns `bg_raised` (existing behaviour).
/// - GAME returns a shade halfway between the panel and the track
///   (≈ panel - 0.10 lerp), so the popup is distinguishable from
///   both but stays in the same accent family.
pub fn popup_fill(accent: egui::Color32) -> egui::Color32 {
    let th = theme();
    match th.panel_fill_mode {
        ColorMode::FromAccent { lerp_factor } => {
            // Popup sits one tier ABOVE the panel — `lerp(panel,
            // WHITE, 0.18)` so menus always read as raised against
            // whatever the user's accent ended up being.
            let panel_color = lerp_rgb(egui::Color32::BLACK, accent, lerp_factor);
            lerp_rgb(panel_color, egui::Color32::WHITE, 0.18)
        }
        ColorMode::FromBg => th.bg_raised,
    }
}

// ─── Theme-aware text colours ───────────────────────────────────────
//
// Six no-arg helpers covering the three frost surfaces (panel,
// section, track) × two intensities (primary / dim). Each picks a
// luma-contrasted text colour against the surface fill the active
// theme + the active accent produce, so a yellow accent on PRO and
// a pastel accent on GAME both stay readable. The "dim" variant
// blends the primary text 40 % toward the surface — same role
// `TEXT_SECONDARY` plays on a dark panel, just generalised.
//
// These read from [`active_accent`] so callers don't have to thread
// the accent through every widget signature. `apply_theme` keeps
// the active accent in sync each frame.

fn dim_against(text: egui::Color32, surface: egui::Color32) -> egui::Color32 {
    // 40 % blend toward the surface — close enough to the surface to
    // read as "secondary" hierarchy, far enough off to stay
    // legible. Matches the visual weight `TEXT_SECONDARY` (#9A) had
    // against `BG_1_PANEL` (#24).
    lerp_rgb(text, surface, 0.4)
}

/// Primary-weight text colour for paint directly on the pane fill.
/// Resolves via `contrast_text_for(pane_fill(active_accent))`.
pub fn on_panel() -> egui::Color32 {
    contrast_text_for(pane_fill(active_accent()))
}
/// Secondary-weight (~`TEXT_SECONDARY` role) version of [`on_panel`].
pub fn on_panel_dim() -> egui::Color32 {
    dim_against(on_panel(), pane_fill(active_accent()))
}

/// Primary-weight text colour for paint inside a section frame.
/// When the active theme has `section_show_frame = false`, falls
/// through to [`on_panel`] since the body content is now sitting on
/// the pane fill instead.
pub fn on_section() -> egui::Color32 {
    let acc = active_accent();
    if theme().section_show_frame {
        contrast_text_for(section_fill(acc))
    } else {
        contrast_text_for(pane_fill(acc))
    }
}
/// Secondary-weight version of [`on_section`].
pub fn on_section_dim() -> egui::Color32 {
    let acc = active_accent();
    let surface = if theme().section_show_frame {
        section_fill(acc)
    } else {
        pane_fill(acc)
    };
    dim_against(on_section(), surface)
}

/// Primary-weight text colour for paint on a track surface — search
/// field input, dropdown trigger label, slider/progress-bar readout
/// over the unfilled portion. Resolves via `contrast_text_for(track_fill(active_accent))`.
pub fn on_track() -> egui::Color32 {
    contrast_text_for(track_fill(active_accent()))
}
/// Secondary-weight version of [`on_track`] — placeholder hints,
/// trailing chrome glyphs, secondary readouts.
pub fn on_track_dim() -> egui::Color32 {
    dim_against(on_track(), track_fill(active_accent()))
}

/// Derived "hover" variant of the runtime accent — used by the
/// scrollbar's foreground colour for the handle's hover state, and
/// by any widget that wants a lighter accent for hover affordance.
/// Lerps the accent 25 % toward white. Replaces the legacy
/// hardcoded `ACCENT_HOVER` constant which never tracked the user's
/// chosen accent.
pub fn accent_hover() -> egui::Color32 {
    lerp_rgb(active_accent(), egui::Color32::WHITE, 0.25)
}

/// Derived "pressed" variant of the runtime accent — used by the
/// scrollbar's drag-state foreground and the code-editor selection
/// fill. Lerps the accent 25 % toward black. Replaces the legacy
/// `ACCENT_PRESSED`.
pub fn accent_pressed() -> egui::Color32 {
    lerp_rgb(active_accent(), egui::Color32::BLACK, 0.25)
}

/// Fill colour used by **multi-state row widgets** (tree row, hybrid
/// select row, dropdown popup row, command-palette row) when the
/// row is being hovered. Rather than the static `bg_hover` palette
/// colour these widgets used to hardcode, this helper accent-tints
/// the surface so hover pops on GAME's accent panel and stays
/// recognisable on PRO's dark panel.
pub fn row_hover_fill(accent: egui::Color32) -> egui::Color32 {
    let th = theme();
    let surface = if th.section_show_frame {
        section_fill(accent)
    } else {
        pane_fill(accent)
    };
    // 18 % accent blend — enough to tint hover, not enough to
    // collide with the 45 % accent blend used for "selected".
    lerp_rgb(surface, accent, 0.18)
}

/// Fill colour used by **multi-state row widgets** when the row is
/// the selected one. A 45 % accent blend over the row's natural
/// surface — clearly louder than `row_hover_fill` (18 %) so hover
/// and selected never visually collapse, even on flat themes
/// without strokes / glass.
pub fn row_selected_fill(accent: egui::Color32) -> egui::Color32 {
    let th = theme();
    let surface = if th.section_show_frame {
        section_fill(accent)
    } else {
        pane_fill(accent)
    };
    lerp_rgb(surface, accent, 0.45)
}

pub mod font {
    //! Typographic hierarchy — specific sizes so "small", "body",
    //! "strong" read as distinct tiers. Bodies 11 pt; captions 10;
    //! small-numeric (monospace, readouts) 11.
    pub const TITLE: f32 = 13.0;
    pub const BODY: f32 = 11.0;
    pub const CAPTION: f32 = 10.0;
    pub const NUMERIC: f32 = 11.0;
}

/// Draw a 1 px subtle divider line across the current row. Used to
/// separate the section header from its body and to split in-section
/// blocks (e.g. vehicle info vs controls).
pub fn divider(ui: &mut egui::Ui) {
    let bw = theme().border_width;
    if bw <= 0.0 {
        return;
    }
    let full_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(full_width, 1.0),
        egui::Sense::empty(),
    );
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(bw, BORDER_SUBTLE),
    );
}

/// Title divider — same shape as [`divider`], painted hard enough to
/// clearly read as "the container title ends here". Used under
/// foldable section headers so the title block stands apart from the
/// body content. Matches the opacity of the panel's main title rule
/// so every title-to-body transition in the UI reads the same way.
pub fn thin_divider(ui: &mut egui::Ui) {
    let bw = theme().border_width;
    if bw <= 0.0 {
        return;
    }
    let full_width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(full_width, 1.0),
        egui::Sense::empty(),
    );
    let color = egui::Color32::from_rgba_unmultiplied(
        BORDER_SUBTLE.r(),
        BORDER_SUBTLE.g(),
        BORDER_SUBTLE.b(),
        220,
    );
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(bw, color),
    );
}

/// Uppercase title text for panel-level headings (above sections).
/// Pops against the Light body font via `.strong()` + an enlarged
/// point size (20 % above `font::TITLE`) + primary text colour.
pub fn title_text(label: &str) -> egui::RichText {
    egui::RichText::new(label)
        .strong()
        .size(font::TITLE * 1.20)
        .color(TEXT_PRIMARY)
}

/// Small dim label — the "what is this row" caption-sized text that
/// sits in the left cell of a labelled row. Resolves the colour via
/// `on_section_dim()` so it stays readable on whichever surface
/// the active theme + accent end up producing (PRO: dim grey on
/// dark card; GAME: contrast-tinted dim against the accent panel).
pub fn body_label(label: &str) -> egui::RichText {
    egui::RichText::new(label).small().color(on_section_dim())
}

/// Italic caption — for under-row hints ("drag to edit", etc.). Like
/// `body_label`, the colour comes from the active theme's
/// dim-against-section helper so it never decays into "grey on
/// bright accent" under GAME.
pub fn caption(label: &str) -> egui::RichText {
    egui::RichText::new(label).small().italics().color(on_section_dim())
}

/// Text colour that stays readable on top of an arbitrary accent
/// fill. Uses Rec. 709 luma of the fill — bright fills get near-black
/// text, dim fills get white — so progress-bar readouts never
/// disappear into the accent when the user drives a yellow harvester
/// or a pastel-lavender husky.
pub fn contrast_text_for(fill: egui::Color32) -> egui::Color32 {
    let r = fill.r() as f32 / 255.0;
    let g = fill.g() as f32 / 255.0;
    let b = fill.b() as f32 / 255.0;
    let luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    if luma > 0.55 {
        egui::Color32::from_rgb(0x18, 0x18, 0x1C)
    } else {
        TEXT_PRIMARY
    }
}
