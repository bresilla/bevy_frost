//! PRO theme — soft glass, rounded corners, subtle accent-tinted
//! borders. Default theme on first launch.
//!
//! See [`crate::themes`](super) for the "how to add a theme" guide.

use crate::style::{
    radius, ColorMode, Mode, TextColorMode, Theme,
    // shared (dark-mode) palette + text constants
    BG_0_WINDOW, BG_1_PANEL, BG_2_RAISED, BG_3_HOVER, BG_4_INPUT,
    BORDER_INNER, BORDER_SUBTLE,
    TEXT_DISABLED, TEXT_DISABLED_LIGHT, TEXT_PRIMARY, TEXT_PRIMARY_LIGHT,
    TEXT_SECONDARY, TEXT_SECONDARY_LIGHT,
};

/// PRO Light surface palette — paper-tinted neutrals matching
/// GitHub Primer's light-mode tokens. Text colours are NOT defined
/// here; they come from the shared `TEXT_*_LIGHT` constants so all
/// light variants pick the same body-text tones.
pub const PRO_LIGHT_BG_WINDOW: egui::Color32 = egui::Color32::from_rgb(0xF5, 0xF5, 0xF7);
pub const PRO_LIGHT_BG_PANEL:  egui::Color32 = egui::Color32::from_rgb(0xFF, 0xFF, 0xFF);
// Raised + input tiers tightened — the previous values (`F6F8FA`
// raised, `FAFAFC` input) sat ~5 units off the white panel, so
// dropdowns and button surfaces were effectively invisible. Mirrors
// the Dark tier deltas (panel ± ~12 units) inverted toward darker
// grey.
pub const PRO_LIGHT_BG_RAISED: egui::Color32 = egui::Color32::from_rgb(0xF1, 0xF3, 0xF6);
pub const PRO_LIGHT_BG_HOVER:  egui::Color32 = egui::Color32::from_rgb(0xE6, 0xE8, 0xEC);
pub const PRO_LIGHT_BG_INPUT:  egui::Color32 = egui::Color32::from_rgb(0xEF, 0xF1, 0xF4);
pub const PRO_LIGHT_BORDER_SUBTLE:  egui::Color32 = egui::Color32::from_rgb(0xD1, 0xD9, 0xE0);
pub const PRO_LIGHT_BORDER_INNER:   egui::Color32 = egui::Color32::from_rgb(0xC5, 0xCC, 0xD3);

/// Built-in PRO profile — soft glass, rounded corners, subtle
/// accent-tinted borders. Pick a [`Mode`] to flip between the
/// original dark surfaces and a paper-tinted light variant; every
/// other field (shape / chrome / brackets) is shared across modes.
pub const fn theme_pro(mode: Mode) -> Theme {
    let dark = matches!(mode, Mode::Dark);
    Theme {
        name: if dark { "PRO_DARK" } else { "PRO_LIGHT" },
        is_light: !dark,
        bg_window:  if dark { BG_0_WINDOW } else { PRO_LIGHT_BG_WINDOW },
        bg_panel:   if dark { BG_1_PANEL  } else { PRO_LIGHT_BG_PANEL  },
        bg_raised:  if dark { BG_2_RAISED } else { PRO_LIGHT_BG_RAISED },
        bg_hover:   if dark { BG_3_HOVER  } else { PRO_LIGHT_BG_HOVER  },
        bg_input:   if dark { BG_4_INPUT  } else { PRO_LIGHT_BG_INPUT  },
        panel_fill_mode:    ColorMode::FromBg,
        section_fill_mode:  ColorMode::FromBg,
        section_show_frame: true,
        section_show_title_divider: true,
        section_pad_x: 2,
        section_pad_y: 2,
        section_body_indent: 8.0,
        section_outer_margin_flow_title: 3,
        section_outer_margin_flow_body: 3,
        section_outer_margin_span: 3,
        section_body_inner_top_pad: 0.0,
        pane_title_chromatic_aberration: false,
        // PRO — quick snappy fold / unfold so flipping sections
        // open while inspecting feels responsive.
        section_animation_time: 0.06,
        animations_enabled: true,
        button_anim_scale: 1.0,
        pane_fade_scale: 0.5,
        // Text — pulled from the SHARED light/dark tone constants so
        // every variant ends up with the same body-text colours.
        text_primary:   if dark { TEXT_PRIMARY }   else { TEXT_PRIMARY_LIGHT },
        text_secondary: if dark { TEXT_SECONDARY } else { TEXT_SECONDARY_LIGHT },
        text_disabled:  if dark { TEXT_DISABLED }  else { TEXT_DISABLED_LIGHT },
        title_color_mode: TextColorMode::Accent,
        title_softness: 0.0,
        ribbon_button_accent_fill: false,
        section_gap: 0.0,
        section_corner_ticks_inset: 0.0,
        section_title_brackets: false,
        section_title_prefix: None,
        section_title_letter_spacing: 0.0,
        subcaption_prefix: None,
        section_bottom_rule: false,
        pane_fill_visible: true,
        show_section_chevron: true,
        title_strip_filled: false,
        section_title_size: 11.0,
        body_accent_darken: 0.0,
        section_icon_at_end: false,
        section_icon_size: 0.0,
        section_body_top_pad: 0.0,
        row_separator_dash: None,
        section_title_trailing_rule: false,
        section_corner_ticks: 0.0,
        border_subtle:      if dark { BORDER_SUBTLE } else { PRO_LIGHT_BORDER_SUBTLE },
        border_inner:       if dark { BORDER_INNER }  else { PRO_LIGHT_BORDER_INNER  },
        border_alpha:       if dark { 70 } else { 100 },
        border_accent_tint: 0.06,
        border_width:       1.0,
        row_separator_alpha: if dark { 35 } else { 50 },
        glass_card_factor:  0.92,
        glass_group_factor: 0.78,
        glass_accent_tint:  0.03,
        radius_widget:  radius::WIDGET,
        radius_compact: radius::COMPACT,
        radius_sm:      radius::SM,
        radius_md:      radius::MD,
        radius_lg:      radius::LG,
        row_alternation: false,
        row_alt_lift: 0.0,
        button_full_accent_on_press: false,
        button_tint_rest:  0.08,
        button_tint_hover: 0.16,
        button_tint_press: 0.30,
        pane_shadow_blur:  24,
        pane_shadow_y:     8,
        pane_show_title_divider: true,
        pane_title_stripes: false,
        scramble_titles: false,
        tree_guide_width: 1.0,
        graph_pin_width:  1.0,
        graph_wire_glow:  0.6,
        graph_pin_glow:   0.5,
        graph_canvas_hex: false,
        progressbar_segmented: false,
        pane_title_brackets: false,
        section_separator_strip_h: 2.0,
        section_separator_alpha: 128,
        section_body_inner_end_pad: 0.0,
        ghost_fill_alpha:   28,
        ghost_stroke_width: 1.5,
        pastel_accent: true,
    }
}
