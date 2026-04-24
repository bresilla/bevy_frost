//! Hybrid-select row — a single row with **two independent click
//! targets**:
//!
//!   1. a main *body* (click / double-click / drag source) for
//!      **transient** selection — "what am I pointing at right now",
//!   2. a right-edge *radio* for **durable** one-at-a-time state —
//!      "which of these is the canonical one" (camera-follow target,
//!      active layer, default account, …).
//!
//! Why two targets on one row? Collapsing them would force every
//! click to flip the durable state — wrong ergonomics for a list
//! where hovering / tapping to read shouldn't touch the pinned slot.
//! Splitting them lets a row be selected (body-click), examined
//! (body-double-click), dragged out, *or* pinned (radio-click) with
//! no overlap.
//!
//! The two rects are laid out so they never intersect, so a click on
//! the radio never propagates to the body and vice versa. The widget
//! paints the row body's selection + hover fill, the label + trailing
//! text, and the radio's ring + centre dot; the caller just reads
//! `body.clicked()`, `body.double_clicked()`, `radio.clicked()` and
//! reacts.
//!
//! Shape:
//! ```text
//!   [ Planet                                  #3     (o) ]
//!     └── body (click / double-click)         └───┘  └─┘  radio
//!                                             trailing
//! ```
//!
//! This widget is scene-panel shaped (Blender 4 outliner / UE5
//! world-outliner) but isn't tied to any one domain — "scene",
//! "entities", "layers", whatever list the caller wants to present
//! with that split-semantics click model.

use egui;

use crate::style::{BG_2_RAISED, BG_3_HOVER, TEXT_PRIMARY, TEXT_SECONDARY};

use super::shared::widget_separator;

/// Row height. Matches the Blender 4 outliner / UE5 world-outliner
/// rhythm (20 px row, 12 px label).
pub const HYBRID_SELECT_ROW_H: f32 = 20.0;

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

/// Render a hybrid-select row. `id_salt` is anything hashable that's
/// unique among the rows in this ui (an index, an entity id, a
/// string). The two sub-rects get the same salt with internal
/// prefixes so the interaction ids never collide.
///
/// `selected` paints the body's selection tint (accent-blended);
/// `radio_on` paints the radio's filled dot. Both are pure
/// presentation — caller owns the state.
pub fn hybrid_select_row(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    trailing: Option<&str>,
    selected: bool,
    radio_on: bool,
    accent: egui::Color32,
) -> HybridSelectResponse {
    // Geometry — compact so the label still owns most of the row.
    const RADIO_OUTER_R: f32 = 4.5;
    const RADIO_SLOT_W: f32 = 14.0;
    const RADIO_PAD_R: f32 = 5.0;
    const LABEL_PAD_L: f32 = 10.0;
    const TRAILING_PAD_R: f32 = 6.0;

    let w = ui.available_width();

    // Reserve the whole row with `hover` so sub-rects own click
    // routing — same trick egui uses internally for compound widgets.
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(w, HYBRID_SELECT_ROW_H),
        egui::Sense::hover(),
    );

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

    let painter = ui.painter_at(rect);
    let mid_y = rect.center().y;

    // One unified fill across the WHOLE row (including the radio
    // slot) — the row reads as a single visual button while the
    // click routing stays split underneath. Hover lights up from
    // *either* sub-rect so nothing flickers as the pointer crosses
    // the internal boundary.
    let any_hover = body.hovered() || radio.hovered();
    if selected {
        let blend = |a: u8, b: u8| ((a as f32) * 0.65 + (b as f32) * 0.35).round() as u8;
        let tint = egui::Color32::from_rgb(
            blend(BG_2_RAISED.r(), accent.r()),
            blend(BG_2_RAISED.g(), accent.g()),
            blend(BG_2_RAISED.b(), accent.b()),
        );
        painter.rect_filled(rect, egui::CornerRadius::same(3), tint);
    } else if any_hover {
        painter.rect_filled(rect, egui::CornerRadius::same(3), BG_3_HOVER);
    }

    painter.text(
        egui::pos2(body_rect.min.x + LABEL_PAD_L, mid_y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        TEXT_PRIMARY,
    );
    if let Some(t) = trailing {
        painter.text(
            egui::pos2(body_rect.max.x - TRAILING_PAD_R, mid_y),
            egui::Align2::RIGHT_CENTER,
            t,
            egui::FontId::proportional(10.0),
            TEXT_SECONDARY,
        );
    }

    // Radio: outline ring + filled dot when on. Hover brightens the
    // ring to accent so the control reads as interactive.
    let radio_center = egui::pos2(radio_rect.center().x, mid_y);
    let ring_color = if radio_on || radio.hovered() {
        accent
    } else {
        TEXT_SECONDARY
    };
    painter.circle_stroke(
        radio_center,
        RADIO_OUTER_R,
        egui::Stroke::new(1.2, ring_color),
    );
    if radio_on {
        painter.circle_filled(radio_center, RADIO_OUTER_R - 1.8, accent);
    }

    HybridSelectResponse { body, radio }
}

/// Module-style variant: same row as [`hybrid_select_row`], then
/// paints a trailing [`widget_separator`] so a stack of rows matches
/// the every-row-divided rhythm of the other frost widgets. Use this
/// when the list sits inside a section alongside toggles / sliders /
/// etc.; reach for [`hybrid_select_row`] directly when you're
/// building a dense list that should not double-stripe.
pub fn hybrid_select_row_divided(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    label: &str,
    trailing: Option<&str>,
    selected: bool,
    radio_on: bool,
    accent: egui::Color32,
) -> HybridSelectResponse {
    let resp = hybrid_select_row(ui, id_salt, label, trailing, selected, radio_on, accent);
    widget_separator(ui);
    resp
}
