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
    /// `true` if this rail's pane is taller than it is wide
    /// (Left/Right rail panes). Drives the flex direction.
    pub fn is_vertical_pane(self) -> bool {
        matches!(self, PaneAnchor::LeftRail(_) | PaneAnchor::RightRail(_))
    }

    /// `true` if the title strip sits on the END side of the pane
    /// — i.e. on the rail-anchor side when the rail is on the right
    /// or bottom. RightRail panes carry their title on the RIGHT
    /// (END of horizontal flex); BottomRail panes on the BOTTOM
    /// (END of vertical flex). LeftRail / TopRail keep the title at
    /// the START.
    fn title_at_end(self) -> bool {
        matches!(
            self,
            PaneAnchor::RightRail(_) | PaneAnchor::BottomRail(_)
        )
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

/// Inset from each screen edge: `EDGE_GAP + SIDE_BTN_SIZE +
/// RAIL_PANEL_GAP`. Matches the original
/// `floating::floating_window_scoped` exactly so panes from new
/// and old code stacks land at identical positions.
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
        // `content_rect` excludes any reserved areas (statusbar /
        // menubar / docked panels). Matches what the original
        // `floating::floating_window_scoped` reads for its anchor
        // math, so panes land exactly where ribbon buttons expect.
        let screen = ctx.content_rect();
        let pos = compute_pane_pos(self.anchor, screen, pane_size);

        let area_id = self.id.with("pane2_area");
        egui::Area::new(area_id)
            // `Order::Middle` (not Foreground) — same layer egui's
            // ribbon buttons live on. Foreground would render the
            // pane above the buttons even when their rects don't
            // overlap, producing the visual stacking that read as
            // "pane goes above the rails". Original `floating.rs`
            // uses Order::Middle for the same reason.
            .order(egui::Order::Middle)
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
            // Vertical pane (Left/Right rail): vertical title strip
            // on the rail-anchor side (LEFT for LeftRail, RIGHT for
            // RightRail). Flex direction = row → title is one
            // narrow column, body fills the rest beside it.
            //
            // Cross axis = Y. Title min_size must claim the full
            // inner height so paint-only items don't collapse to 0.
            // Main axis = X → basis 25 width.
            (Flex::horizontal(), vec2(TITLE_STRIP_THICKNESS, inner.y))
        } else {
            // Horizontal pane (Top/Bottom rail): horizontal title
            // bar on the rail-anchor side (TOP for TopRail, BOTTOM
            // for BottomRail). Flex direction = column → title is
            // one short row, body fills the rest below/above it.
            //
            // Cross axis = X. Title claims the full inner width.
            // Main axis = Y → basis 25 height.
            (Flex::vertical(), vec2(inner.x, TITLE_STRIP_THICKNESS))
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
    // Two insets:
    //   * `inset_near = RAIL_INSET (46)` — the original 4 px gap
    //     to the rail button strip. Used on TOP, LEFT, and the
    //     perpendicular-far edges of every pane.
    //   * `inset_far = RAIL_INSET + SIDE_BTN_SIZE/2` — extra
    //     breathing room on the PANE'S OWN ANCHOR side when that
    //     side is RIGHT (RightRail panes) or BOTTOM (BottomRail
    //     panes). Vertical-rail End-zone panes (LE / RE) still
    //     reach the bottom with the standard 4 px gap; only the
    //     bottom-anchored panes get the extra inset.
    let inset_near = RAIL_INSET;
    // `SIDE_BTN_SIZE/2` (half the button) + `2 * SIDE_BTN_GAP`
    // (twice the inter-button spacing) — user-tuned for the
    // bottom/right anchor breathing room.
    let inset_far =
        RAIL_INSET + crate::ribbon::SIDE_BTN_SIZE * 0.5 + 2.0 * crate::ribbon::SIDE_BTN_GAP;

    // Range along the perpendicular axis of each rail. Vertical
    // rails (Left/Right) lay panes out along Y from
    // `near` (top edge) to `screen.max.y - near` (bottom edge);
    // horizontal rails along X with the same recipe.
    let v_top = screen.min.y + inset_near;
    let v_bottom = screen.max.y - inset_near;
    let h_left = screen.min.x + inset_near;
    let h_right = screen.max.x - inset_near;

    match anchor {
        PaneAnchor::LeftRail(zone) => {
            let x = screen.min.x + inset_near;       // anchor: LEFT (near)
            let y = zone_along(zone, v_top, v_bottom, pane.y);
            pos2(x, y)
        }
        PaneAnchor::RightRail(zone) => {
            let x = screen.max.x - inset_far - pane.x; // anchor: RIGHT (far)
            let y = zone_along(zone, v_top, v_bottom, pane.y);
            pos2(x, y)
        }
        PaneAnchor::TopRail(zone) => {
            let y = screen.min.y + inset_near;       // anchor: TOP (near)
            let x = zone_along(zone, h_left, h_right, pane.x);
            pos2(x, y)
        }
        PaneAnchor::BottomRail(zone) => {
            let y = screen.max.y - inset_far - pane.y; // anchor: BOTTOM (far)
            let x = zone_along(zone, h_left, h_right, pane.x);
            pos2(x, y)
        }
    }
}

/// Place a pane along its rail's perpendicular axis given the
/// available range `[range_min, range_max]` and pane extent.
fn zone_along(zone: RailZone, range_min: f32, range_max: f32, pane_extent: f32) -> f32 {
    match zone {
        RailZone::Start => range_min,
        RailZone::Middle => range_min + (range_max - range_min - pane_extent) * 0.5,
        RailZone::End => range_max - pane_extent,
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

    // Title strip is HORIZONTAL on Top/Bottom rail panes (text
    // upright, reads left-to-right) and VERTICAL on Left/Right
    // rail panes (text rotated, reads along the strip).
    let is_horizontal_strip = !anchor.is_vertical_pane();

    // ── 3. Title text paint ──
    if is_horizontal_strip {
        // Horizontal title bar (Top/Bottom rail panes). Text at
        // LEFT edge of strip, vertically centred. Pip at RIGHT.
        let pos = egui::pos2(
            (rect.min.x + TITLE_INSET).round(),
            rect.center().y.round(),
        );
        ui.painter()
            .text(pos, egui::Align2::LEFT_CENTER, displayed, font, text_col);
    } else {
        // Vertical title strip (Left/Right rail panes). Reading
        // direction follows the rail:
        //   • LeftRail (strip on LEFT)  → text reads BOTTOM-TO-TOP
        //                                 (`-π/2`); first letter
        //                                 at the strip's BOTTOM.
        //   • RightRail (strip on RIGHT) → text reads TOP-TO-BOTTOM
        //                                  (`+π/2`); first letter
        //                                  at the strip's TOP.
        // Across the narrow axis, the rotated galley is centred on
        // the strip's centre X.
        let galley = ui.painter().layout_no_wrap(displayed, font, text_col);
        let g = galley.size();
        let cx = rect.center().x;
        let on_right = matches!(anchor, PaneAnchor::RightRail(_));
        let (pos, angle) = if on_right {
            // RightRail → +π/2: text bbox extends LEFT of pos by
            // g.y; first letter at top → pos.y = min.y + INSET;
            // centre across narrow axis → pos.x = cx + g.y/2.
            (
                egui::pos2((cx + g.y * 0.5).round(), (rect.min.y + TITLE_INSET).round()),
                std::f32::consts::FRAC_PI_2,
            )
        } else {
            // LeftRail → -π/2: text bbox extends RIGHT of pos by
            // g.y; first letter at bottom → pos.y = max.y - INSET;
            // centre across narrow axis → pos.x = cx - g.y/2.
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
        let pip_rect = if is_horizontal_strip {
            // Horizontal strip (Top/Bottom rail) → pip at the END
            // (right edge), text is at the BEGINNING (left edge).
            let pip_x = rect.max.x - PIP_INSET - PIP_SIZE;
            Rect::from_min_size(
                pos2(pip_x.round(), (rect.center().y - PIP_SIZE * 0.5).round()),
                egui::vec2(PIP_SIZE, PIP_SIZE),
            )
        } else {
            // Vertical strip (Left/Right rail) → pip at the OPPOSITE
            // end of the text reading direction. RightRail (text
            // top-to-bottom) → pip at bottom; LeftRail (text
            // bottom-to-top) → pip at top.
            let on_right = matches!(anchor, PaneAnchor::RightRail(_));
            let pip_y = if on_right {
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
        match anchor {
            PaneAnchor::TopRail(_) => {
                // Title at TOP → divider on the strip's BOTTOM edge.
                ui.painter().hline(rect.min.x..=rect.max.x, rect.max.y - 0.5, stroke);
            }
            PaneAnchor::BottomRail(_) => {
                // Title at BOTTOM → divider on the strip's TOP edge.
                ui.painter().hline(rect.min.x..=rect.max.x, rect.min.y + 0.5, stroke);
            }
            PaneAnchor::LeftRail(_) => {
                // Title on LEFT → divider on the strip's RIGHT edge.
                ui.painter().vline(rect.max.x - 0.5, rect.min.y..=rect.max.y, stroke);
            }
            PaneAnchor::RightRail(_) => {
                // Title on RIGHT → divider on the strip's LEFT edge.
                ui.painter().vline(rect.min.x + 0.5, rect.min.y..=rect.max.y, stroke);
            }
        }
    }
}

