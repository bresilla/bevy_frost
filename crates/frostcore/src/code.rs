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

pub use crate::features::code_editor::{CodeEditor, ColorTheme, Syntax};

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

/// Build a [`ColorTheme`] whose background / text / selection
/// colours come from the frost palette, while the syntactic
/// colours reuse the existing accent / status hues — so the
/// editor belongs to the same visual family as sections and
/// widgets around it.
///
/// Now that [`ColorTheme`] stores [`Color32`] directly (the
/// vendored struct was rewritten from `&'static str` hex), the
/// background uses the same `glass_fill` recipe as the node-graph
/// canvas and the floating-pane frame — so the global
/// `GlassOpacity` slider dims the code editor in lockstep with
/// every other frost surface.
///
/// `accent` drives keyword highlighting + the cursor; status
/// colours (`SUCCESS`, `AXIS_X/Y/Z`) tint literals / types /
/// punctuation for a readable hierarchy.
fn frost_code_theme(accent: egui::Color32) -> ColorTheme {
    use crate::style::{
        glass_alpha_window, glass_fill, ACCENT_PRESSED, AXIS_X, AXIS_Y, AXIS_Z, BG_1_PANEL,
        SUCCESS, TEXT_SECONDARY,
    };
    ColorTheme {
        name: "Frost",
        dark: true,
        // `glass_fill` produces an accent-tinted semi-transparent
        // fill of `BG_1_PANEL`; it picks up `GlassOpacity`
        // automatically because `glass_alpha_window()` reads from
        // the shared atomic. Same recipe the snarl canvas uses —
        // so the two surfaces read as one glass family.
        bg: glass_fill(BG_1_PANEL, accent, glass_alpha_window()),
        cursor: accent,
        selection: ACCENT_PRESSED,
        comments: TEXT_SECONDARY,
        functions: AXIS_Y,
        keywords: accent,
        literals: AXIS_X,
        numerics: AXIS_X,
        punctuation: TEXT_SECONDARY,
        strs: SUCCESS,
        types: AXIS_Z,
        special: accent,
    }
}
