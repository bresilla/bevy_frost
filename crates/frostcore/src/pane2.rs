//! Pane2 — flex-based pane skeleton (Phase 1 of `PLAN_NEWUI.md`).
//!
//! A floating pane that paints a theme-aware **title strip** in any
//! of 12 anchor positions (4 screen rails × 3 zones each) and
//! reserves the remainder for a body closure. Layout is delegated
//! entirely to [`crate::flex`] (vendored `egui_flex`) so children
//! always fit and the pane never overflows.
//!
//! This is the ground-up replacement for `crate::floating::PaneBuilder`
//! and lives alongside the existing pane code; the old code keeps
//! working until we reach Phase 5/6 of `PLAN_NEWUI.md`.

use egui::{Color32, Id, Rect, Stroke, Vec2, pos2, vec2};

use crate::flex::{Flex, FlexAlign, item};
use crate::style;

// ─── Anchor model ───────────────────────────────────────────────────

/// One of the 4 screen rails the pane can live on. The rail
/// determines the **pane orientation** (which edge the title strip
/// sits on) and the **rotation** of the title text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneAnchor {
    /// Vertical pane, title strip on the LEFT edge of the pane.
    LeftRail(RailZone),
    /// Vertical pane, title strip on the RIGHT edge.
    RightRail(RailZone),
    /// Horizontal pane, title strip at the TOP edge.
    TopRail(RailZone),
    /// Horizontal pane, title strip at the BOTTOM edge.
    BottomRail(RailZone),
}

/// Where on the rail the pane sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailZone {
    /// Top of a vertical rail / left of a horizontal rail.
    Start,
    /// Centred on the rail.
    Middle,
    /// Bottom of a vertical rail / right of a horizontal rail.
    End,
}

impl PaneAnchor {
    /// `true` if this rail's pane has a vertical title strip
    /// (left/right rail) — i.e. the **pane** itself is "tall".
    pub fn is_vertical_pane(self) -> bool {
        matches!(self, PaneAnchor::LeftRail(_) | PaneAnchor::RightRail(_))
    }

    /// Always `false` in Phase 1 — title strip sits at the start of
    /// the flex (top of a vertical pane, left side of a horizontal
    /// pane). Phase 4+ may flip per-anchor for more variety; for now
    /// every pane reads "title-then-body" so the layout is uniform.
    fn title_at_end(self) -> bool {
        false
    }
}

// ─── Sizing constants ───────────────────────────────────────────────

/// Pane size for vertical rails (LeftRail / RightRail).
pub const PANE_VERTICAL_SIZE: Vec2 = vec2(280.0, 320.0);
/// Pane size for horizontal rails (TopRail / BottomRail).
pub const PANE_HORIZONTAL_SIZE: Vec2 = vec2(560.0, 220.0);

/// Thickness of the title strip on its main axis (perpendicular to
/// the strip's reading direction). Same value for every rail so the
/// 4 orientations read at matching weight.
pub const TITLE_STRIP_THICKNESS: f32 = 25.0;

/// Inset from each screen edge that excludes the ribbon strip:
/// `EDGE_GAP + SIDE_BTN_SIZE + RAIL_PANEL_GAP`. Same recipe the
/// existing `floating::floating_window_scoped` uses, lifted into a
/// single named constant so all 4 rails clear their ribbon
/// uniformly.
pub const RAIL_INSET: f32 = crate::ribbon::EDGE_GAP
    + crate::ribbon::SIDE_BTN_SIZE
    + RAIL_PANEL_GAP;

/// Visual gap between the ribbon's button strip and the pane edge.
const RAIL_PANEL_GAP: f32 = 4.0;

// ─── Builder ────────────────────────────────────────────────────────

/// Pane2 — a single floating window keyed by `id` and pinned to one
/// of 12 screen positions. Build with [`Pane2::new`], chain optional
/// `.with_*` methods, then call [`Pane2::show`] each frame the pane
/// should be visible.
pub struct Pane2 {
    id: Id,
    title: String,
    anchor: PaneAnchor,
    accent: Color32,
}

impl Pane2 {
    pub fn new(
        id: impl Into<Id>,
        title: impl Into<String>,
        anchor: PaneAnchor,
        accent: Color32,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            anchor,
            accent,
        }
    }

    /// Render the pane this frame.
    ///
    /// `body` is invoked with a Ui whose `max_rect` is exactly the
    /// flex-allocated body area (everything the title strip didn't
    /// take). In Phase 1 callers pass `|_| {}` — Phase 3 will start
    /// putting containers here.
    pub fn show(self, ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) {
        let pane_size = if self.anchor.is_vertical_pane() {
            PANE_VERTICAL_SIZE
        } else {
            PANE_HORIZONTAL_SIZE
        };
        let screen = ctx.screen_rect();
        let pos = compute_pane_pos(self.anchor, screen, pane_size);

        let area_id = self.id.with("pane2_area");
        egui::Area::new(area_id)
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                // Outer Frame — pane fill + border + corner radius
                // straight from the active theme. The Frame paints
                // BEHIND the flex container, so the body sees only
                // the inner Ui's rect (post-margin, pre-stroke).
                let theme = style::theme();
                let fill = if theme.pane_fill_visible {
                    style::glass_fill(
                        style::pane_fill(self.accent),
                        self.accent,
                        style::glass_alpha_window(),
                    )
                } else {
                    Color32::TRANSPARENT
                };
                egui::Frame::new()
                    .fill(fill)
                    .stroke(Stroke::new(theme.border_width, style::widget_border(self.accent)))
                    .corner_radius(egui::CornerRadius::same(theme.radius_lg))
                    .inner_margin(egui::Margin::same(2))
                    .show(ui, |ui| {
                        ui.set_min_size(pane_size - vec2(4.0, 4.0));
                        ui.set_max_size(pane_size - vec2(4.0, 4.0));
                        self.lay_out_flex(ui, body);
                    });
            });
    }

    /// Inner flex layout: split the pane Ui into a fixed-size title
    /// strip + a `grow(1.0)` body. Direction depends on the rail —
    /// vertical pane (Left/Right rail) lays out horizontally so the
    /// strip is a tall side panel; horizontal pane (Top/Bottom rail)
    /// lays out vertically so the strip is a wide top/bottom band.
    fn lay_out_flex(self, ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
        let Pane2 { id, title, anchor, accent } = self;

        let flex = if anchor.is_vertical_pane() {
            // Vertical pane (Left/Right rail): horizontal title bar
            // at the TOP, body fills the rest below it → flex
            // direction = column. Title is wide and short, text
            // reads upright (no rotation).
            Flex::vertical()
        } else {
            // Horizontal pane (Top/Bottom rail): vertical title
            // strip on the LEFT, body fills the rest to its right
            // → flex direction = row. Title is tall and thin, text
            // reads rotated.
            Flex::horizontal()
        };

        flex
            .gap(Vec2::ZERO)
            .align_items(FlexAlign::Stretch)
            .w_full()
            .h_full()
            .id_salt(id.with("pane2_flex"))
            .show(ui, |flex| {
                let title_text = title.clone();
                let title_paint = move |ui: &mut egui::Ui| {
                    paint_pane_title(
                        ui,
                        ui.max_rect(),
                        id,
                        &title_text,
                        anchor,
                        accent,
                    );
                };
                let body_paint = move |ui: &mut egui::Ui| body(ui);

                if anchor.title_at_end() {
                    flex.add_ui(item().grow(1.0), body_paint);
                    flex.add_ui(item().basis(TITLE_STRIP_THICKNESS), title_paint);
                } else {
                    flex.add_ui(item().basis(TITLE_STRIP_THICKNESS), title_paint);
                    flex.add_ui(item().grow(1.0), body_paint);
                }
            });
    }
}

// ─── Anchor → screen position ──────────────────────────────────────

fn compute_pane_pos(anchor: PaneAnchor, screen: Rect, pane: Vec2) -> egui::Pos2 {
    // Inner box that excludes the ribbon strip on every screen edge —
    // panes never overlap a ribbon button. Even when only one rail
    // is in use, the symmetric inset keeps zone centring honest
    // (the centre of the available area, not of the raw screen).
    let inset = RAIL_INSET;
    let inner = Rect::from_min_max(
        pos2(screen.min.x + inset, screen.min.y + inset),
        pos2(screen.max.x - inset, screen.max.y - inset),
    );
    match anchor {
        PaneAnchor::LeftRail(zone) => {
            let x = inner.min.x;
            let y = vertical_zone_y(zone, inner, pane.y);
            pos2(x, y)
        }
        PaneAnchor::RightRail(zone) => {
            let x = inner.max.x - pane.x;
            let y = vertical_zone_y(zone, inner, pane.y);
            pos2(x, y)
        }
        PaneAnchor::TopRail(zone) => {
            let y = inner.min.y;
            let x = horizontal_zone_x(zone, inner, pane.x);
            pos2(x, y)
        }
        PaneAnchor::BottomRail(zone) => {
            let y = inner.max.y - pane.y;
            let x = horizontal_zone_x(zone, inner, pane.x);
            pos2(x, y)
        }
    }
}

fn vertical_zone_y(zone: RailZone, inner: Rect, pane_h: f32) -> f32 {
    match zone {
        RailZone::Start => inner.min.y,
        RailZone::Middle => inner.min.y + (inner.height() - pane_h) * 0.5,
        RailZone::End => inner.max.y - pane_h,
    }
}

fn horizontal_zone_x(zone: RailZone, inner: Rect, pane_w: f32) -> f32 {
    match zone {
        RailZone::Start => inner.min.x,
        RailZone::Middle => inner.min.x + (inner.width() - pane_w) * 0.5,
        RailZone::End => inner.max.x - pane_w,
    }
}

// ─── Title painting ─────────────────────────────────────────────────

/// Paint the title strip background + text inside `rect`. Theme
/// aware:
///
/// * `theme.pane_title_stripes` ON (GAME): caution-stripe banner
///   via [`crate::style::paint_caution_stripes`].
/// * OFF (PRO): solid accent fill.
///
/// Text rotation depends on the rail: vertical strips rotate text
/// `±π/2`; horizontal strips paint text upright.
fn paint_pane_title(
    ui: &mut egui::Ui,
    rect: Rect,
    id: Id,
    title: &str,
    anchor: PaneAnchor,
    accent: Color32,
) {
    let theme = style::theme();

    // Background — caution stripes (GAME) or solid accent (PRO).
    if theme.pane_title_stripes {
        style::paint_caution_stripes(ui.painter(), rect, accent);
    } else {
        ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, accent);
    }

    // Foreground text colour: contrast against the accent banner.
    let text_col = style::contrast_text_for(accent);

    // Optional GAME scramble-decode reveal.
    let title_uc = title.to_uppercase();
    let displayed = if theme.scramble_titles {
        let session_id = id.with("pane2_title_session");
        let session = style::appearance_session(ui.ctx(), session_id);
        let scramble_id = session_id.with(session);
        style::scramble_text(ui.ctx(), scramble_id, &title_uc, true)
    } else {
        title_uc
    };

    let font = egui::FontId::new(theme.section_title_size + 1.0, style::title_font_family());
    let format = egui::TextFormat {
        font_id: font,
        color: text_col,
        extra_letter_spacing: theme.section_title_letter_spacing,
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.append(&displayed, 0.0, format);
    let galley = ui.painter().layout_job(job);

    match anchor {
        // Vertical pane (Left/Right rail) → horizontal title bar at
        // the top, upright text centred in the strip.
        PaneAnchor::LeftRail(_) | PaneAnchor::RightRail(_) => {
            let pos = rect.center() - galley.size() * 0.5;
            ui.painter().galley(pos, galley, text_col);
        }

        // Top-rail pane → vertical title strip on the LEFT, text
        // reads TOP-TO-BOTTOM (`+π/2`). Eye flows from the rail
        // (above the pane) down into the title.
        PaneAnchor::TopRail(_) => {
            paint_rotated_centred(ui, rect, galley, text_col, std::f32::consts::FRAC_PI_2);
        }

        // Bottom-rail pane → vertical title strip on the LEFT, text
        // reads BOTTOM-TO-TOP (`-π/2`). Eye flows from the rail
        // (below the pane) up into the title.
        PaneAnchor::BottomRail(_) => {
            paint_rotated_centred(ui, rect, galley, text_col, -std::f32::consts::FRAC_PI_2);
        }
    }

}

/// Paint a rotated single-line galley centred inside `rect`. Used
/// only for the two vertical-strip orientations.
fn paint_rotated_centred(
    ui: &mut egui::Ui,
    rect: Rect,
    galley: std::sync::Arc<egui::Galley>,
    color: Color32,
    angle: f32,
) {
    let g = galley.size();
    let cx = rect.center().x;
    let cy = rect.center().y;

    // Rotation around the TextShape's `pos`:
    //   +π/2: (dx,dy) → (-dy, dx); galley (0..g.x, 0..g.y) rotates
    //         to (-g.y..0, 0..g.x). Centre of rotated bbox is at
    //         (pos.x - g.y/2, pos.y + g.x/2).
    //   -π/2: (dx,dy) → ( dy,-dx); galley rotates to (0..g.y, -g.x..0).
    //         Centre at (pos.x + g.y/2, pos.y - g.x/2).
    let pos = if angle > 0.0 {
        // +π/2: centre at (pos.x - g.y/2, pos.y + g.x/2) = (cx, cy).
        pos2(cx + g.y * 0.5, cy - g.x * 0.5)
    } else {
        // -π/2: centre at (pos.x + g.y/2, pos.y - g.x/2) = (cx, cy).
        pos2(cx - g.y * 0.5, cy + g.x * 0.5)
    };

    let mut shape = egui::epaint::TextShape::new(pos, galley, color);
    shape.angle = angle;
    ui.painter().add(shape);
}
