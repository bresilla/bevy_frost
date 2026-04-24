//! Cmd-K / Ctrl-P style command palette.
//!
//! A centred floating overlay with a search field at the top and
//! a fuzzy-matched list of named actions below. Kept open by
//! caller-owned state ([`CommandPaletteState`]) so the host
//! controls the key binding that opens it.
//!
//! Semantics:
//!
//! * **Open**: caller sets `state.open = true` — usually from a
//!   keyboard shortcut in the host app.
//! * **Dismiss**: Escape, clicking outside, or selecting an item.
//! * **Select**: Enter picks the currently-highlighted item; Up /
//!   Down moves the highlight. The id of the picked item is
//!   returned so the caller can dispatch.
//!
//! Matching: substring + initials ("otp" → "Open Timeline
//! Panel"). Simple scoring is enough for most command sets — for
//! sublime-grade ranking, wrap this palette and pre-filter
//! `items` yourself before passing them in.

use egui;

use crate::style::{
    font, glass_alpha_card, glass_alpha_window, glass_fill, radius, widget_border, BG_1_PANEL,
    BG_2_RAISED, TEXT_PRIMARY, TEXT_SECONDARY,
};

/// One entry in the palette's action list.
pub struct PaletteItem {
    pub id: &'static str,
    pub label: &'static str,
    /// Optional secondary hint — dim right-aligned text shown on
    /// each row. Use for keybindings ("Ctrl+P") or categories
    /// ("Layout").
    pub hint: Option<&'static str>,
}

/// Persistent state the palette owns. Wrap in whatever the host
/// prefers (bevy: `Resource`; plain egui: app field).
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Debug, Clone, Default)]
pub struct CommandPaletteState {
    /// Master toggle. Set from a keyboard-shortcut handler in the
    /// host. The palette also clears this on Escape / outside
    /// click / selection.
    pub open: bool,
    /// Current search query.
    pub query: String,
    /// Index into the filtered-items list of the row currently
    /// highlighted. Moved by Up / Down keys.
    pub selected: usize,
}

/// Draw the palette overlay when `state.open == true`. Returns
/// `Some(id)` on the frame an item is picked (via Enter /
/// click); otherwise `None`.
pub fn command_palette(
    ctx: &egui::Context,
    state: &mut CommandPaletteState,
    items: &[PaletteItem],
    accent: egui::Color32,
) -> Option<&'static str> {
    if !state.open {
        return None;
    }

    // Filter + score. `matcher` is a simple case-insensitive
    // substring check; initials match on tokens. Keep the cost
    // negligible even with thousands of items.
    let filtered: Vec<&PaletteItem> = if state.query.is_empty() {
        items.iter().collect()
    } else {
        let q = state.query.to_lowercase();
        items
            .iter()
            .filter(|it| matches(it.label, &q))
            .collect()
    };

    // Clamp the selected index against the CURRENT filtered view
    // — the query may have just shrunk the list.
    if filtered.is_empty() {
        state.selected = 0;
    } else {
        state.selected = state.selected.min(filtered.len() - 1);
    }

    let mut picked: Option<&'static str> = None;

    // Keyboard input — Up / Down / Enter / Escape. Consumed
    // before the palette body draws so the text field doesn't
    // swallow them.
    ctx.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
            state.open = false;
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
            && !filtered.is_empty()
        {
            state.selected = (state.selected + 1).min(filtered.len() - 1);
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
            state.selected = state.selected.saturating_sub(1);
        }
        if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
            && !filtered.is_empty()
        {
            picked = Some(filtered[state.selected].id);
        }
    });

    let screen = ctx.content_rect();
    // Full-screen scrim so clicks outside the palette dismiss it.
    // `Order::Foreground` places it above panes, below the
    // palette itself (which we paint at `Tooltip`).
    let scrim_clicked = egui::Area::new(egui::Id::new("frost_palette_scrim"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.allocate_exact_size(screen.size(), egui::Sense::click())
        })
        .inner
        .1
        .clicked();
    if scrim_clicked {
        state.open = false;
    }

    // Palette window — centred, fixed width, content-driven
    // height. Painted at `Order::Tooltip` so it sits above the
    // scrim.
    //
    // The Area + inner ScrollArea IDs fold in a **content
    // fingerprint** of the item slice — a hash of every item id —
    // so switching between palette contexts (e.g. graph-maximised
    // palette vs. general palette) gives the new context a fresh
    // Area / ScrollArea identity instead of re-using the previous
    // context's remembered dimensions. Without this, going from a
    // 3-item graph palette back to the 11-item general palette
    // would stay "tight" for a frame because egui remembered the
    // smaller content size from the previous render.
    let items_sig = items_fingerprint(items);
    const WIDTH: f32 = 560.0;
    let pos = egui::pos2(
        screen.center().x - WIDTH * 0.5,
        screen.min.y + screen.height() * 0.22,
    );
    egui::Area::new(egui::Id::new(("frost_palette", items_sig)))
        .order(egui::Order::Tooltip)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            ui.set_max_width(WIDTH);
            let frame = egui::Frame::new()
                .fill(glass_fill(BG_1_PANEL, accent, glass_alpha_window()))
                .stroke(egui::Stroke::new(1.0, widget_border(accent)))
                .corner_radius(egui::CornerRadius::same(radius::LG))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 10],
                    blur: 28,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(150),
                });
            frame.show(ui, |ui| {
                ui.set_width(WIDTH - 16.0);
                // Search input — plain TextEdit since we want
                // keyboard focus to land here automatically.
                let edit = egui::TextEdit::singleline(&mut state.query)
                    .desired_width(f32::INFINITY)
                    .frame(true)
                    .hint_text("Type a command…")
                    .background_color(glass_fill(
                        BG_2_RAISED,
                        accent,
                        glass_alpha_card(),
                    ))
                    .font(egui::FontId::proportional(13.0));
                let edit_resp = ui.add(edit);
                if edit_resp.changed() {
                    // Query changed — reset selection to the top
                    // of the filtered list so the highlight stays
                    // sensible.
                    state.selected = 0;
                }
                // Focus the text field the frame the palette
                // opens so the user can type immediately.
                if !edit_resp.has_focus() {
                    edit_resp.request_focus();
                }

                ui.add_space(4.0);

                // Results list.
                egui::ScrollArea::vertical()
                    .id_salt(("frost_palette_list", items_sig))
                    .auto_shrink([false, true])
                    .max_height(320.0)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        if filtered.is_empty() {
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new("No matches")
                                        .color(TEXT_SECONDARY)
                                        .size(font::BODY),
                                );
                            });
                        }
                        for (i, it) in filtered.iter().enumerate() {
                            if paint_row(ui, it, i == state.selected, accent).clicked() {
                                picked = Some(it.id);
                            }
                        }
                    });
            });
        });

    if picked.is_some() {
        state.open = false;
        state.query.clear();
        state.selected = 0;
    }

    picked
}

/// Paint one row: label on the left, optional dim hint on the
/// right. Selected row gets an accent-tinted fill so keyboard
/// navigation is visible.
fn paint_row(
    ui: &mut egui::Ui,
    item: &PaletteItem,
    selected: bool,
    accent: egui::Color32,
) -> egui::Response {
    const ROW_H: f32 = 24.0;
    let w = ui.available_width();
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(w, ROW_H), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let bg = if selected {
            let blend = |a: u8, b: u8| ((a as f32) * 0.6 + (b as f32) * 0.4).round() as u8;
            Some(egui::Color32::from_rgb(
                blend(BG_2_RAISED.r(), accent.r()),
                blend(BG_2_RAISED.g(), accent.g()),
                blend(BG_2_RAISED.b(), accent.b()),
            ))
        } else if resp.hovered() {
            Some(BG_2_RAISED)
        } else {
            None
        };
        if let Some(c) = bg {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::same(4), c);
        }
        let mid_y = rect.center().y;
        ui.painter().text(
            egui::pos2(rect.min.x + 10.0, mid_y),
            egui::Align2::LEFT_CENTER,
            item.label,
            egui::FontId::proportional(font::BODY + 2.0),
            TEXT_PRIMARY,
        );
        if let Some(hint) = item.hint {
            ui.painter().text(
                egui::pos2(rect.max.x - 10.0, mid_y),
                egui::Align2::RIGHT_CENTER,
                hint,
                egui::FontId::proportional(font::CAPTION),
                TEXT_SECONDARY,
            );
        }
    }
    resp
}

/// Fold every item's static `id` into a single `u64`. Used as
/// an Area / ScrollArea id discriminator so egui's cached sizes
/// / scroll offsets for one palette context don't bleed into a
/// different context the next frame.
fn items_fingerprint(items: &[PaletteItem]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    items.len().hash(&mut h);
    for it in items {
        it.id.hash(&mut h);
    }
    h.finish()
}

/// Substring + initials match. Returns true if the LOWERCASE
/// `label` contains `q` as a substring, OR if `q` matches the
/// initials of the label's whitespace-separated tokens.
fn matches(label: &str, q: &str) -> bool {
    let lower = label.to_lowercase();
    if lower.contains(q) {
        return true;
    }
    // Build initials: first char of each alphabetic token.
    let initials: String = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|w| w.chars().next())
        .collect();
    if initials.contains(q) {
        return true;
    }
    false
}
