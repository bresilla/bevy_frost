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

    /// Which side of the pane the title strip sits on.
    /// Middle-zone panes use the rail-anchor side (the original
    /// convention). Corner-zone (Start/End) panes flip per the
    /// user's pair spec (LS=h-top, TS=v-left, RE=h-bottom, BE=v-right,
    /// LE=h-bottom, BS=v-left). TE and RS are already on their
    /// rail-anchor side, which the spec preserves.
    fn title_side(self) -> TitleSide {
        match self {
            PaneAnchor::LeftRail(RailZone::Start)   => TitleSide::Top,
            PaneAnchor::LeftRail(RailZone::End)     => TitleSide::Bottom,
            PaneAnchor::RightRail(RailZone::End)    => TitleSide::Bottom,
            PaneAnchor::TopRail(RailZone::Start)    => TitleSide::Left,
            PaneAnchor::BottomRail(RailZone::Start) => TitleSide::Left,
            PaneAnchor::BottomRail(RailZone::End)   => TitleSide::Right,
            PaneAnchor::LeftRail(_)                 => TitleSide::Left,
            PaneAnchor::RightRail(_)                => TitleSide::Right,
            PaneAnchor::TopRail(_)                  => TitleSide::Top,
            PaneAnchor::BottomRail(_)               => TitleSide::Bottom,
        }
    }
}

/// Which side of the pane rect carries the title strip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TitleSide { Top, Bottom, Left, Right }

impl TitleSide {
    fn is_horizontal_strip(self) -> bool {
        matches!(self, TitleSide::Top | TitleSide::Bottom)
    }
    fn is_at_end(self) -> bool {
        matches!(self, TitleSide::Bottom | TitleSide::Right)
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
    /// strip + a `grow(1.0)` body. Direction comes from `title_side()`
    /// (per-anchor) — horizontal strips (Top/Bottom side) need a
    /// vertical flex, vertical strips (Left/Right side) need a
    /// horizontal flex.
    fn lay_out_flex(self, ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
        let Pane2 { id, title, anchor, accent } = self;
        let title_side = anchor.title_side();

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

        let (flex, title_min_size) = if title_side.is_horizontal_strip() {
            // Title bar runs HORIZONTALLY across the pane (Top/
            // Bottom side). Flex direction = column. Title claims
            // the full inner width × `TITLE_STRIP_THICKNESS` height.
            (Flex::vertical(), vec2(inner.x, TITLE_STRIP_THICKNESS))
        } else {
            // Title strip runs VERTICALLY down the pane (Left/Right
            // side). Flex direction = row. Title claims `TITLE_STRIP_THICKNESS`
            // width × the full inner height.
            (Flex::horizontal(), vec2(TITLE_STRIP_THICKNESS, inner.y))
        };
        let title_at_end = title_side.is_at_end();

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

                if title_at_end {
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
    //   • `near = RAIL_INSET (46)` — the original 4 px gap to the
    //     rail's button strip. Used by every pane on the TOP and
    //     LEFT edges. Also used by **corner-zone End/Start panes**
    //     when they reach the BOTTOM or RIGHT — user explicitly
    //     listed TE, RE, BE, BS, LE as panes that shouldn't be
    //     pushed.
    //   • `far = RAIL_INSET + SIDE_BTN_SIZE/2 + 2*SIDE_BTN_GAP` —
    //     extra breathing room. Used by **Middle-zone panes** that
    //     reach the BOTTOM or RIGHT (LM, RM, BM) and by Start-zone
    //     RIGHT-edge panes (RS).
    let near = RAIL_INSET;
    let far = RAIL_INSET + crate::ribbon::SIDE_BTN_SIZE * 0.5 + 2.0 * crate::ribbon::SIDE_BTN_GAP;

    // Helpers — `start_*` always uses near (top/left of screen).
    let left = screen.min.x + near;
    let top = screen.min.y + near;

    // Right and bottom can be near (no push) OR far (with push)
    // depending on the anchor.
    let right_near = screen.max.x - near;
    let right_far  = screen.max.x - far;
    let bottom_near = screen.max.y - near;
    let bottom_far  = screen.max.y - far;

    match anchor {
        // ── LEFT rail: x always near (left edge). y per zone. ──
        PaneAnchor::LeftRail(RailZone::Start) =>
            pos2(left, top),
        PaneAnchor::LeftRail(RailZone::Middle) =>
            // Middle panes get pushed → centre between top-near and
            // bottom-far so LM sits a touch above true centre,
            // matching how RM / BM behave.
            pos2(left, ((top + bottom_far) - pane.y) * 0.5),
        PaneAnchor::LeftRail(RailZone::End) =>
            // LE pushed UP.
            pos2(left, bottom_far - pane.y),

        // ── RIGHT rail: y per zone. x near for End, far otherwise. ──
        PaneAnchor::RightRail(RailZone::Start) =>
            pos2(right_far - pane.x, top),
        PaneAnchor::RightRail(RailZone::Middle) =>
            pos2(right_far - pane.x, ((top + bottom_far) - pane.y) * 0.5),
        PaneAnchor::RightRail(RailZone::End) =>
            // RE pushed UP only (x stays at near).
            pos2(right_near - pane.x, bottom_far - pane.y),

        // ── TOP rail: y always near (top edge). x per zone. ──
        PaneAnchor::TopRail(RailZone::Start) =>
            pos2(left, top),
        PaneAnchor::TopRail(RailZone::Middle) =>
            pos2(((left + right_far) - pane.x) * 0.5, top),
        PaneAnchor::TopRail(RailZone::End) =>
            // TE NOT pushed.
            pos2(right_near - pane.x, top),

        // ── BOTTOM rail: x per zone. y near for Start/End, far for Middle. ──
        PaneAnchor::BottomRail(RailZone::Start) =>
            // BS NOT pushed.
            pos2(left, bottom_near - pane.y),
        PaneAnchor::BottomRail(RailZone::Middle) =>
            pos2(((left + right_far) - pane.x) * 0.5, bottom_far - pane.y),
        PaneAnchor::BottomRail(RailZone::End) =>
            // BE pushed LEFT only (y stays at near).
            pos2(right_far - pane.x, bottom_near - pane.y),
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

    let title_side = anchor.title_side();
    let is_horizontal_strip = title_side.is_horizontal_strip();

    // ── 3. Title text paint ──
    if is_horizontal_strip {
        // Horizontal title bar (TitleSide::Top or Bottom). Text at
        // LEFT edge of strip, vertically centred. Pip at RIGHT.
        let pos = egui::pos2(
            (rect.min.x + TITLE_INSET).round(),
            rect.center().y.round(),
        );
        ui.painter()
            .text(pos, egui::Align2::LEFT_CENTER, displayed, font, text_col);
    } else {
        // Vertical title strip (TitleSide::Left or Right). Reading
        // direction follows which side the strip is on:
        //   • Left  → text reads BOTTOM-TO-TOP (`-π/2`); first
        //             letter at the strip's BOTTOM.
        //   • Right → text reads TOP-TO-BOTTOM (`+π/2`); first
        //             letter at the strip's TOP.
        let galley = ui.painter().layout_no_wrap(displayed, font, text_col);
        let g = galley.size();
        let cx = rect.center().x;
        let on_right = title_side == TitleSide::Right;
        let (pos, angle) = if on_right {
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
        let pip_rect = if is_horizontal_strip {
            // Horizontal strip → pip at the right edge (text on left).
            let pip_x = rect.max.x - PIP_INSET - PIP_SIZE;
            Rect::from_min_size(
                pos2(pip_x.round(), (rect.center().y - PIP_SIZE * 0.5).round()),
                egui::vec2(PIP_SIZE, PIP_SIZE),
            )
        } else {
            // Vertical strip → pip opposite the reading-start.
            // Right side (top-to-bottom) → pip at bottom.
            // Left side (bottom-to-top) → pip at top.
            let on_right = title_side == TitleSide::Right;
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
        match title_side {
            TitleSide::Top => {
                // Title at TOP → divider on the strip's BOTTOM edge.
                ui.painter().hline(rect.min.x..=rect.max.x, rect.max.y - 0.5, stroke);
            }
            TitleSide::Bottom => {
                // Title at BOTTOM → divider on the strip's TOP edge.
                ui.painter().hline(rect.min.x..=rect.max.x, rect.min.y + 0.5, stroke);
            }
            TitleSide::Left => {
                // Title on LEFT → divider on the strip's RIGHT edge.
                ui.painter().vline(rect.max.x - 0.5, rect.min.y..=rect.max.y, stroke);
            }
            TitleSide::Right => {
                // Title on RIGHT → divider on the strip's LEFT edge.
                ui.painter().vline(rect.min.x + 0.5, rect.min.y..=rect.max.y, stroke);
            }
        }
    }
}

