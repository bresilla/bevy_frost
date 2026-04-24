//! Code-editor integration — thin wrapper around
//! [`egui_code_editor`] that pipes a multiline text buffer
//! through the same maximise / restore affordance the graph
//! widget uses.
//!
//! Minimal usage (inside a [`section`](crate::widgets::section)
//! body, since panes require containers):
//!
//! ```ignore
//! frost_code_editor(
//!     ui,
//!     "my_code",
//!     &mut state.code,
//!     Syntax::rust(),
//!     accent,
//!     egui::vec2(w, 300.0),
//! );
//! ```
//!
//! The widget paints:
//!
//! * Line numbers in the gutter.
//! * Monospace text with syntax highlighting for the chosen
//!   [`Syntax`] (Rust, shell, SQL, ASM, or custom).
//! * The maximise / restore chip in the top-left corner — click
//!   once to blow the editor up to full window, click again to
//!   snap it back inline.
//!
//! Re-exports: `Syntax`, `ColorTheme`, `CodeEditor` from
//! `egui_code_editor` so callers don't need a second dep.

use std::hash::Hash;

use egui;

pub use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

use crate::maximize::maximizable;

/// Render a syntax-highlighted code editor bound to `text`,
/// wrapped in the shared maximise / restore toggle. The caller
/// owns the text buffer — the widget just edits it in place.
///
/// `syntax` controls keyword / punctuation / literal highlighting.
/// Pre-built variants: `Syntax::rust()`, `Syntax::shell()`,
/// `Syntax::sql()`, `Syntax::asm()`. Build a custom one with the
/// `Syntax` struct fields directly for other languages.
pub fn frost_code_editor(
    ui: &mut egui::Ui,
    id_salt: impl Hash + Copy,
    text: &mut String,
    syntax: Syntax,
    accent: egui::Color32,
    min_size: egui::Vec2,
) {
    maximizable(ui, id_salt, accent, min_size, |ui| {
        let id = format!("frost_code_editor_{:?}", ui.id());
        let font_size = 13.0;
        let line_h = font_size * 1.2;
        let rows = ((ui.available_height() / line_h).floor() as usize).max(6);
        CodeEditor::default()
            .id_source(id)
            .with_syntax(syntax)
            .with_theme(frost_code_theme(accent))
            .with_fontsize(font_size)
            .with_rows(rows)
            .with_numlines(true)
            .show(ui, text);
    });
}

/// Build an `egui_code_editor::ColorTheme` whose background / text
/// / selection colours come from the frost palette (same BG
/// tones the rest of the UI uses), while the syntactic colours
/// reuse the crate's existing accent / status hues — so the
/// editor belongs to the same visual family as sections and
/// widgets around it.
///
/// The `accent` parameter is used as the colour of keywords
/// (most prominent token family); literals / types / punctuation
/// use subtler tints so they read as a hierarchy.
fn frost_code_theme(accent: egui::Color32) -> ColorTheme {
    use crate::style::{
        ACCENT_PRESSED, AXIS_X, AXIS_Y, AXIS_Z, BG_1_PANEL, SUCCESS, TEXT_SECONDARY,
    };
    let hex6 = |c: egui::Color32| -> &'static str {
        // `ColorTheme` stores colours as `&'static str` HTML
        // hex literals. Leaking the String is a one-shot cost per
        // accent-change — acceptable for a theme constructor
        // invoked only when `AccentColor` actually changes.
        let s = format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
        Box::leak(s.into_boxed_str())
    };
    ColorTheme {
        name: "Frost",
        dark: true,
        // Same base colour as the node-editor's glass canvas at
        // full opacity (`BG_1_PANEL`). `egui_code_editor`'s
        // `ColorTheme.bg` is `&'static str` and `color_from_hex`
        // only parses the RGB bytes — there's no alpha channel
        // support, so the editor stays opaque while the rest of
        // the pane dims with the `GlassOpacity` slider. (The
        // `"none"` sentinel that egui_code_editor exposes renders
        // as magenta in this setup — premultiplied `(255, 0, 255,
        // 0)` leaks through some render paths.)
        bg: hex6(BG_1_PANEL),
        cursor: hex6(accent),
        selection: hex6(ACCENT_PRESSED),
        comments: hex6(TEXT_SECONDARY),
        functions: hex6(AXIS_Y),
        keywords: hex6(accent),
        literals: hex6(AXIS_X),
        numerics: hex6(AXIS_X),
        punctuation: hex6(TEXT_SECONDARY),
        strs: hex6(SUCCESS),
        types: hex6(AXIS_Z),
        special: hex6(accent),
    }
}
