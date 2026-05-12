use egui::Id;

use crate::view::ViewId;

use super::{RibbonAction, RibbonCluster, RibbonEdge};

/// Scope that decides when a slot-based ribbon participates in
/// resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RibbonScope {
    Permanent,
    View(ViewId),
    WorkspaceLevel(Id),
}

/// Stable id for an overridable ribbon slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RibbonSlotId(pub Id);

impl RibbonSlotId {
    #[must_use]
    pub fn new(source: impl std::hash::Hash) -> Self {
        Self(Id::new(source))
    }
}

/// Whether an active view/workspace layer may alter a permanent slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RibbonOverridePolicy {
    Fixed,
    LayerOverride,
    LayerAppend,
}

/// Slot-aware ribbon item. This intentionally does not replace the
/// existing assembly [`super::RibbonItem`] yet; it is the new
/// resolver model that can be lowered to assembly items later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonSlotItem {
    pub id: Id,
    pub icon: &'static str,
    pub label: String,
    pub tooltip: String,
    pub action: RibbonAction,
    pub active: bool,
}

impl RibbonSlotItem {
    #[must_use]
    pub fn new(
        id: impl Into<Id>,
        icon: &'static str,
        label: impl Into<String>,
        tooltip: impl Into<String>,
        action: RibbonAction,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            label: label.into(),
            tooltip: tooltip.into(),
            action,
            active: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonSlot {
    pub id: RibbonSlotId,
    pub default_item: Option<RibbonSlotItem>,
    pub override_policy: RibbonOverridePolicy,
}

impl RibbonSlot {
    #[must_use]
    pub fn new(
        id: RibbonSlotId,
        default_item: Option<RibbonSlotItem>,
        override_policy: RibbonOverridePolicy,
    ) -> Self {
        Self {
            id,
            default_item,
            override_policy,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonSlotOverride {
    pub slot: RibbonSlotId,
    pub item: Option<RibbonSlotItem>,
}

impl RibbonSlotOverride {
    #[must_use]
    pub fn new(slot: RibbonSlotId, item: RibbonSlotItem) -> Self {
        Self {
            slot,
            item: Some(item),
        }
    }

    /// Explicitly hide a slot for the active layer.
    ///
    /// This is the API-level opt-out for persistent bar icons: the
    /// main bar and its slots stay registered, but a view/workspace
    /// can intentionally suppress one inherited icon.
    #[must_use]
    pub fn hidden(slot: RibbonSlotId) -> Self {
        Self { slot, item: None }
    }

    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.item.is_none()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RibbonOverrideLayer {
    pub overrides: Vec<RibbonSlotOverride>,
}

impl RibbonOverrideLayer {
    #[must_use]
    pub fn new(overrides: Vec<RibbonSlotOverride>) -> Self {
        Self { overrides }
    }

    #[must_use]
    pub fn find(&self, slot: RibbonSlotId) -> Option<&RibbonSlotOverride> {
        self.overrides
            .iter()
            .find(|candidate| candidate.slot == slot)
    }

    #[must_use]
    pub fn with_hidden_slot(mut self, slot: RibbonSlotId) -> Self {
        self.overrides.push(RibbonSlotOverride::hidden(slot));
        self
    }
}

/// Slot-based ribbon declaration. Kept separate from the existing
/// assembly RibbonDef during migration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonSlotDef {
    pub id: Id,
    pub scope: RibbonScope,
    pub edge: RibbonEdge,
    pub cluster: RibbonCluster,
    pub slots: Vec<RibbonSlot>,
}

impl RibbonSlotDef {
    #[must_use]
    pub fn new(
        id: impl Into<Id>,
        scope: RibbonScope,
        edge: RibbonEdge,
        cluster: RibbonCluster,
        slots: Vec<RibbonSlot>,
    ) -> Self {
        Self {
            id: id.into(),
            scope,
            edge,
            cluster,
            slots,
        }
    }
}
