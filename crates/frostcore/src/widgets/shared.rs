//! Small helpers shared across widgets. Kept `pub(super)` so they
//! don't leak into the public surface — each widget module picks
//! what it needs.

use egui;

use crate::style::BORDER_SUBTLE;

/// Key under which the "pending trailing separator" flag lives in
/// egui temp data, scoped to the current `ui`'s id. Stored value is
/// the `cumulative_pass_nr` at which the mark was set, so a stale
/// mark left over from a previous frame (e.g. the last widget
/// disappeared between frames) is ignored on the current frame.
fn pending_separator_key(ui: &egui::Ui) -> egui::Id {
    ui.id().with("frost_pending_separator")
}

/// Per-row state tracked by [`flush_pending_separator`] so the
/// active theme's optional zebra-stripe row backdrop can paint
/// without each widget knowing about it. Stored at
/// [`row_zebra_key`] for the duration of one section body.
///
/// The key insight: a row's bottom Y isn't known until the NEXT
/// row's `flush_pending_separator` call runs. So each `flush` does
/// two things:
///
/// 1. Resolve the **previous** row's zebra by setting its reserved
///    `Shape::Noop` to a `rect_filled` spanning the previous row's
///    Y range × the section's full inner width.
/// 2. Reserve a fresh `Shape::Noop` placeholder + record this row's
///    starting Y, ready for the *next* `flush` (or for the
///    section-body finalize call) to commit.
///
/// `pass_nr` is stored so a stale entry left from a previous frame
/// (where the section may have been collapsed) is ignored cleanly.
#[derive(Clone, Copy)]
pub(super) struct RowZebraState {
    pub pass_nr: u64,
    pub start_y: f32,
    pub min_x: f32,
    pub width: f32,
    pub shape_idx: egui::layers::ShapeIdx,
    pub row_index: u32,
    /// Id of the ui inside which the zebra mechanism is active.
    /// Captured on the FIRST `flush_pending_separator` call after
    /// `begin_row_zebra`, then enforced — any nested ui (subsection
    /// body, group, hybrid select) that calls `flush_pending_separator`
    /// with a different id is treated as zebra-inert. That stops
    /// nested-ui flushes from corrupting the outer row counter (a
    /// subsection's rect lives in a different cursor space, so a
    /// mixed-ui resolve would paint zebra fills with the wrong x or
    /// height).
    pub owner_ui: egui::Id,
}

/// Key for the zebra-tracking entry. Hardcoded (single-string id)
/// rather than mixed with `ui.id()` because the zebra mechanism
/// only meaningfully scopes to a single section body — when the
/// next section starts, [`begin_row_zebra`] resets it explicitly.
pub(super) fn row_zebra_key() -> egui::Id {
    egui::Id::new("frost_row_zebra")
}

/// Reset the zebra tracker and arm it for the next section body.
/// Stores a sentinel state under [`row_zebra_key`] whose `owner_ui`
/// is set to the *body ui* id passed in — the very next
/// `flush_pending_separator(ui)` call from inside that ui will
/// upgrade the sentinel into a live tracker. `flush` calls from
/// nested uis (different `ui.id()`) skip zebra entirely.
pub(crate) fn begin_row_zebra(ui: &mut egui::Ui) {
    let pass = ui.ctx().cumulative_pass_nr();
    let sentinel = RowZebraState {
        pass_nr: pass,
        start_y: 0.0,
        min_x: 0.0,
        width: 0.0,
        // Reserve a dummy placeholder so the field type is valid; it
        // never gets resolved (the first real flush replaces the
        // whole state).
        shape_idx: ui.painter().add(egui::Shape::Noop),
        row_index: u32::MAX, // sentinel — first real flush rolls over to 0
        owner_ui: ui.id(),
    };
    ui.ctx()
        .data_mut(|d| d.insert_temp(row_zebra_key(), sentinel));
}

/// Resolve the FINAL pending zebra entry (if any) at the end of a
/// section body. Without this, the last row in a section never
/// closes — a one-row visual quirk in long lists. Called from
/// `widgets::foldable::section_tracked` after body() returns.
pub(crate) fn commit_row_zebra(ui: &mut egui::Ui, accent: egui::Color32) {
    let key = row_zebra_key();
    let pass = ui.ctx().cumulative_pass_nr();
    let prev: Option<RowZebraState> = ui.ctx().data(|d| d.get_temp(key));
    if let Some(p) = prev {
        if p.pass_nr == pass && p.row_index != u32::MAX {
            paint_zebra_into_placeholder(ui, p, accent, ui.cursor().min.y);
        }
    }
    ui.ctx().data_mut(|d| d.remove::<RowZebraState>(key));
}

fn paint_zebra_into_placeholder(
    ui: &egui::Ui,
    state: RowZebraState,
    accent: egui::Color32,
    bottom_y: f32,
) {
    if let Some(fill) = crate::style::row_alt_fill(accent, state.row_index) {
        let rect = egui::Rect::from_min_max(
            egui::pos2(state.min_x, state.start_y),
            egui::pos2(state.min_x + state.width, bottom_y),
        );
        ui.painter().set(
            state.shape_idx,
            egui::Shape::rect_filled(rect, egui::CornerRadius::ZERO, fill),
        );
    }
    // If no fill (even row or alternation off), leave the Noop alone.
}

/// Trailing divider marker appended by every widget module. Does
/// NOT paint immediately — it only records that *this frame* the
/// current `ui` has a pending trailing separator. The paint is
/// performed lazily by [`flush_pending_separator`] at the START of
/// whichever widget comes next.
///
/// Consequence: if nothing follows (the mark is the last thing in
/// its container), the mark simply decays without ever being
/// painted — so a container's last row auto-hides its trailing
/// divider without the caller needing to know or annotate it.
///
/// Also re-exported publicly as [`super::row_separator`] so callers
/// who assemble bespoke inline row layouts can request the matching
/// divider with the same smart behaviour.
pub(super) fn widget_separator(ui: &mut egui::Ui) {
    let pass = ui.ctx().cumulative_pass_nr();
    let key = pending_separator_key(ui);
    ui.ctx().data_mut(|d| d.insert_temp::<u64>(key, pass));
}

/// Paint the deferred trailing separator — if any — that the prior
/// widget marked on this same frame. Call this at the very start of
/// every widget body. Idempotent; cheap no-op when no mark is
/// pending or the mark is stale. Clears the mark after handling so
/// subsequent calls on the same frame (e.g. during a re-run inside
/// the same pass) don't double-paint.
pub(super) fn flush_pending_separator(ui: &mut egui::Ui) {
    let key = pending_separator_key(ui);
    let current = ui.ctx().cumulative_pass_nr();
    let stored: Option<u64> = ui.ctx().data(|d| d.get_temp::<u64>(key));
    if stored.is_some() {
        ui.ctx().data_mut(|d| d.remove::<u64>(key));
    }
    if stored == Some(current) {
        paint_hairline(ui);
    }

    // Zebra row tracking — runs *after* the optional hairline so the
    // row's start Y already accounts for the separator's allocated
    // space. Cheap when alternation is off (one ctx-data peek + one
    // shape allocation per row).
    advance_row_zebra(ui);
}

/// Close off the previous row's zebra fill (if any) and arm a fresh
/// `Shape::Noop` placeholder for the row that's about to render. No
/// painting happens here for the row currently arming — the actual
/// fill is committed on the NEXT `flush_pending_separator` call (or
/// by `commit_row_zebra` at section finalize), once the row's
/// bottom Y is known.
fn advance_row_zebra(ui: &mut egui::Ui) {
    let key = row_zebra_key();
    let pass = ui.ctx().cumulative_pass_nr();
    let prev: Option<RowZebraState> = ui.ctx().data(|d| d.get_temp(key));

    // No active section body — bail. Section bodies install a
    // sentinel via `begin_row_zebra`; without that, this `flush`
    // call is happening outside of any frost section (statusbar,
    // floating window chrome, etc.) and shouldn't paint zebra.
    let Some(prev) = prev else { return };
    if prev.pass_nr != pass {
        return;
    }
    // Foreign ui — a subsection / group / hybrid-select-row body
    // that hosts its own row stack. Skip zebra so the outer
    // section's rect math stays correct.
    if prev.owner_ui != ui.id() {
        return;
    }

    let cursor_top = ui.cursor().min.y;
    let accent = ui.visuals().selection.stroke.color;
    // Sentinel `row_index = u32::MAX` is the just-after-`begin`
    // state — there's no previous row to commit; just arm row 0.
    if prev.row_index != u32::MAX {
        paint_zebra_into_placeholder(ui, prev, accent, cursor_top);
    }

    let shape_idx = ui.painter().add(egui::Shape::Noop);
    let row_index = if prev.row_index == u32::MAX {
        0
    } else {
        prev.row_index.wrapping_add(1)
    };
    let next = RowZebraState {
        pass_nr: pass,
        start_y: cursor_top,
        min_x: ui.cursor().min.x,
        width: ui.available_width(),
        shape_idx,
        row_index,
        owner_ui: prev.owner_ui,
    };
    ui.ctx().data_mut(|d| d.insert_temp(key, next));
}

/// Discard any pending trailing separator WITHOUT painting it. Used
/// by the resize-grip separator so a widget stack above + grip below
/// doesn't stack a hairline on top of the grip — the grip itself IS
/// the visual separator at that boundary.
pub(super) fn clear_pending_separator(ui: &mut egui::Ui) {
    let key = pending_separator_key(ui);
    ui.ctx().data_mut(|d| d.remove::<u64>(key));
}

/// Hairline trailing divider painted by [`flush_pending_separator`].
/// Driven by `theme().row_separator_alpha` (NOT `border_alpha` /
/// `border_width`) so a theme can hide every panel / section /
/// widget outline while still keeping faint row dividers — the
/// exact split GAME wants. Alpha 0 collapses the line entirely
/// (cadence-only mode); any positive alpha paints a 1 px hairline
/// in `border_subtle` with that alpha.
fn paint_hairline(ui: &mut egui::Ui) {
    let th = crate::style::theme();
    if th.row_separator_alpha == 0 {
        ui.add_space(3.0);
        return;
    }
    ui.add_space(1.0);
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 1.0), egui::Sense::hover());
    // Use the kit-wide [`crate::style::outline_base`] helper —
    // single source of truth for the mode-aware "lerp 50 % toward
    // the opposite luma extreme" base colour. Same base every
    // border, divider, and separator in the kit uses; each just
    // applies its own alpha on top.
    let base = crate::style::outline_base();
    let color = egui::Color32::from_rgba_unmultiplied(
        base.r(),
        base.g(),
        base.b(),
        th.row_separator_alpha,
    );
    let stroke = egui::Stroke::new(1.0, color);
    if let Some((on, off)) = th.row_separator_dash {
        crate::style::paint_dashed_line(
            ui.painter(),
            rect.left_center(),
            rect.right_center(),
            on,
            off,
            stroke,
        );
    } else {
        ui.painter()
            .line_segment([rect.left_center(), rect.right_center()], stroke);
    }
    ui.add_space(1.0);
}

/// Linear colour interpolation across RGBA channels. `t` is clamped
/// to `0.0..=1.0`.
pub(super) fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    egui::Color32::from_rgba_premultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

/// Paint a track + accent-filled portion with a centred value
/// readout. Shared by [`super::slider`] (which layers interaction
/// on top) and [`super::progressbar`] (which doesn't).
///
/// The text is painted twice — once per side of the fill edge —
/// each time clipped to that side's rect. Callers pass two colours
/// so the readout reads cleanly whichever colour it lands on.
pub(super) fn paint_value_bar(
    ui: &egui::Ui,
    rect: egui::Rect,
    fill_fraction: f32,
    text: &str,
    font: egui::FontId,
    accent: egui::Color32,
    track_text_color: egui::Color32,
    fill_text_color: egui::Color32,
    corner_radius: u8,
) {
    let painter = ui.painter_at(rect);
    let fraction = fill_fraction.clamp(0.0, 1.0);
    let fill_w = rect.width() * fraction;

    // Unfilled track — `track_fill(accent)` resolves PRO to the
    // dark `bg_input` and GAME to a dim-accent shade, so the
    // unfilled portion of every slider / progress bar reads as part
    // of the same colour family as the panel it lives on.
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(corner_radius),
        crate::style::track_fill(accent),
    );

    // Accent fill pinned to the left. Uses `body_accent(accent)` so
    // GAME's progress-bar / slider fills come out a touch darker
    // than the title banner accent, keeping the banner the brightest
    // tier in each card.
    if fill_w > 0.5 {
        let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, rect.height()));
        painter.rect_filled(
            fill_rect,
            egui::CornerRadius::same(corner_radius),
            crate::style::body_accent(accent),
        );
    }

    // Text — two passes, each clipped so the colour switches
    // cleanly at the fill edge. `painter_at` restricts the
    // sub-painter's clip rect, so draws outside the sub-rect are
    // hidden.
    let center = rect.center();

    if fraction < 1.0 {
        let track_sub = egui::Rect::from_min_max(
            egui::pos2(rect.min.x + fill_w, rect.min.y),
            rect.max,
        );
        ui.painter_at(track_sub).text(
            center,
            egui::Align2::CENTER_CENTER,
            text,
            font.clone(),
            track_text_color,
        );
    }
    if fraction > 0.0 {
        let fill_sub = egui::Rect::from_min_size(rect.min, egui::vec2(fill_w, rect.height()));
        ui.painter_at(fill_sub).text(
            center,
            egui::Align2::CENTER_CENTER,
            text,
            font,
            fill_text_color,
        );
    }
}
