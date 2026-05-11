//! Frost-styled select rows.
//!
//! Two variants ship from the same paint primitive:
//!
//! * [`select_row`] — single click target on the whole row body.
//!   Use for plain "pick one of N" lists where each row's only
//!   semantic is "select this".
//! * [`hybrid_select_row`] — body click target PLUS an independent
//!   right-edge radio. Use when a row needs *transient* selection
//!   ("what am I pointing at right now") and *durable* pinning
//!   ("which is the canonical one") — the two click targets never
//!   intersect, so a click on the radio doesn't propagate to the
//!   body and vice-versa.
//!
//! Visual:
//! ```text
//!   [ Planet                                   #3        ]    <- select_row
//!   [ Planet                                   #3    (o) ]    <- hybrid_select_row
//!     └── body click                           └─┘   └─┘
//!                                              trailing  radio
//! ```
//!
//! Both variants paint a unified hover / selected fill across the
//! whole row, so the row reads as a single visual button while the
//! click routing under it stays split.

use crate::style::{
    on_section, on_section_dim, row_hover_fill, row_selected_fill, theme, ColorMode,
};

/// Row height — matches the Blender 4 outliner / UE5 world-outliner
/// rhythm (20 px row, 12 px label).
pub const SELECT_ROW_H: f32 = 20.0;
/// Alias for callers that previously imported the hybrid-only constant.
pub const HYBRID_SELECT_ROW_H: f32 = SELECT_ROW_H;

/// The two independent `egui::Response`s produced by one
/// [`hybrid_select_row`]. Inspect each separately: `body` for
/// click / double-click / hover on the main row, `radio` for the
/// right-edge toggle.
#[derive(Debug)]
pub struct HybridSelectResponse {
    /// Click target covering everything except the radio slot.
    pub body: egui::Response,
    /// Click target for the right-edge radio circle only.
    pub radio: egui::Response,
}

/// Plain select row — one click target across the whole row.
///
/// `id_salt` disambiguates this row from siblings (an index, an entity
/// id, a string). `selected` paints the body's selection tint;
/// `trailing` is rendered dim-right (e.g. an index, a hotkey).
/// Caller owns the state — wire `resp.clicked()` / `resp.double_clicked()`
/// up to your selection logic.
pub fn select_row(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    trailing: Option<&str>,
    selected: bool,
    accent: egui::Color32,
) -> egui::Response {
    select_row_h(ui, id_salt, label, trailing, selected, accent, SELECT_ROW_H)
}

/// Variable-height plain select row — used by resizable pods.
pub fn select_row_h(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    trailing: Option<&str>,
    selected: bool,
    accent: egui::Color32,
    height: f32,
) -> egui::Response {
    const LABEL_PAD_L: f32 = 10.0;
    const TRAILING_PAD_R: f32 = 6.0;
    let w = ui.available_width();
    let resp = ui.interact(
        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(w, height)),
        ui.id().with(("frost_select_body", &id_salt)),
        egui::Sense::click(),
    );
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(w, height), egui::Sense::hover());
    paint_row_bg(ui, rect, selected, resp.hovered(), accent);
    paint_row_text(ui, rect, label, trailing, LABEL_PAD_L, TRAILING_PAD_R);
    resp
}

/// Hybrid select row — body click target + right-edge radio.
///
/// `radio_on` paints the radio's filled dot. Body and radio sub-rects
/// never intersect; their `Response` ids are independent so the two
/// click sources stay separate.
pub fn hybrid_select_row(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    trailing: Option<&str>,
    selected: bool,
    radio_on: bool,
    accent: egui::Color32,
) -> HybridSelectResponse {
    hybrid_select_row_h(
        ui, id_salt, label, trailing, selected, radio_on, accent, SELECT_ROW_H,
    )
}

/// Variable-height hybrid select row.
pub fn hybrid_select_row_h(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    trailing: Option<&str>,
    selected: bool,
    radio_on: bool,
    accent: egui::Color32,
    height: f32,
) -> HybridSelectResponse {
    const RADIO_OUTER_R: f32 = 4.5;
    const RADIO_SLOT_W: f32 = 14.0;
    const RADIO_PAD_R: f32 = 5.0;
    const LABEL_PAD_L: f32 = 10.0;
    const TRAILING_PAD_R: f32 = 6.0;

    let w = ui.available_width();
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(w, height), egui::Sense::hover());

    let radio_rect = egui::Rect::from_min_size(
        egui::pos2(rect.max.x - RADIO_SLOT_W - RADIO_PAD_R, rect.min.y),
        egui::vec2(RADIO_SLOT_W, rect.height()),
    );
    let body_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(radio_rect.min.x, rect.max.y),
    );

    let body = ui.interact(
        body_rect,
        ui.id().with(("frost_hybrid_body", &id_salt)),
        egui::Sense::click(),
    );
    let radio = ui.interact(
        radio_rect,
        ui.id().with(("frost_hybrid_radio", &id_salt)),
        egui::Sense::click(),
    );

    let any_hover = body.hovered() || radio.hovered();
    paint_row_bg(ui, rect, selected, any_hover, accent);
    paint_row_text(ui, body_rect, label, trailing, LABEL_PAD_L, TRAILING_PAD_R);

    // Radio: outline ring + filled dot when on. Hover brightens the
    // ring to accent so the control reads as interactive.
    let mid_y = rect.center().y;
    let painter = ui.painter_at(rect);
    let radio_center = egui::pos2(radio_rect.center().x, mid_y);
    let ring_color = if radio_on || radio.hovered() {
        accent
    } else {
        on_section_dim()
    };
    painter.circle_stroke(
        radio_center,
        RADIO_OUTER_R,
        egui::Stroke::new(1.2, ring_color),
    );
    if radio_on {
        // Inner dot fills with `accent` UNLESS the row is also
        // accent-derived (GAME's accent panel + accent dot would
        // collide); in that case use a contrasting solid against
        // the panel so the dot stays visible.
        let dot_col = if matches!(theme().panel_fill_mode, ColorMode::FromAccent { .. }) {
            on_section()
        } else {
            accent
        };
        painter.circle_filled(radio_center, RADIO_OUTER_R - 1.8, dot_col);
    }

    HybridSelectResponse { body, radio }
}

fn paint_row_bg(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    selected: bool,
    hovered: bool,
    accent: egui::Color32,
) {
    let painter = ui.painter_at(rect);
    if selected {
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(theme().radius_compact),
            row_selected_fill(accent),
        );
    } else if hovered {
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(theme().radius_compact),
            row_hover_fill(accent),
        );
    }
}

fn paint_row_text(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    label: &str,
    trailing: Option<&str>,
    label_pad_l: f32,
    trailing_pad_r: f32,
) {
    let painter = ui.painter_at(rect);
    let mid_y = rect.center().y;
    painter.text(
        egui::pos2(rect.min.x + label_pad_l, mid_y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        on_section(),
    );
    if let Some(t) = trailing {
        painter.text(
            egui::pos2(rect.max.x - trailing_pad_r, mid_y),
            egui::Align2::RIGHT_CENTER,
            t,
            egui::FontId::proportional(10.0),
            on_section_dim(),
        );
    }
}
