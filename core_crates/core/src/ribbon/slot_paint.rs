use egui::{Color32, Context, Id, Rect, Response, Sense, Vec2, pos2, vec2};

use super::{
    RibbonAction, RibbonCluster, RibbonEdge, RibbonScope, RibbonSlotItem,
    paint::{paint_ribbon_button, paint_ribbon_glyph, ribbon_button_fg},
};

#[derive(Clone, Debug)]
pub struct ResolvedSlotRibbon {
    pub id: Id,
    pub scope: RibbonScope,
    pub edge: RibbonEdge,
    pub cluster: RibbonCluster,
    pub items: Vec<RibbonSlotItem>,
}

#[derive(Clone, Debug)]
pub struct RibbonSlotClick {
    pub ribbon: Id,
    pub item: Id,
    pub action: RibbonAction,
    pub response: Response,
}

/// Paint already-resolved slot ribbons as simple icon/button rails.
///
/// This is the bridge renderer for the new permanent/view/workspace
/// slot model. It intentionally omits legacy drag and panel-open
/// behavior: those remain in `draw_assembly` while hosts migrate
/// root chrome to resolved slot ribbons.
pub fn draw_slot_ribbons(
    ctx: &Context,
    accent: Color32,
    ribbons: &[ResolvedSlotRibbon],
) -> Vec<RibbonSlotClick> {
    let mut clicks = Vec::new();
    for ribbon in ribbons {
        draw_one_slot_ribbon(ctx, accent, ribbon, &mut clicks);
    }
    clicks
}

fn draw_one_slot_ribbon(
    ctx: &Context,
    accent: Color32,
    ribbon: &ResolvedSlotRibbon,
    clicks: &mut Vec<RibbonSlotClick>,
) {
    if ribbon.items.is_empty() {
        return;
    }

    let screen = ctx.content_rect();
    let chrome = chrome_for_scope(ribbon.scope);
    let count = ribbon.items.len() as f32;
    let vertical = ribbon.edge.is_vertical();
    let span = count * chrome.button_size + (count - 1.0).max(0.0) * chrome.button_gap;
    let size = if vertical {
        vec2(chrome.button_size, span)
    } else {
        vec2(span, chrome.button_size)
    };
    let pos = ribbon_origin(screen, ribbon.edge, ribbon.cluster, size, chrome.edge_gap);
    let area_id = Id::new(("frost_slot_ribbon", ribbon.id));

    egui::Area::new(area_id)
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            ui.set_min_size(size);
            for (idx, item) in ribbon.items.iter().enumerate() {
                let offset = idx as f32 * (chrome.button_size + chrome.button_gap);
                let min = if vertical {
                    pos2(0.0, offset)
                } else {
                    pos2(offset, 0.0)
                };
                let rect = Rect::from_min_size(min, Vec2::splat(chrome.button_size));
                let response = ui
                    .interact(rect, ui.id().with(item.id), Sense::click())
                    .on_hover_text(item.tooltip.clone());
                paint_ribbon_button(ui.painter(), rect, accent, item.active, response.hovered());
                let glyph = glyph_for_item(item);
                paint_ribbon_glyph(
                    ui,
                    rect,
                    glyph,
                    ribbon_button_fg(accent, item.active, response.hovered(), glyph),
                );
                if response.clicked() {
                    clicks.push(RibbonSlotClick {
                        ribbon: ribbon.id,
                        item: item.id,
                        action: item.action,
                        response,
                    });
                }
            }
        });
}

fn chrome_for_scope(scope: RibbonScope) -> crate::style::RibbonChromeTheme {
    let ribbon = crate::style::theme().ribbon;
    match scope {
        RibbonScope::Permanent => ribbon.permanent,
        RibbonScope::View(_) => ribbon.view_local,
        RibbonScope::WorkspaceLevel(_) => ribbon.workspace,
    }
}

fn ribbon_origin(
    screen: Rect,
    edge: RibbonEdge,
    cluster: RibbonCluster,
    size: Vec2,
    margin: f32,
) -> egui::Pos2 {
    match edge {
        RibbonEdge::Left => {
            let y = cluster_axis_pos(screen.top(), screen.bottom(), size.y, cluster, margin);
            pos2(screen.left() + margin, y)
        }
        RibbonEdge::Right => {
            let y = cluster_axis_pos(screen.top(), screen.bottom(), size.y, cluster, margin);
            pos2(screen.right() - margin - size.x, y)
        }
        RibbonEdge::Top => {
            let x = cluster_axis_pos(screen.left(), screen.right(), size.x, cluster, margin);
            pos2(x, screen.top() + margin)
        }
        RibbonEdge::Bottom => {
            let x = cluster_axis_pos(screen.left(), screen.right(), size.x, cluster, margin);
            pos2(x, screen.bottom() - margin - size.y)
        }
    }
}

fn cluster_axis_pos(min: f32, max: f32, span: f32, cluster: RibbonCluster, margin: f32) -> f32 {
    match cluster {
        RibbonCluster::Start => min + margin,
        RibbonCluster::Middle => (min + max - span) * 0.5,
        RibbonCluster::End => max - margin - span,
    }
}

fn glyph_for_item(item: &RibbonSlotItem) -> super::RibbonGlyph {
    super::RibbonGlyph::Icon(item.icon)
}
