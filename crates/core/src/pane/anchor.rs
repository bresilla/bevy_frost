//! Anchor model for [`super::Pane2`] — where the pane sits on screen
//! and which side of itself it carries the title strip on.

/// One of the 4 screen rails the pane can live on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneAnchor {
    /// Vertical pane on the LEFT screen rail.
    LeftRail(RailZone),
    /// Vertical pane on the RIGHT screen rail.
    RightRail(RailZone),
    /// Horizontal pane on the TOP screen rail.
    TopRail(RailZone),
    /// Horizontal pane on the BOTTOM screen rail.
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
    /// `true` if the pane is taller than it is wide (Left/Right rail).
    /// Drives the flex direction.
    pub fn is_vertical_pane(self) -> bool {
        matches!(self, PaneAnchor::LeftRail(_) | PaneAnchor::RightRail(_))
    }

    /// Which side of the pane the title strip sits on.
    /// Middle-zone panes use the rail-anchor side (the original
    /// convention). All corner-zone (Start/End) panes flip:
    /// vertical-pane corners get a horizontal title; horizontal-pane
    /// corners get a vertical title — perpendicular to the rail.
    pub(crate) fn title_side(self) -> TitleSide {
        match self {
            PaneAnchor::LeftRail(RailZone::Start)   => TitleSide::Top,
            PaneAnchor::LeftRail(RailZone::End)     => TitleSide::Bottom,
            PaneAnchor::RightRail(RailZone::Start)  => TitleSide::Top,
            PaneAnchor::RightRail(RailZone::End)    => TitleSide::Bottom,
            PaneAnchor::TopRail(RailZone::Start)    => TitleSide::Left,
            PaneAnchor::TopRail(RailZone::End)      => TitleSide::Right,
            PaneAnchor::BottomRail(RailZone::Start) => TitleSide::Left,
            PaneAnchor::BottomRail(RailZone::End)   => TitleSide::Right,
            PaneAnchor::LeftRail(_)                 => TitleSide::Left,
            PaneAnchor::RightRail(_)                => TitleSide::Right,
            PaneAnchor::TopRail(_)                  => TitleSide::Top,
            PaneAnchor::BottomRail(_)               => TitleSide::Bottom,
        }
    }

    /// `true` → reverse the title text's reading-start so the FIRST
    /// letter sits next to the pane's own button on the rail. After
    /// flipping TE/RS to perpendicular title strips, the "reversed"
    /// set is TS, RS, RE, BE.
    pub(crate) fn title_reversed(self) -> bool {
        matches!(
            self,
            PaneAnchor::TopRail(RailZone::Start)
                | PaneAnchor::RightRail(RailZone::Start)
                | PaneAnchor::RightRail(RailZone::End)
                | PaneAnchor::BottomRail(RailZone::End)
        )
    }

    /// `true` for the four Middle-zone anchors (LM, RM, TM, BM).
    /// Middle panes get centred title text and (in GAME) a pip at
    /// each end of the strip.
    pub(crate) fn is_middle(self) -> bool {
        matches!(
            self,
            PaneAnchor::LeftRail(RailZone::Middle)
                | PaneAnchor::RightRail(RailZone::Middle)
                | PaneAnchor::TopRail(RailZone::Middle)
                | PaneAnchor::BottomRail(RailZone::Middle)
        )
    }
}

/// Which side of the pane rect carries the title strip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TitleSide {
    Top,
    Bottom,
    Left,
    Right,
}

impl TitleSide {
    pub(crate) fn is_horizontal_strip(self) -> bool {
        matches!(self, TitleSide::Top | TitleSide::Bottom)
    }
    pub(crate) fn is_at_end(self) -> bool {
        matches!(self, TitleSide::Bottom | TitleSide::Right)
    }
}

// `far_flags` (per-anchor extra-inset table) lived here in earlier
// revisions to compensate for what looked like layout bugs at
// certain corner anchors. The actual cause was the flex
// intrinsic-size pass double-painting title strips at the wrong
// rect — fixed in `super::Pane2::lay_out_flex`. With the paint
// fix, every anchor lands cleanly on a uniform `RAIL_INSET`, so
// the per-anchor table got removed.
