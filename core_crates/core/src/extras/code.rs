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

pub use frost_code::{CodeEditor, ColorTheme, Syntax};

// `maximizable` is no longer called directly from this file — both
// `frost_code_editor` and `frost_code_editor_with_opts` route
// through `crate::embed::maximizable_with_opts` so the opts path
// is always live. `pub use` re-exports the symbol callers expect
// when migrating from the older signature.
pub use crate::embed::OverlayOpts;

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
    frost_code_editor_with_opts(
        ui,
        id_salt,
        text,
        syntax,
        accent,
        min_size,
        crate::embed::OverlayOpts::default(),
    )
}

/// The maximise-state key the code-editor wrapper registers with
/// [`crate::embed`], computed from the caller-supplied `id_salt`
/// (the same one passed to [`frost_code_editor`]). Compare against
/// [`crate::embed::fullscreen_owner`] to detect "is THIS code
/// editor the one currently in fullscreen?".
#[must_use]
pub fn code_fullscreen_key(id_salt: impl Hash) -> egui::Id {
    crate::embed::maximize_state_key(id_salt)
}

/// `true` while the code editor identified by `id_salt` is
/// currently in its fullscreen overlay. Shorthand for
/// `fullscreen_owner(ctx) == Some(code_fullscreen_key(id_salt))`.
#[must_use]
pub fn is_code_fullscreen(ctx: &egui::Context, id_salt: impl Hash) -> bool {
    crate::embed::fullscreen_owner(ctx) == Some(code_fullscreen_key(id_salt))
}

/// Same as [`frost_code_editor`] but accepts an [`OverlayOpts`] so
/// the caller can choose where the minimize chip lands on the
/// fullscreen overlay (which edge + which cluster along that edge).
pub fn frost_code_editor_with_opts(
    ui: &mut egui::Ui,
    id_salt: impl Hash + Copy,
    text: &mut String,
    syntax: Syntax,
    accent: egui::Color32,
    min_size: egui::Vec2,
    fs_opts: crate::embed::OverlayOpts,
) {
    crate::embed::maximizable_with_opts(ui, id_salt, accent, min_size, fs_opts, |ui| {
        let id = format!("frost_code_editor_{:?}", ui.id());
        let code = crate::style::theme().code;
        let line_h = code.font_size * code.line_height_factor;
        let rows = ((ui.available_height() / line_h).floor() as usize).max(code.min_rows);
        CodeEditor::default()
            .id_source(id)
            .with_syntax(syntax)
            .with_theme(frost_code_theme(accent))
            .with_fontsize(code.font_size)
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
    use crate::style::{accent_pressed, glass_alpha_window, glass_fill, on_panel_dim, pane_fill};
    let code = crate::style::theme().code;
    ColorTheme {
        name: "Frost",
        dark: code.force_dark,
        // `glass_fill(pane_fill(...), …)` flows through the active
        // theme so GAME's accent panel becomes the editor bg too,
        // not a hardcoded dark.
        bg: glass_fill(pane_fill(accent), accent, glass_alpha_window()),
        cursor: accent,
        // Selection = darker accent shade derived at runtime so it
        // tracks whatever colour the user picked.
        selection: accent_pressed(),
        // `comments` / `punctuation` flip to whatever contrasts the
        // pane fill, so they stay readable on PRO's dark and GAME's
        // accent-coloured panels alike.
        comments: on_panel_dim(),
        functions: code.functions,
        keywords: accent,
        literals: code.literals,
        numerics: code.numerics,
        punctuation: on_panel_dim(),
        strs: code.strings,
        types: code.types,
        special: accent,
    }
}

// ─── Typed Pod constructor ──────────────────────────────────────────
//
// Adds `Pod::with_code_editor(text_id, syntax, default_text)` so
// pane bodies can host a code editor through the canonical pod path
// instead of reaching into `Pod::with_custom_units` (which is the
// raw-egui escape hatch). The text buffer is stashed in egui ctx
// data under `text_id`; the editor reads / writes it each frame.

impl crate::pod::Pod {
    /// Append a frost-themed code editor to this pod. The editor's
    /// text lives in egui ctx data under `text_id` — pre-seed it
    /// (`ctx.data_mut(|d| d.insert_temp(text_id, "default".to_string()))`)
    /// or rely on `default_text` to seed on first render.
    ///
    /// Uses `frost_core::style::active_accent()` for the inline
    /// theme. The maximise / restore chip in the editor's top-left
    /// corner toggles fullscreen via `frost_core::embed`.
    ///
    /// Reserves 10 row-height units of pod space.
    #[must_use]
    pub fn with_code_editor(
        self,
        text_id: egui::Id,
        syntax: Syntax,
        default_text: impl Into<String>,
    ) -> Self {
        let default = default_text.into();
        self.with_custom_units(10, move |ui| {
            let mut text: String = ui
                .ctx()
                .data(|d| d.get_temp::<String>(text_id))
                .unwrap_or_else(|| default.clone());
            let avail = ui.available_size_before_wrap();
            let accent = crate::style::active_accent();
            frost_code_editor_with_opts(
                ui,
                text_id,
                &mut text,
                syntax.clone(),
                accent,
                avail,
                OverlayOpts::default(),
            );
            ui.ctx().data_mut(|d| d.insert_temp(text_id, text));
        })
    }
}
