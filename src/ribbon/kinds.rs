//! Enums for where a ribbon lives (`Side` / `Bar` / `RibbonKind`)
//! and what ribbons a draggable button may legally land on
//! (`RibbonConstraint`).
//!
//! These are the only types most consumers need to touch — put them
//! near the top of your imports.

/// Which vertical rail a [`SideRibbon`](super::static_ribbon::SideRibbon)
/// sits on. Side rails carry menu-toggle buttons that open
/// floating panels next to the rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
}

/// Which horizontal bar a [`BarRibbon`](super::static_ribbon::BarRibbon)
/// sits on. Bars carry action-only buttons; they do not open
/// menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bar {
    Top,
    Bottom,
}

/// Which of the four screen-edge ribbons a button in the drag-aware
/// [`RibbonLayout`](super::layout::RibbonLayout) currently sits on.
/// This is the common "kind" shared between side rails and bars
/// when the layout needs to talk about them uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RibbonKind {
    Left,
    Right,
    Top,
    Bottom,
}

impl RibbonKind {
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// What ribbon(s) a draggable button may land on.
///
/// * [`Self::SideRails`] — Left ↔ Right. Menu-toggle buttons can be
///   rearranged along either rail or moved between them.
/// * [`Self::TopBar`] — pinned to the Top bar; reorders within it.
/// * [`Self::BottomBar`] — pinned to the Bottom bar; reorders within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonConstraint {
    SideRails,
    TopBar,
    BottomBar,
}

impl RibbonConstraint {
    pub fn allows(self, kind: RibbonKind) -> bool {
        match self {
            Self::SideRails => matches!(kind, RibbonKind::Left | RibbonKind::Right),
            Self::TopBar => kind == RibbonKind::Top,
            Self::BottomBar => kind == RibbonKind::Bottom,
        }
    }
}
