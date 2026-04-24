//! Tree row — one row of a hierarchical list, rendered with
//! per-depth indent guides, a chevron for expandable nodes, an
//! optional type-icon slot, a truncating label, and a right-aligned
//! gutter of **uniform icon toggles** (eye, lock, …) that are
//! identical in count and kind across every row in the tree.
//!
//! Uniformity is structural: the `slots` slice defines how many
//! icons each row has, and callers pass the same shape to every
//! `tree_row` call in one list. There is no per-row escape hatch —
//! the Blender 4 / UE5 / Maya outliner pattern is "every row has
//! the same gutter controls", so the widget enforces it.
//!
//! Stateless by design — the caller owns the tree data **and** the
//! expansion state. You walk your model, call [`tree_row`] per node,
//! and recurse into its children when the row reports expanded. This
//! matches the outliner shapes in Blender 4, UE5 Outliner, and the
//! USD stage viewer: the data structure and collapse state live in
//! the caller's domain (a `HashSet<PrimPath>`, a `bool` on each
//! node, …) so nothing leaks into widget storage.
//!
//! Shape:
//! ```text
//!   │  │  ▸  ▲ Robot                                 👁 🔒
//!   │  │  │  └── label (body click target, selects)  └─ uniform icon gutter
//!   │  │  └── chevron (expand click target, independent of body)
//!   │  └── type-icon slot (optional glyph painted in the accent colour)
//!   └── indent guides (depth × TREE_INDENT)
//! ```
//!
//! The row's **background fill** (hover / selection tint) is painted
//! on a reserved z-slot *before* the inline widgets draw — so action
//! buttons inside the right gutter read as buttons on top of the
//! row's selection tint, not as glyphs with their own competing
//! background.

use std::hash::Hash;

use egui;

use crate::style::{BG_2_RAISED, BG_3_HOVER, BORDER_SUBTLE, TEXT_PRIMARY, TEXT_SECONDARY};

/// Row height — matches [`super::hybrid_select::HYBRID_SELECT_ROW_H`]
/// so trees and outliner-style lists stack at the same rhythm.
pub const TREE_ROW_H: f32 = 20.0;

/// Horizontal pixels per depth level — indent guides are painted at
/// `depth * TREE_INDENT` from the row's left edge.
pub const TREE_INDENT: f32 = 12.0;

/// Width of the chevron hit-rect. Leaves reserve the same width so
/// labels column-align with their expanded siblings.
const CHEVRON_W: f32 = 12.0;
/// Width of the optional type-icon slot. When [`tree_row`]'s `icon`
/// is `None` the slot collapses to zero.
const ICON_W: f32 = 14.0;
/// Padding between the icon/chevron column and the label.
const LABEL_PAD_L: f32 = 4.0;
/// Width of each right-gutter action-icon slot (square hit rect).
const SLOT_W: f32 = 16.0;
/// Horizontal gap between adjacent right-gutter slots.
const SLOT_GAP: f32 = 2.0;
/// Right-side inner padding — keeps the last action icon off the
/// row's rounded edge.
const RIGHT_PAD_R: f32 = 4.0;
/// Left inset before the first indent guide / chevron — so the
/// chevron doesn't kiss the panel's left edge.
const ROW_PAD_L: f32 = 4.0;

/// Which built-in icon to paint in a [`TreeIconSlot`]. Built-ins are
/// drawn with painter shapes so they work identically regardless of
/// which font subset is installed. Use [`TreeIconKind::Glyph`] as an
/// escape hatch for anything not covered by the named variants.
#[derive(Debug, Clone, Copy)]
pub enum TreeIconKind {
    /// Eye icon — almond outline + pupil when active (visible),
    /// outline + diagonal slash when inactive (hidden).
    Eye,
    /// Padlock — body + closed shackle when active (locked), body +
    /// tilted open shackle when inactive (unlocked).
    Lock,
    /// Custom pair of font glyphs. Painted in the current text font
    /// at `12 px`. Use for icons you don't want to hand-paint.
    Glyph { on: &'static str, off: &'static str },
    /// Read-only colour swatch — a filled rounded square in the
    /// given colour, with the standard frost border stroke. The
    /// slot's `state: &mut bool` is ignored for this variant
    /// (still required by the slice shape — pass any `&mut bool`);
    /// the icon response is returned in
    /// [`TreeRowResponse::icons`] so callers can act on clicks
    /// (e.g. "select the material that this swatch represents").
    Color(egui::Color32),
}

/// One slot in the right-gutter of a [`tree_row`]. The widget paints
/// the appropriate icon for the current `state`, flips `state` in
/// place on click, and returns the slot's `Response` in
/// [`TreeRowResponse::icons`] so callers can hook additional
/// side-effects.
///
/// The *same* slice shape (length + kinds) should be passed to every
/// `tree_row` call in one tree — that's how the widget enforces the
/// "every row has the same gutter" outliner convention.
pub struct TreeIconSlot<'a> {
    /// Which icon to paint.
    pub kind: TreeIconKind,
    /// Active state — flipped in place on click. Use the same field
    /// you'd normally bind to a [`super::toggle`].
    pub state: &'a mut bool,
    /// Hover-text description (rendered via `.on_hover_text`). Pass
    /// `None` to omit.
    pub tooltip: Option<&'static str>,
}

impl<'a> TreeIconSlot<'a> {
    /// Shorthand: `TreeIconSlot::new(kind, &mut flag)` with no tooltip.
    pub fn new(kind: TreeIconKind, state: &'a mut bool) -> Self {
        Self { kind, state, tooltip: None }
    }

    /// Builder-style tooltip setter.
    pub fn with_tooltip(mut self, text: &'static str) -> Self {
        self.tooltip = Some(text);
        self
    }
}

/// The click targets produced by [`tree_row`]. Inspect `body` for
/// select / double-click / drag, `chevron` (when `Some`) for
/// expand-toggle, and `icons[i]` for the `i`-th right-gutter slot.
/// Leaves get `chevron == None` and reserve the chevron column as
/// blank space so labels align with branches.
#[derive(Debug)]
pub struct TreeRowResponse {
    /// Click target covering the label + type-icon area.
    pub body: egui::Response,
    /// Click target for the chevron glyph only. `None` for leaves.
    pub chevron: Option<egui::Response>,
    /// One `Response` per entry in the `slots` slice. The widget has
    /// already toggled each slot's `state` for you on click; this
    /// response is for additional hooks (e.g. "on visibility change,
    /// also hide children").
    pub icons: Vec<egui::Response>,
    /// `true` when the chevron was clicked with the shift modifier
    /// held. Callers interpret as "recursively expand (or collapse)
    /// the whole subtree under this row" — the standard editor
    /// affordance for jumping straight to a deep prim on a scene
    /// with many levels. Always `false` for leaf rows (no chevron).
    pub chevron_shift_clicked: bool,
}

/// Paint one row of a tree. `depth` is the node's nesting level
/// (0 = root); `expanded` is `Some(&mut bool)` for branches and
/// `None` for leaves. `icon` is an optional type-indicator glyph
/// painted between the chevron column and the label. `slots` is
/// the fixed-width right gutter of action toggles — pass the same
/// slice shape for every row in the tree.
///
/// The row paints its own hover + selection fill and a 1 px indent
/// guide per depth level. Caller walks their tree model and calls
/// this once per visible node; expansion state lives with the
/// caller so the widget has no hidden storage.
pub fn tree_row(
    ui: &mut egui::Ui,
    id_salt: impl Hash + Copy,
    depth: u32,
    expanded: Option<&mut bool>,
    icon: Option<&str>,
    label: &str,
    selected: bool,
    accent: egui::Color32,
    slots: &mut [TreeIconSlot<'_>],
) -> TreeRowResponse {
    let w = ui.available_width();

    // Reserve z-slots for the row background fill + indent guides
    // BEFORE inline widgets draw. We don't know the exact rect yet,
    // so we `add(Noop)` now and `set(...)` once the rect is known —
    // same trick egui uses internally for selection fills.
    let bg_anchor = ui.painter().add(egui::Shape::Noop);
    let guide_anchor = ui.painter().add(egui::Shape::Noop);

    let (rect, body_rect, chevron_rect_opt, icon_rect_opt, slot_rects) =
        compute_row_rects(ui, w, depth, expanded.is_some(), icon.is_some(), slots.len());

    // Interactions — body, chevron, per-slot icons. Each has its own
    // id salt so clicks on the gutter never leak into body selection
    // and vice-versa.
    let body = ui.interact(
        body_rect,
        ui.id().with(("frost_tree_body", id_salt)),
        egui::Sense::click(),
    );
    let chevron = chevron_rect_opt.map(|cr| {
        ui.interact(
            cr,
            ui.id().with(("frost_tree_chevron", id_salt)),
            egui::Sense::click(),
        )
    });
    let mut icon_responses: Vec<egui::Response> = Vec::with_capacity(slots.len());
    for (i, slot_rect) in slot_rects.iter().enumerate() {
        let mut r = ui.interact(
            *slot_rect,
            ui.id().with(("frost_tree_slot", id_salt, i)),
            egui::Sense::click(),
        );
        if let Some(tip) = slots[i].tooltip {
            r = r.on_hover_text(tip);
        }
        icon_responses.push(r);
    }

    // Background fill: paint under everything via the reserved slot.
    let any_slot_hovered = icon_responses.iter().any(|r| r.hovered());
    let hovered = body.hovered()
        || chevron.as_ref().map_or(false, |c| c.hovered())
        || any_slot_hovered;
    let bg_shape = if selected {
        let blend = |a: u8, b: u8| ((a as f32) * 0.60 + (b as f32) * 0.40).round() as u8;
        let tint = egui::Color32::from_rgb(
            blend(BG_2_RAISED.r(), accent.r()),
            blend(BG_2_RAISED.g(), accent.g()),
            blend(BG_2_RAISED.b(), accent.b()),
        );
        egui::Shape::rect_filled(rect, egui::CornerRadius::same(2), tint)
    } else if hovered {
        egui::Shape::rect_filled(rect, egui::CornerRadius::same(2), BG_3_HOVER)
    } else {
        egui::Shape::Noop
    };
    ui.painter().set(bg_anchor, bg_shape);

    // Indent guides: faint vertical lines at each ancestor depth so
    // the reader can tell which branch a deep row descends from.
    let guide_color = egui::Color32::from_rgba_unmultiplied(
        BORDER_SUBTLE.r(),
        BORDER_SUBTLE.g(),
        BORDER_SUBTLE.b(),
        90,
    );
    let mut guides = Vec::with_capacity(depth as usize);
    for d in 0..depth {
        let x = rect.min.x + ROW_PAD_L + d as f32 * TREE_INDENT + CHEVRON_W * 0.5;
        guides.push(egui::Shape::line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            egui::Stroke::new(1.0, guide_color),
        ));
    }
    ui.painter().set(
        guide_anchor,
        if guides.is_empty() {
            egui::Shape::Noop
        } else {
            egui::Shape::Vec(guides)
        },
    );

    // Chevron glyph: small rotating triangle. Handled AFTER the
    // chevron interaction has been recorded so the click routing
    // knows its own rect. Shift-click is captured separately so
    // callers can implement "expand / collapse whole subtree"
    // without having to plumb modifier state themselves.
    let mut chevron_shift_clicked = false;
    if let (Some(exp), Some(cr)) = (expanded, chevron_rect_opt) {
        let how_open = ui.ctx().animate_bool_responsive(
            ui.id().with(("frost_tree_chev_anim", id_salt)),
            *exp,
        );
        paint_chevron(ui, cr, how_open, accent);
        if let Some(ref cresp) = chevron {
            if cresp.clicked() {
                let shift_held = ui.ctx().input(|i| i.modifiers.shift);
                if shift_held {
                    chevron_shift_clicked = true;
                    // Don't toggle the local expanded flag — the
                    // caller handles "apply to whole subtree".
                } else {
                    *exp = !*exp;
                }
            }
        }
    }

    // Type-icon slot: centred in its reserved width, painted in the
    // accent colour so group / mesh / light glyphs tint with the
    // current accent.
    if let (Some(glyph), Some(ir)) = (icon, icon_rect_opt) {
        ui.painter().text(
            ir.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::proportional(12.0),
            accent,
        );
    }

    // Label: truncated to the body rect minus its left padding. Only
    // paint when the row is actually inside the parent's clip — the
    // painter would clip it anyway, but building the galley has a
    // cost we can skip for off-screen rows.
    let parent_clip = ui.clip_rect();
    if parent_clip.intersects(rect) {
        let label_left = body_rect.min.x
            + ROW_PAD_L
            + depth as f32 * TREE_INDENT
            + CHEVRON_W
            + if icon.is_some() { ICON_W } else { 0.0 }
            + LABEL_PAD_L;
        let label_rect = egui::Rect::from_min_max(
            egui::pos2(label_left, rect.min.y),
            egui::pos2(body_rect.max.x, rect.max.y),
        );
        let label_color = TEXT_PRIMARY;
        let font = egui::FontId::proportional(12.0);
        let galley = {
            let mut job = egui::text::LayoutJob::single_section(
                label.to_string(),
                egui::TextFormat::simple(font, label_color),
            );
            job.wrap.max_width = label_rect.width().max(0.0);
            job.wrap.max_rows = 1;
            job.wrap.break_anywhere = true;
            job.halign = egui::Align::LEFT;
            ui.painter().layout_job(job)
        };
        ui.painter().galley(
            egui::pos2(label_rect.min.x, label_rect.center().y - galley.size().y * 0.5),
            galley,
            label_color,
        );

        // Right-gutter slots. Paint each icon into its reserved
        // square rect. Click was already routed above; flip state
        // after we've painted so the current frame's paint reflects
        // the PRE-click state (one frame lag is imperceptible and
        // avoids a tri-state flicker from toggling mid-frame).
        for (i, slot) in slots.iter_mut().enumerate() {
            let rect = slot_rects[i];
            let resp = &icon_responses[i];
            paint_slot_icon(ui, rect, &slot.kind, *slot.state, resp.hovered(), accent);
        }
    }

    // Flip the slot states after painting. Using `resp.clicked()` so
    // hover alone doesn't flip. `Color` slots are read-only — the
    // response is still returned (caller can hook click handlers),
    // but the backing `bool` stays put.
    for (i, slot) in slots.iter_mut().enumerate() {
        if icon_responses[i].clicked() && !matches!(slot.kind, TreeIconKind::Color(_)) {
            *slot.state = !*slot.state;
        }
    }

    TreeRowResponse {
        body,
        chevron,
        icons: icon_responses,
        chevron_shift_clicked,
    }
}

/// Allocate the row rect and slice it into the fixed left sub-rects
/// (indent + chevron + icon + label area) and the right-gutter slot
/// rects.
fn compute_row_rects(
    ui: &mut egui::Ui,
    w: f32,
    depth: u32,
    has_chevron: bool,
    has_icon: bool,
    slot_count: usize,
) -> (
    egui::Rect,
    egui::Rect,
    Option<egui::Rect>,
    Option<egui::Rect>,
    Vec<egui::Rect>,
) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(w, TREE_ROW_H),
        egui::Sense::hover(),
    );
    let left_start = rect.min.x + ROW_PAD_L + depth as f32 * TREE_INDENT;
    let chevron_rect = if has_chevron {
        Some(egui::Rect::from_min_size(
            egui::pos2(left_start, rect.min.y),
            egui::vec2(CHEVRON_W, rect.height()),
        ))
    } else {
        None
    };
    let icon_rect = if has_icon {
        Some(egui::Rect::from_min_size(
            egui::pos2(left_start + CHEVRON_W, rect.min.y),
            egui::vec2(ICON_W, rect.height()),
        ))
    } else {
        None
    };

    // Right gutter: exactly `slot_count` squares, separated by
    // SLOT_GAP, flush-right with RIGHT_PAD_R padding. Width is
    // deterministic, so body_rect never needs a post-paint shrink.
    let gutter_w = if slot_count == 0 {
        0.0
    } else {
        slot_count as f32 * SLOT_W + (slot_count as f32 - 1.0) * SLOT_GAP + RIGHT_PAD_R
    };
    let mut slot_rects = Vec::with_capacity(slot_count);
    if slot_count > 0 {
        // First (rightmost) slot hugs `rect.max.x - RIGHT_PAD_R`;
        // subsequent slots step leftward.
        let mut x_max = rect.max.x - RIGHT_PAD_R;
        for _ in 0..slot_count {
            let x_min = x_max - SLOT_W;
            slot_rects.push(egui::Rect::from_min_max(
                egui::pos2(x_min, rect.min.y),
                egui::pos2(x_max, rect.max.y),
            ));
            x_max = x_min - SLOT_GAP;
        }
        // Paint in left-to-right order so `slots[0]` is LEFTMOST —
        // matches reading order. The rects were built right-to-left,
        // so reverse.
        slot_rects.reverse();
    }

    let body_rect = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x - gutter_w, rect.max.y),
    );
    (rect, body_rect, chevron_rect, icon_rect, slot_rects)
}

/// Paint a small rotating triangle chevron inside `rect`. `how_open`
/// is 0.0 (closed, ▸) → 1.0 (open, ▾); fractional values rotate.
fn paint_chevron(ui: &egui::Ui, rect: egui::Rect, how_open: f32, accent: egui::Color32) {
    let c = rect.center();
    let r = 3.2_f32;
    let angle = how_open * std::f32::consts::FRAC_PI_2;
    let rot = |p: egui::Vec2| -> egui::Pos2 {
        let (s, co) = (angle.sin(), angle.cos());
        egui::pos2(
            c.x + p.x * co - p.y * s,
            c.y + p.x * s + p.y * co,
        )
    };
    let a = rot(egui::vec2(r, 0.0));
    let b = rot(egui::vec2(-r * 0.7, -r * 0.7));
    let d = rot(egui::vec2(-r * 0.7, r * 0.7));
    let fill = lerp_color(
        egui::Color32::from_gray(170),
        accent,
        how_open.clamp(0.0, 1.0),
    );
    ui.painter().add(egui::Shape::convex_polygon(
        vec![a, b, d],
        fill,
        egui::Stroke::NONE,
    ));
}

/// Paint the correct icon for a slot's current kind + state. Hovering
/// brightens the icon toward `accent`; active-off states are muted
/// (dim grey) so the row reads "this thing is OFF" at a glance.
fn paint_slot_icon(
    ui: &egui::Ui,
    rect: egui::Rect,
    kind: &TreeIconKind,
    active: bool,
    hovered: bool,
    accent: egui::Color32,
) {
    let color = slot_color(active, hovered, accent);
    match *kind {
        TreeIconKind::Eye => paint_eye(ui, rect, active, color),
        TreeIconKind::Lock => paint_lock(ui, rect, active, color),
        TreeIconKind::Glyph { on, off } => {
            let glyph = if active { on } else { off };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::proportional(12.0),
                color,
            );
        }
        TreeIconKind::Color(fill) => paint_color_chip(ui, rect, fill, accent, hovered),
    }
}

/// Filled rounded swatch in `fill`, with the shared accent-tinted
/// border stroke so it belongs to the same chip family as the
/// rest of the frost surfaces. Inset a touch from the slot rect
/// so it reads as a chip, not a toggle button.
fn paint_color_chip(
    ui: &egui::Ui,
    rect: egui::Rect,
    fill: egui::Color32,
    accent: egui::Color32,
    hovered: bool,
) {
    let inner = rect.shrink(3.0);
    let border = if hovered {
        accent
    } else {
        crate::style::widget_border(accent)
    };
    ui.painter().rect(
        inner,
        egui::CornerRadius::same(2),
        fill,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
}

fn slot_color(active: bool, hovered: bool, accent: egui::Color32) -> egui::Color32 {
    match (active, hovered) {
        (true, true) => accent,
        (true, false) => TEXT_PRIMARY,
        (false, true) => lerp_color(TEXT_SECONDARY, accent, 0.4),
        (false, false) => TEXT_SECONDARY,
    }
}

/// Eye icon — a horizontal almond outline + a small pupil disc when
/// active. When inactive, the almond is drawn with a diagonal slash
/// through it to read as "hidden". Sized to fit comfortably inside
/// [`SLOT_W`] square with a few px of breathing room.
fn paint_eye(ui: &egui::Ui, rect: egui::Rect, active: bool, color: egui::Color32) {
    let c = rect.center();
    let rx = 5.5_f32;
    let ry = 3.2_f32;
    let stroke = egui::Stroke::new(1.1, color);

    // Almond = two symmetric circular arcs meeting at the horizontal
    // tips. Approximated as cubic-sampled polylines — cheap and
    // readable at 16 px.
    let lid = |sign: f32| {
        let mut pts = Vec::with_capacity(11);
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let x = c.x + (t - 0.5) * 2.0 * rx;
            // Parabolic lid — shallow curve through ±ry at x=c.x.
            let y = c.y + sign * ry * (1.0 - ((x - c.x) / rx).powi(2));
            pts.push(egui::pos2(x, y));
        }
        egui::Shape::line(pts, stroke)
    };
    ui.painter().add(lid(1.0));
    ui.painter().add(lid(-1.0));

    if active {
        // Pupil — small filled circle at centre. Radius slightly
        // under `ry` so it doesn't touch the lid.
        ui.painter().circle_filled(c, 1.6, color);
    } else {
        // Hidden: diagonal slash from upper-left to lower-right,
        // extending a touch past the almond for emphasis.
        ui.painter().line_segment(
            [
                egui::pos2(c.x - rx - 0.5, c.y + ry + 0.5),
                egui::pos2(c.x + rx + 0.5, c.y - ry - 0.5),
            ],
            egui::Stroke::new(1.3, color),
        );
    }
}

/// Padlock icon — rounded body + shackle arc on top. When active
/// (locked) the shackle meets the body on both sides; when inactive
/// (unlocked) the right leg is lifted so the shackle reads as "open".
fn paint_lock(ui: &egui::Ui, rect: egui::Rect, active: bool, color: egui::Color32) {
    let c = rect.center();
    let body_w = 7.0_f32;
    let body_h = 5.5_f32;
    let body_top_y = c.y + 0.2;
    let body_rect = egui::Rect::from_min_size(
        egui::pos2(c.x - body_w * 0.5, body_top_y),
        egui::vec2(body_w, body_h),
    );
    // Body: filled rounded rect so the icon has a clear silhouette
    // even at low sizes.
    ui.painter().rect_filled(
        body_rect,
        egui::CornerRadius::same(1),
        color,
    );

    // Shackle: an inverted-U painted above the body. Two vertical
    // legs + a top arc. For the unlocked state, shift the right leg
    // up-and-right so it reads as "disengaged".
    let stroke = egui::Stroke::new(1.1, color);
    let shackle_top_y = body_top_y - 4.0;
    let legs_x_l = c.x - 2.4;
    let legs_x_r = c.x + 2.4;

    // Top arc — short polyline approximation of a half-circle.
    let mut arc = Vec::with_capacity(7);
    for i in 0..=6 {
        let t = i as f32 / 6.0;
        let theta = std::f32::consts::PI - t * std::f32::consts::PI;
        let x = c.x + theta.cos() * 2.4;
        let y = shackle_top_y - theta.sin() * 1.6;
        arc.push(egui::pos2(x, y));
    }
    ui.painter().add(egui::Shape::line(arc, stroke));

    // Left leg — always attached.
    ui.painter().line_segment(
        [
            egui::pos2(legs_x_l, shackle_top_y),
            egui::pos2(legs_x_l, body_top_y + 0.3),
        ],
        stroke,
    );
    // Right leg — attached (locked) or lifted (unlocked).
    if active {
        ui.painter().line_segment(
            [
                egui::pos2(legs_x_r, shackle_top_y),
                egui::pos2(legs_x_r, body_top_y + 0.3),
            ],
            stroke,
        );
    } else {
        // Lifted: stops short of the body, suggesting the shackle
        // has popped open on that side.
        ui.painter().line_segment(
            [
                egui::pos2(legs_x_r, shackle_top_y),
                egui::pos2(legs_x_r, shackle_top_y + 2.0),
            ],
            stroke,
        );
    }

    // Keyhole dot — tiny hollow mark on the body so the body doesn't
    // read as a plain rectangle.
    if active {
        ui.painter().circle_filled(
            egui::pos2(c.x, body_rect.center().y),
            0.7,
            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 140),
        );
    }
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| ((x as f32) * (1.0 - t) + (y as f32) * t).round() as u8;
    egui::Color32::from_rgba_premultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}
