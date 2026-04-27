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
                let theme = style::theme();
                // Outer pane fill — matches `floating.rs::floating_window_scoped`:
                // PRO uses the glass-tinted panel colour; GAME (`pane_fill_visible
                // = false`) leaves it transparent. The same shadow params come
                // from the theme so PRO panes drop a soft shadow and GAME's
                // can flatten if it wants.
                let fill = if theme.pane_fill_visible {
                    style::glass_fill(
                        style::pane_fill(self.accent),
                        self.accent,
                        style::glass_alpha_window(),
                    )
                } else {
                    Color32::TRANSPARENT
                };
                let shadow = egui::epaint::Shadow {
                    offset: [0, theme.pane_shadow_y],
                    blur: theme.pane_shadow_blur,
                    spread: 0,
                    color: Color32::from_black_alpha(115),
                };
                egui::Frame {
                    inner_margin: egui::Margin::same(2),
                    outer_margin: egui::Margin::ZERO,
                    fill,
                    stroke: Stroke::new(theme.border_width, style::widget_border(self.accent)),
                    corner_radius: egui::CornerRadius::same(theme.radius_lg),
                    shadow,
                }
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

        // Inner Ui size — Frame's `inner_margin = 2` shrinks each
        // axis by 4. Used to size the flex container AND the
        // title strip's `min_size` cross-axis so paint-only items
        // don't collapse to 0 on the cross axis.
        let pane_size = if anchor.is_vertical_pane() {
            PANE_VERTICAL_SIZE
        } else {
            PANE_HORIZONTAL_SIZE
        };
        let inner = pane_size - vec2(4.0, 4.0);

        let (flex, title_min_size) = if anchor.is_vertical_pane() {
            // Vertical pane (Left/Right rail): horizontal title bar
            // at the TOP, body fills the rest below it → flex
            // direction = column. Title is wide and short, text
            // reads upright (no rotation).
            //
            // Cross axis = X. Title must claim the FULL inner width
            // (otherwise paint-only items intrinsically size to 0
            // and `align_items=Stretch` has nothing to stretch
            // against). Main axis = Y → basis 25 height.
            (Flex::vertical(), vec2(inner.x, TITLE_STRIP_THICKNESS))
        } else {
            // Horizontal pane (Top/Bottom rail): vertical title
            // strip on the LEFT, body fills the rest to its right
            // → flex direction = row. Title is tall and thin.
            //
            // Cross axis = Y. Title must claim the FULL inner
            // height. Main axis = X → basis 25 width.
            (Flex::horizontal(), vec2(TITLE_STRIP_THICKNESS, inner.y))
        };

        flex
            .gap(Vec2::ZERO)
            .align_items(FlexAlign::Stretch)
            .size(inner)
            .id_salt(id.with("pane2_flex"))
            .show(ui, |flex| {
                let title_text = title.clone();
                let title_paint = move |ui: &mut egui::Ui| {
                    // Allocate the full slot so the inner Ui
                    // reports a real `min_rect` to flex (not 0×0).
                    // Without this, the flex's intrinsic-size pass
                    // collapses paint-only items on the cross axis.
                    let avail = ui.available_size_before_wrap();
                    let (alloc_rect, _) =
                        ui.allocate_exact_size(avail, egui::Sense::hover());
                    paint_pane_title(
                        ui,
                        alloc_rect,
                        id,
                        &title_text,
                        anchor,
                        accent,
                    );
                };
                let body_paint = move |ui: &mut egui::Ui| {
                    let avail = ui.available_size_before_wrap();
                    let (_alloc_rect, _) =
                        ui.allocate_exact_size(avail, egui::Sense::hover());
                    body(ui);
                };

                if anchor.title_at_end() {
                    flex.add_ui(item().grow(1.0).min_size(inner), body_paint);
                    flex.add_ui(
                        item().basis(TITLE_STRIP_THICKNESS).min_size(title_min_size),
                        title_paint,
                    );
                } else {
                    flex.add_ui(
                        item().basis(TITLE_STRIP_THICKNESS).min_size(title_min_size),
                        title_paint,
                    );
                    flex.add_ui(item().grow(1.0).min_size(inner), body_paint);
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

/// Paint the title strip background + text inside `rect`,
/// matching the visual recipe from `floating.rs::paint_title` so
/// PRO and GAME read identically across the old and new pane
/// stacks. Six pieces:
///
/// 1. Background: theme-driven panel fill (PRO) or animated
///    caution stripes (GAME).
/// 2. Title text: scramble-decoded when `scramble_titles` is on,
///    aligned to the anchor side (left for LeftRail/TopRail,
///    right/bottom for RightRail/BottomRail), rotated for
///    horizontal panes.
/// 3. Blinking pip at the OPPOSITE end of the strip (GAME only).
/// 4. Divider hairline on the body-facing edge of the strip
///    (`pane_show_title_divider`).
///
/// Constants `TITLE_INSET = 8`, `title_size = 15 × 1.15`,
/// `PIP_SIZE = 6` are the same numbers `floating.rs` uses.
fn paint_pane_title(
    ui: &mut egui::Ui,
    rect: Rect,
    id: Id,
    title: &str,
    anchor: PaneAnchor,
    accent: Color32,
) {
    const TITLE_INSET: f32 = 8.0;
    const PIP_SIZE: f32 = 6.0;
    let title_size = 15.0 * 1.15;
    let theme = style::theme();
    let stripes_on = theme.pane_title_stripes;

    // ── 1. Background ──
    // PRO themes with the outer pane frame invisible (`!pane_fill_visible`)
    // and no stripes get an explicit fill on the title strip so it
    // still reads as a panel header. GAME (`stripes_on`) gets the
    // animated caution-stripe banner painted ONLY over the title
    // strip — body never gets the stripes.
    if !theme.pane_fill_visible && !stripes_on {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(theme.radius_lg),
            style::pane_fill(accent),
        );
    }
    if stripes_on {
        style::paint_caution_stripes(ui.painter(), rect, accent);
    }

    // ── 2. Title text colour + content ──
    let text_col = if stripes_on {
        if theme.is_light { Color32::BLACK } else { Color32::WHITE }
    } else {
        style::section_title_color(accent)
    };
    let font = egui::FontId::new(title_size, style::title_font_family());
    let title_uc = title.to_uppercase();
    let displayed = if theme.scramble_titles {
        let session_id = id.with("pane2_title_session");
        let session = style::appearance_session(ui.ctx(), session_id);
        let scramble_id = session_id.with(session);
        style::scramble_text(ui.ctx(), scramble_id, &title_uc, true)
    } else {
        title_uc
    };

    // Convenience flags — the 4 anchors collapse into 2 axes.
    let on_right_anchor = matches!(anchor, PaneAnchor::RightRail(_));
    let is_horizontal_title = matches!(
        anchor,
        PaneAnchor::LeftRail(_) | PaneAnchor::RightRail(_)
    );

    // ── 3. Title text paint ──
    // Text starts at the BEGINNING of the strip in the reading
    // direction; the blinking pip (GAME only) lives at the END.
    //   • Horizontal title bar → first letter at LEFT edge
    //     (reading starts on the left). Pip at right edge.
    //   • Vertical title strip → first letter at the strip's
    //     "start" edge (TOP for top-to-bottom, BOTTOM for
    //     bottom-to-top). Pip at the opposite end.
    let _ = on_right_anchor;
    if is_horizontal_title {
        let pos = egui::pos2(
            (rect.min.x + TITLE_INSET).round(),
            rect.center().y.round(),
        );
        ui.painter()
            .text(pos, egui::Align2::LEFT_CENTER, displayed, font, text_col);
    } else {
        // Vertical title strip on the LEFT. Pin the FIRST letter at
        // the strip's start edge (top for `+π/2`, bottom for
        // `-π/2`) and centre the text on the strip's narrow axis.
        // After rotation, the unrotated galley (g.x × g.y) becomes:
        //   +π/2: from (pos.x - g.y, pos.y) to (pos.x, pos.y + g.x).
        //         Pin pos.y = min.y + TITLE_INSET (first letter near
        //         top); centre across narrow axis → pos.x = cx + g.y/2.
        //   -π/2: from (pos.x, pos.y - g.x) to (pos.x + g.y, pos.y).
        //         Pin pos.y = max.y - TITLE_INSET (first letter near
        //         bottom); centre across narrow axis → pos.x = cx - g.y/2.
        let galley = ui.painter().layout_no_wrap(displayed, font, text_col);
        let g = galley.size();
        let cx = rect.center().x;
        let top_to_bottom = matches!(anchor, PaneAnchor::TopRail(_));
        let (pos, angle) = if top_to_bottom {
            (
                egui::pos2((cx + g.y * 0.5).round(), (rect.min.y + TITLE_INSET).round()),
                std::f32::consts::FRAC_PI_2,
            )
        } else {
            (
                egui::pos2((cx - g.y * 0.5).round(), (rect.max.y - TITLE_INSET).round()),
                -std::f32::consts::FRAC_PI_2,
            )
        };
        let mut shape = egui::epaint::TextShape::new(pos, galley, text_col);
        shape.angle = angle;
        ui.painter().add(shape);
    }

    // ── 4. Blinking pip at the OPPOSITE strip end (GAME only) ──
    if stripes_on {
        const PIP_INSET: f32 = TITLE_INSET;
        let time = ui.ctx().input(|i| i.time) as f32;
        let on = time.fract() < 0.08;
        let alpha = if on { 255 } else { 76 };
        let pip_color = Color32::from_rgba_unmultiplied(
            text_col.r(), text_col.g(), text_col.b(), alpha,
        );
        let pip_rect = if is_horizontal_title {
            // Horizontal strip → pip at the END (right edge), text
            // is at the BEGINNING (left edge).
            let pip_x = rect.max.x - PIP_INSET - PIP_SIZE;
            Rect::from_min_size(
                pos2(pip_x.round(), (rect.center().y - PIP_SIZE * 0.5).round()),
                egui::vec2(PIP_SIZE, PIP_SIZE),
            )
        } else {
            // Vertical strip → pip at the OPPOSITE vertical end
            // from the text. TopRail (top-to-bottom text) → pip at
            // bottom; BottomRail (bottom-to-top text) → pip at top.
            let top_to_bottom = matches!(anchor, PaneAnchor::TopRail(_));
            let pip_y = if top_to_bottom {
                rect.max.y - PIP_INSET - PIP_SIZE
            } else {
                rect.min.y + PIP_INSET
            };
            Rect::from_min_size(
                pos2((rect.center().x - PIP_SIZE * 0.5).round(), pip_y.round()),
                egui::vec2(PIP_SIZE, PIP_SIZE),
            )
        };
        ui.painter().rect_filled(pip_rect, egui::CornerRadius::ZERO, pip_color);
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }

    // ── 5. Divider hairline on the body-facing edge ──
    if theme.pane_show_title_divider {
        let stroke = egui::Stroke::new(theme.border_width, style::widget_border(accent));
        if is_horizontal_title {
            // Horizontal title bar at top of pane → divider sits at
            // the BOTTOM of the strip.
            ui.painter().hline(rect.min.x..=rect.max.x, rect.max.y - 0.5, stroke);
        } else {
            // Vertical title strip on left of pane → divider on the
            // RIGHT edge of the strip.
            ui.painter().vline(rect.max.x - 0.5, rect.min.y..=rect.max.y, stroke);
        }
    }
}

