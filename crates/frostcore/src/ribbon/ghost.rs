//! Ghost preview paint for an in-flight ribbon-button drag.
//!
//! Framework-agnostic — the paint work lives in [`paint_drop_ghost`]
//! which takes a plain `&egui::Context`. `bevy_frost` wraps that
//! in a Bevy system and a `SystemSet` for scheduling.

use crate::style::AccentColor;

use super::layout::{
    insertion_slot_under_cursor, rect_for, ribbon_under_cursor, RibbonLayout,
};
use super::paint::SIDE_BTN_SIZE;

/// Paint the ghost slot under the cursor if a drag is in progress.
/// No-op when no drag is active or when the cursor is over a
/// ribbon the dragged button's constraint forbids.
pub fn paint_drop_ghost(
    ctx: &egui::Context,
    layout: &RibbonLayout,
    accent: AccentColor,
) {
    let Some(drag) = layout.drag.as_ref() else { return };

    let screen = ctx.content_rect();
    let Some(target_kind) = ribbon_under_cursor(drag.cursor, screen) else {
        return;
    };
    if !drag.constraint.allows(target_kind) {
        return;
    }

    let target_slot = insertion_slot_under_cursor(
        target_kind,
        drag.cursor,
        &layout.placements,
        &drag.id,
        screen,
    );

    // Siblings-after-move count on target: current non-dragged
    // count, plus one for the landing slot.
    let siblings_on_target = layout
        .placements
        .iter()
        .filter(|(id, p)| p.kind == target_kind && id.as_str() != drag.id)
        .count() as u32;
    let total_visible = siblings_on_target + 1;

    let rect = rect_for(target_kind, target_slot, total_visible, screen);

    let ghost_fill = egui::Color32::from_rgba_unmultiplied(
        accent.0.r(),
        accent.0.g(),
        accent.0.b(),
        40,
    );
    let ghost_stroke = egui::Color32::from_rgba_unmultiplied(
        accent.0.r(),
        accent.0.g(),
        accent.0.b(),
        180,
    );

    egui::Area::new(egui::Id::new("ribbon_drop_ghost"))
        .order(egui::Order::Middle)
        .fixed_pos(rect.min)
        .interactable(false)
        .show(ctx, |ui| {
            let (ghost_rect, _) = ui.allocate_exact_size(
                egui::vec2(SIDE_BTN_SIZE, SIDE_BTN_SIZE),
                egui::Sense::hover(),
            );
            ui.painter().rect(
                ghost_rect,
                egui::CornerRadius::same(crate::style::theme().radius_md),
                ghost_fill,
                egui::Stroke::new(1.5, ghost_stroke),
                egui::StrokeKind::Inside,
            );
        });
}
