//! GAME theme — square corners, accent-tinted panels, bracket
//! titles on a solid accent banner, dashed row separators, L-bracket
//! corner ticks.
//!
//! See [`crate::themes`](super) for the "how to add a theme" guide.

use crate::style::{
    ColorMode, Mode, TextColorMode, Theme,
    // shared text constants
    TEXT_DISABLED, TEXT_DISABLED_LIGHT, TEXT_PRIMARY, TEXT_PRIMARY_LIGHT,
    TEXT_SECONDARY, TEXT_SECONDARY_LIGHT,
};

/// GAME Light surface palette — bright accent-tinted surfaces, dark
/// text. Text colours flow through the shared `TEXT_*_LIGHT`
/// constants, not per-theme overrides.
pub const GAME_LIGHT_BG_WINDOW: egui::Color32 = egui::Color32::from_rgb(0xF0, 0xF1, 0xF5);
pub const GAME_LIGHT_BG_PANEL:  egui::Color32 = egui::Color32::from_rgb(0xFA, 0xFB, 0xFD);
// Raised + input tightened (same reasoning as PRO Light). Raised
// flipped from `FFFFFF` (which was actually *brighter* than the
// panel — wrong direction for a Light theme) to a tone visibly
// darker than the panel. Input also pulled away from the panel.
pub const GAME_LIGHT_BG_RAISED: egui::Color32 = egui::Color32::from_rgb(0xF1, 0xF3, 0xF7);
pub const GAME_LIGHT_BG_HOVER:  egui::Color32 = egui::Color32::from_rgb(0xE6, 0xE8, 0xEE);
pub const GAME_LIGHT_BG_INPUT:  egui::Color32 = egui::Color32::from_rgb(0xEE, 0xF0, 0xF5);

/// Built-in GAME profile — square corners, accent-tinted panels,
/// bracket-decorated titles on a solid accent banner, dashed row
/// separators, L-bracket corner ticks. Pick a [`Mode`] to flip the
/// whole brightness axis: Dark lerps surfaces toward black for the
/// deep tactical look, Light lerps toward white for a paper /
/// accent-stained variant.
pub const fn theme_game(mode: Mode) -> Theme {
    let dark = matches!(mode, Mode::Dark);
    let lerp_target = if dark {
        egui::Color32::BLACK
    } else {
        egui::Color32::WHITE
    };
    let lerp_factor = if dark { 0.22 } else { 0.18 };
    Theme {
        name: if dark { "GAME_DARK" } else { "GAME_LIGHT" },
        is_light: !dark,
        bg_window:  if dark { egui::Color32::from_rgb(0x08, 0x0A, 0x12) } else { GAME_LIGHT_BG_WINDOW },
        bg_panel:   if dark { egui::Color32::from_rgb(0x10, 0x14, 0x1F) } else { GAME_LIGHT_BG_PANEL },
        bg_raised:  if dark { egui::Color32::from_rgb(0x16, 0x1B, 0x29) } else { GAME_LIGHT_BG_RAISED },
        bg_hover:   if dark { egui::Color32::from_rgb(0x1F, 0x26, 0x38) } else { GAME_LIGHT_BG_HOVER },
        bg_input:   if dark { egui::Color32::from_rgb(0x06, 0x08, 0x0E) } else { GAME_LIGHT_BG_INPUT },
        panel_fill_mode:   ColorMode::FromAccent { lerp_factor, lerp_target },
        section_fill_mode: ColorMode::FromAccent { lerp_factor, lerp_target },
        section_show_frame: true,
        section_show_title_divider: false,
        section_pad_x: 3,
        section_pad_y: 2,
        section_body_indent: 8.0,
        section_outer_margin_flow_title: 6,
        section_outer_margin_flow_body: 0,
        section_outer_margin_span: 1,
        section_body_inner_top_pad: 12.0,
        pane_title_chromatic_aberration: true,
        section_animation_time: 0.35,
        animations_enabled: true,
        button_anim_scale: 2.0,
        pane_fade_scale: 1.0,
        text_primary:   if dark { TEXT_PRIMARY }   else { TEXT_PRIMARY_LIGHT },
        text_secondary: if dark { TEXT_SECONDARY } else { TEXT_SECONDARY_LIGHT },
        text_disabled:  if dark { TEXT_DISABLED }  else { TEXT_DISABLED_LIGHT },
        title_color_mode: TextColorMode::Accent,
        title_softness: 0.0,
        ribbon_button_accent_fill: true,
        section_gap: 4.0,
        section_corner_ticks_inset: 3.0,
        section_title_brackets: true,
        section_title_prefix: None,
        section_title_letter_spacing: 1.5,
        subcaption_prefix: Some("// "),
        section_bottom_rule: true,
        pane_fill_visible: false,
        show_section_chevron: false,
        title_strip_filled: true,
        section_title_size: 11.5,
        body_accent_darken: 0.18,
        section_icon_at_end: true,
        section_icon_size: 20.0,
        section_body_top_pad: 16.0,
        row_separator_dash: Some((4.0, 3.0)),
        section_title_trailing_rule: false,
        section_corner_ticks: 10.0,
        border_subtle:      if dark {
            egui::Color32::from_rgb(0x80, 0x80, 0x80)
        } else {
            egui::Color32::from_rgb(0x6B, 0x70, 0x78)
        },
        border_inner:       egui::Color32::from_rgb(0x1F, 0x26, 0x38),
        border_alpha:       if dark { 35 } else { 0 },
        border_accent_tint: 0.0,
        border_width:       if dark { 0.63 } else { 0.0 },
        row_separator_alpha: if dark { 25 } else { 28 },
        glass_card_factor:  1.0,
        glass_group_factor: 1.0,
        glass_accent_tint:  0.0,
        radius_widget:  0,
        radius_compact: 0,
        radius_sm:      0,
        radius_md:      0,
        radius_lg:      0,
        row_alternation: false,
        row_alt_lift: 0.0,
        button_full_accent_on_press: true,
        button_tint_rest:  0.12,
        button_tint_hover: 0.18,
        button_tint_press: 0.40,
        pane_shadow_blur:  0,
        pane_shadow_y:     0,
        pane_show_title_divider: false,
        pane_title_stripes: true,
        scramble_titles: true,
        tree_guide_width: 0.0,
        graph_pin_width:  0.0,
        graph_wire_glow:  1.0,
        graph_pin_glow:   0.85,
        graph_canvas_hex: true,
        progressbar_segmented: true,
        pane_title_brackets: true,
        section_separator_strip_h: 14.0,
        section_separator_alpha: 64,
        section_body_inner_end_pad: 12.0,
        ghost_fill_alpha:   90,
        ghost_stroke_width: 0.0,
        pastel_accent: true,
    }
}
