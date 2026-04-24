//! Per-side exclusive menu-activation state.
//!
//! Tracks which menu-id (if any) is open on each side rail. One
//! open at a time per side — clicking a button toggles the slot on
//! whichever side the button lives, so a button dragged to the
//! opposite rail plays exclusively with whatever already lived
//! there.

#[cfg(feature = "bevy")] use bevy::prelude::*;

use super::kinds::RibbonKind;
use super::layout::RibbonLayout;

/// Bevy resource. Persist its fields if you want open menus to
/// survive restarts.
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
#[derive(Default, Debug, Clone)]
pub struct SideActive {
    pub left: Option<String>,
    pub right: Option<String>,
}

impl SideActive {
    /// Menu id currently open on `side`, if any. `None` for bars.
    pub fn on(&self, side: RibbonKind) -> Option<&str> {
        match side {
            RibbonKind::Left => self.left.as_deref(),
            RibbonKind::Right => self.right.as_deref(),
            _ => None,
        }
    }

    /// Toggle `id` on `side`. Clicking the currently-open menu
    /// closes it; clicking any other menu replaces whatever was
    /// there. Bar sides are no-ops.
    pub fn toggle(&mut self, side: RibbonKind, id: &str) {
        let slot = match side {
            RibbonKind::Left => &mut self.left,
            RibbonKind::Right => &mut self.right,
            _ => return,
        };
        *slot = if slot.as_deref() == Some(id) {
            None
        } else {
            Some(id.to_string())
        };
    }

    /// Is the menu identified by `id` currently open? Consults
    /// `layout` to learn which side the button lives on, then
    /// checks whether that side's active slot matches `id`.
    pub fn is_menu_open(&self, layout: &RibbonLayout, id: &str) -> bool {
        match layout.ribbon_of(id) {
            Some(side) => self.on(side) == Some(id),
            None => false,
        }
    }

    /// Toggle the menu identified by `id` on whatever side its
    /// button is currently on. No-op for bar-pinned buttons.
    pub fn toggle_menu(&mut self, layout: &RibbonLayout, id: &str) {
        if let Some(side) = layout.ribbon_of(id) {
            self.toggle(side, id);
        }
    }

    /// Clear any active entries whose button is no longer on the
    /// matching side — happens right after a drag crosses sides.
    /// Call at the top of each frame, before reading `on(...)`.
    pub fn invalidate_stale(&mut self, layout: &RibbonLayout) {
        if let Some(id) = &self.left {
            if layout.ribbon_of(id) != Some(RibbonKind::Left) {
                self.left = None;
            }
        }
        if let Some(id) = &self.right {
            if layout.ribbon_of(id) != Some(RibbonKind::Right) {
                self.right = None;
            }
        }
    }
}
