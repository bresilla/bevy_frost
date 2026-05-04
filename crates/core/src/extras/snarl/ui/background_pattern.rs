use egui::{emath::Rot2, vec2, Painter, Rect, Style, Vec2};

use super::SnarlStyle;

///Grid background pattern.
///Use `SnarlStyle::background_pattern_stroke` for change stroke options
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct Grid {
    /// Spacing between grid lines.
    pub spacing: Vec2,

    /// Angle of the grid.
    #[cfg_attr(feature = "egui-probe", egui_probe(as egui_probe::angle))]
    pub angle: f32,
}

const DEFAULT_GRID_SPACING: Vec2 = vec2(50.0, 50.0);
macro_rules! default_grid_spacing {
    () => {
        stringify!(vec2(50.0, 50.0))
    };
}

const DEFAULT_GRID_ANGLE: f32 = 1.0;
macro_rules! default_grid_angle {
    () => {
        stringify!(1.0)
    };
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            spacing: DEFAULT_GRID_SPACING,
            angle: DEFAULT_GRID_ANGLE,
        }
    }
}

impl Grid {
    /// Create new grid with given spacing and angle.
    #[must_use]
    pub const fn new(spacing: Vec2, angle: f32) -> Self {
        Self { spacing, angle }
    }

    fn draw(&self, viewport: &Rect, snarl_style: &SnarlStyle, style: &Style, painter: &Painter) {
        let bg_stroke = snarl_style.get_bg_pattern_stroke(style);

        let spacing = vec2(self.spacing.x.max(1.0), self.spacing.y.max(1.0));

        let rot = Rot2::from_angle(self.angle);
        let rot_inv = rot.inverse();

        let pattern_bounds = viewport.rotate_bb(rot_inv);

        let min_x = (pattern_bounds.min.x / spacing.x).ceil();
        let max_x = (pattern_bounds.max.x / spacing.x).floor();

        #[allow(clippy::cast_possible_truncation)]
        for x in 0..=f32::ceil(max_x - min_x) as i64 {
            #[allow(clippy::cast_precision_loss)]
            let x = (x as f32 + min_x) * spacing.x;

            let top = (rot * vec2(x, pattern_bounds.min.y)).to_pos2();
            let bottom = (rot * vec2(x, pattern_bounds.max.y)).to_pos2();

            painter.line_segment([top, bottom], bg_stroke);
        }

        let min_y = (pattern_bounds.min.y / spacing.y).ceil();
        let max_y = (pattern_bounds.max.y / spacing.y).floor();

        #[allow(clippy::cast_possible_truncation)]
        for y in 0..=f32::ceil(max_y - min_y) as i64 {
            #[allow(clippy::cast_precision_loss)]
            let y = (y as f32 + min_y) * spacing.y;

            let top = (rot * vec2(pattern_bounds.min.x, y)).to_pos2();
            let bottom = (rot * vec2(pattern_bounds.max.x, y)).to_pos2();

            painter.line_segment([top, bottom], bg_stroke);
        }
    }
}

/// Dot grid background pattern — small filled circles at each
/// grid intersection. Matches Blender's node-editor dot grid.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub struct Dots {
    /// Spacing between dots.
    pub spacing: Vec2,
    /// Dot radius in points.
    pub radius: f32,
}

impl Default for Dots {
    fn default() -> Self {
        Self { spacing: DEFAULT_GRID_SPACING, radius: 1.0 }
    }
}

impl Dots {
    /// Create new dot pattern with given spacing and radius.
    #[must_use]
    pub const fn new(spacing: Vec2, radius: f32) -> Self {
        Self { spacing, radius }
    }

    fn draw(&self, viewport: &Rect, snarl_style: &SnarlStyle, style: &Style, painter: &Painter) {
        let stroke = snarl_style.get_bg_pattern_stroke(style);
        let fill = stroke.color;

        let spacing = vec2(self.spacing.x.max(1.0), self.spacing.y.max(1.0));

        let min_x = (viewport.min.x / spacing.x).ceil();
        let max_x = (viewport.max.x / spacing.x).floor();
        let min_y = (viewport.min.y / spacing.y).ceil();
        let max_y = (viewport.max.y / spacing.y).floor();

        #[allow(clippy::cast_possible_truncation)]
        let nx = f32::ceil(max_x - min_x) as i64;
        #[allow(clippy::cast_possible_truncation)]
        let ny = f32::ceil(max_y - min_y) as i64;

        for ix in 0..=nx {
            #[allow(clippy::cast_precision_loss)]
            let x = (ix as f32 + min_x) * spacing.x;
            for iy in 0..=ny {
                #[allow(clippy::cast_precision_loss)]
                let y = (iy as f32 + min_y) * spacing.y;
                painter.circle_filled(egui::pos2(x, y), self.radius, fill);
            }
        }
    }
}

/// Background pattern show beneath nodes and wires.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "egui-probe", derive(egui_probe::EguiProbe))]
pub enum BackgroundPattern {
    /// No pattern.
    NoPattern,

    /// Linear grid.
    #[cfg_attr(feature = "egui-probe", egui_probe(transparent))]
    Grid(Grid),

    /// Dot grid (Blender-style) — a filled circle at each
    /// intersection of an invisible grid.
    #[cfg_attr(feature = "egui-probe", egui_probe(transparent))]
    Dots(Dots),
}

impl Default for BackgroundPattern {
    fn default() -> Self {
        BackgroundPattern::new()
    }
}

impl BackgroundPattern {
    /// Create new background pattern with default values.
    ///
    /// Default patter is `Grid` with spacing - `
    #[doc = default_grid_spacing!()]
    /// ` and angle - `
    #[doc = default_grid_angle!()]
    /// ` radian.
    #[must_use]
    pub const fn new() -> Self {
        Self::Grid(Grid::new(DEFAULT_GRID_SPACING, DEFAULT_GRID_ANGLE))
    }

    /// Create new grid background pattern with given spacing and angle.
    #[must_use]
    pub const fn grid(spacing: Vec2, angle: f32) -> Self {
        Self::Grid(Grid::new(spacing, angle))
    }

    /// Draws background pattern.
    pub fn draw(
        &self,
        viewport: &Rect,
        snarl_style: &SnarlStyle,
        style: &Style,
        painter: &Painter,
    ) {
        match self {
            BackgroundPattern::Grid(g) => g.draw(viewport, snarl_style, style, painter),
            BackgroundPattern::Dots(d) => d.draw(viewport, snarl_style, style, painter),
            BackgroundPattern::NoPattern => {}
        }
    }
}
