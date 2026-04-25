//! Shared layout constants + the single paint helper every ribbon
//! button renders through. Internal to the `ribbon` module; the
//! static ribbons and the drag-aware layout both route here so the
//! pixel-level look stays identical whichever path the caller took.

use egui;

use crate::style::{
    glass_alpha_card, glass_alpha_window, glass_fill, BG_1_PANEL, BG_2_RAISED, BORDER_SUBTLE,
};

/// sRGB lerp on RGB channels, alpha left at 255. Local copy so the
/// ribbon paint module doesn't reach into `style`'s private helpers.
pub(crate) fn lerp_rgb(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    egui::Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
}

/// Foreground (glyph / label) colour matching the recipe in
/// [`paint_ribbon_button`]. Centralised so the static-ribbon path
/// (`ribbon_button_area`) and the dynamic drag-aware path
/// (`ribbon::layout`, `ribbon::assembly`) all pick text that
/// contrasts with the EXACT fill the button paints, rather than
/// each call site re-deriving it (and drifting out of sync the
/// next time the bg recipe changes).
/// Shared paint dispatch for the three ribbon-button glyph kinds.
/// Centred on `rect`'s middle; size = 14 px (text/icon) or rect
/// shrunk by 6 px (svg). Tinted in `fg`.
pub(crate) fn paint_ribbon_glyph(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    glyph: super::assembly::RibbonGlyph,
    fg: egui::Color32,
) {
    use super::assembly::RibbonGlyph;
    match glyph {
        RibbonGlyph::Text(s) => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                s,
                egui::FontId::new(14.0, egui::FontFamily::Monospace),
                fg,
            );
        }
        RibbonGlyph::Icon(name) => {
            crate::icons::paint_icon(
                &ui.painter(),
                rect.center(),
                egui::Align2::CENTER_CENTER,
                name,
                18.0,
                fg,
            );
        }
        RibbonGlyph::Svg(svg) => {
            crate::icons::paint_section_icon(
                ui,
                rect.center(),
                egui::Align2::CENTER_CENTER,
                crate::icons::Icon::Svg(svg),
                rect.shrink(6.0).width(),
                fg,
            );
        }
    }
}

pub(crate) fn ribbon_button_fg(
    accent: egui::Color32,
    is_active: bool,
    hovered: bool,
    glyph: super::assembly::RibbonGlyph,
) -> egui::Color32 {
    if crate::style::theme().ribbon_button_accent_fill {
        // GAME ladder — pick contrast text against the EXACT fill the
        // accent path painted, so the glyph sits cleanly against the
        // dim / accent / brightened tier.
        let fill = if is_active {
            lerp_rgb(accent, egui::Color32::WHITE, 0.28)
        } else if hovered {
            accent
        } else {
            lerp_rgb(accent, egui::Color32::BLACK, 0.30)
        };
        return crate::style::contrast_text_for(fill);
    }
    // PRO recipe. Active button paints over an accent-tinted bg.
    // For Text / Icon glyphs render the active fg as a *brightened*
    // accent (`lerp(accent, WHITE, 0.20)`) so the glyph reads as the
    // selected tier — vivid accent letter / icon on the
    // accent-tinted button. SVG glyphs keep the contrast colour
    // because their author chose their own colours via the SVG
    // markup; tinting them accent would corrupt their look.
    use super::assembly::RibbonGlyph;
    if is_active {
        if matches!(glyph, RibbonGlyph::Svg(_)) {
            crate::style::contrast_text_for(accent)
        } else {
            lerp_rgb(accent, egui::Color32::WHITE, 0.20)
        }
    } else {
        crate::style::on_panel_dim()
    }
}

// ─── Layout constants ───────────────────────────────────────────────

/// Edge length of each square ribbon button (VS Code / Fleet size).
pub const SIDE_BTN_SIZE: f32 = 34.0;
/// Gap between adjacent ribbon buttons.
pub const SIDE_BTN_GAP: f32 = 4.0;
/// Distance from the screen edge to the near edge of each button.
pub const EDGE_GAP: f32 = 8.0;

// ─── Paint ──────────────────────────────────────────────────────────

/// Background / border recipe for every ribbon button. Per-theme
/// branch:
///
/// * `ribbon_button_accent_fill = false` (PRO) → original glass
///   look. Idle paints `BG_1_PANEL`, hover lifts to `BG_2_RAISED`,
///   active blends 25 % accent into the raised tier and adds an
///   accent stroke. Same recipe the kit shipped with.
/// * `ribbon_button_accent_fill = true` (GAME) → three-tier accent
///   ladder. Idle = accent dimmed 30 % toward black, hover = pure
///   accent, active = accent brightened 28 % toward white + 1.5 px
///   outer accent halo.
pub(crate) fn paint_ribbon_button(
    painter: &egui::Painter,
    rect: egui::Rect,
    accent: egui::Color32,
    is_active: bool,
    hovered: bool,
) {
    let theme = crate::style::theme();
    let radius = egui::CornerRadius::same(theme.radius_md);

    if theme.ribbon_button_accent_fill {
        // Just filled, three accent tiers, no stroke / halo / border.
        // The active state's brightness lift is the entire selection
        // cue — no extra outline.
        let fill = if is_active {
            lerp_rgb(accent, egui::Color32::WHITE, 0.28)
        } else if hovered {
            accent
        } else {
            lerp_rgb(accent, egui::Color32::BLACK, 0.30)
        };
        painter.rect(
            rect,
            radius,
            fill,
            egui::Stroke::NONE,
            egui::StrokeKind::Inside,
        );
        return;
    }

    // PRO recipe — restored unchanged from the original glass look.
    let bg = if is_active {
        let blend = |a: u8, b: u8| ((a as f32) * 0.75 + (b as f32) * 0.25).round() as u8;
        let tinted = egui::Color32::from_rgb(
            blend(BG_2_RAISED.r(), accent.r()),
            blend(BG_2_RAISED.g(), accent.g()),
            blend(BG_2_RAISED.b(), accent.b()),
        );
        glass_fill(tinted, accent, glass_alpha_window())
    } else if hovered {
        glass_fill(BG_2_RAISED, accent, glass_alpha_window())
    } else {
        glass_fill(BG_1_PANEL, accent, glass_alpha_window())
    };
    let stroke = if is_active { accent } else { BORDER_SUBTLE };
    painter.rect(
        rect,
        radius,
        bg,
        egui::Stroke::new(theme.border_width, stroke),
        egui::StrokeKind::Inside,
    );
    let _ = glass_alpha_card();
}

/// Paint a single static ribbon button at `anchor + offset` and
/// return its `Response`. Shared by [`super::static_ribbon`]; the
/// drag-aware `RibbonLayout` constructs its areas by hand so it can
/// set `Order::Tooltip` while a button is lifted.
pub(crate) fn ribbon_button_area(
    id: &'static str,
    ctx: &egui::Context,
    anchor: egui::Align2,
    offset: egui::Vec2,
    glyph: super::assembly::RibbonGlyph,
    tooltip: &str,
    is_active: bool,
    accent: egui::Color32,
    on_click: impl FnOnce(),
) {
    egui::Area::new(egui::Id::new(id))
        .anchor(anchor, offset)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(
                egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
                egui::Sense::click(),
            );

            paint_ribbon_button(ui.painter(), rect, accent, is_active, resp.hovered());
            let fg = ribbon_button_fg(accent, is_active, resp.hovered(), glyph);
            paint_ribbon_glyph(ui, rect, glyph, fg);

            if resp.on_hover_text(tooltip).clicked() {
                on_click();
            }
        });
}
