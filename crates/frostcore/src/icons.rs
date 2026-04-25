//! Filled Fluent UI System Icons via the [`iconflow`] crate.
//!
//! frostcore registers every Fluent UI font variant in
//! [`crate::style::apply_theme`]'s font install pass so widgets can
//! render an icon glyph anywhere a `RichText` or `painter().text(..)`
//! call lands. Lookup is by string name (e.g. `"search"`,
//! `"chevron_down"`); style is filled and size is the regular
//! variant — that's the look the user asked for.
//!
//! Two entry points:
//!
//! * [`icon`] — returns `Option<(char, FontFamily)>` so callers that
//!   need the codepoint directly (custom painters) can place it
//!   themselves.
//! * [`icon_text`] — wraps the same lookup in a `RichText` ready to
//!   drop into `ui.label(...)` / `ui.add(Label::new(...))`.
//!
//! Fonts are bundled via iconflow's `fonts()` registry — we walk it
//! once at theme-apply time and register each `(family, bytes)` pair
//! as `egui::FontFamily::Name(family)`.

use std::sync::Arc;

use egui;
use iconflow::{fonts, try_icon, IconRef, Pack, Size, Style};

/// Pull every iconflow font into `FontDefinitions` and register
/// each as a named family so `FontFamily::Name(family)` resolves to
/// the right glyph table. Called from [`crate::style::install_fonts`].
pub(crate) fn install_iconflow_fonts(fonts_def: &mut egui::FontDefinitions) {
    for asset in fonts() {
        let key = asset.family.to_string();
        fonts_def
            .font_data
            .insert(key.clone(), Arc::new(egui::FontData::from_static(asset.bytes)));
        fonts_def
            .families
            .insert(egui::FontFamily::Name(asset.family.into()), vec![key]);
    }
}

/// Look up a filled Fluent UI System Icon by name. Returns the
/// glyph character + the font family to render it in. Returns
/// `None` when the icon isn't in the bundled set — caller should
/// fall back gracefully.
pub fn icon(name: &str) -> Option<(char, egui::FontFamily)> {
    let IconRef { family, codepoint } =
        try_icon(Pack::Fluentui, name, Style::Filled, Size::Regular).ok()?;
    let glyph = char::from_u32(codepoint)?;
    Some((glyph, egui::FontFamily::Name(family.into())))
}

/// Build a `RichText` rendering the named filled Fluent UI icon at
/// `size` px in `color`. Returns `None` if the icon isn't bundled —
/// callers can `.unwrap_or_else(|| RichText::new("?"))` or similar.
///
/// ```ignore
/// if let Some(t) = frostcore::icons::icon_text("search", 14.0, accent) {
///     ui.label(t);
/// }
/// ```
pub fn icon_text(name: &str, size: f32, color: egui::Color32) -> Option<egui::RichText> {
    let (glyph, family) = icon(name)?;
    Some(
        egui::RichText::new(glyph.to_string())
            .font(egui::FontId::new(size, family))
            .color(color),
    )
}

/// Paint a named filled Fluent UI icon at `pos` aligned by `align`
/// in `color` at `size` px. No-op when the icon isn't bundled.
pub fn paint_icon(
    painter: &egui::Painter,
    pos: egui::Pos2,
    align: egui::Align2,
    name: &str,
    size: f32,
    color: egui::Color32,
) {
    if let Some((glyph, family)) = icon(name) {
        painter.text(pos, align, glyph.to_string(), egui::FontId::new(size, family), color);
    }
}
