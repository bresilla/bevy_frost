use egui::Id;

use crate::view::ViewId;

use super::{RibbonAction, RibbonCluster, RibbonEdge, RibbonMode, RibbonRole};

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

/// Slot-aware ribbon item. This is the single public ribbon button
/// declaration; featureful chrome fields let it keep drag, panel,
/// and fullscreen behavior without exposing a second button type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonSlotItem {
    pub id: Id,
    /// Optional stable id for featureful chrome.
    ///
    /// When this is present, the slot item can participate in the
    /// same chrome path as drag/reorderable ribbons, panel toggles,
    /// pane anchoring, and fullscreen rails. This is how the slot API
    /// and the original featureful API converge instead of competing.
    pub chrome_id: Option<&'static str>,
    pub chrome_tooltip: Option<&'static str>,
    pub icon: &'static str,
    pub label: String,
    pub tooltip: String,
    pub action: RibbonAction,
    pub active: bool,
    pub role: Option<RibbonRole>,
    pub child_ribbon: Option<&'static str>,
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
            chrome_id: None,
            chrome_tooltip: None,
            icon,
            label: label.into(),
            tooltip: tooltip.into(),
            action,
            active: false,
            role: None,
            child_ribbon: None,
        }
    }

    /// Construct a slot item that is immediately eligible for the
    /// featureful chrome.
    ///
    /// Use this for app chrome that needs the original ribbon
    /// capabilities: draggable placement, panel toggles, live pane
    /// anchors, and fullscreen layering. The same item still carries
    /// slot actions/override semantics.
    #[must_use]
    pub fn featureful(
        id: &'static str,
        icon: &'static str,
        label: impl Into<String>,
        tooltip: &'static str,
        action: RibbonAction,
    ) -> Self {
        Self::new(Id::new(id), icon, label, tooltip, action)
            .with_chrome_id(id)
            .with_chrome_tooltip(tooltip)
    }

    #[must_use]
    pub fn with_chrome_id(mut self, id: &'static str) -> Self {
        self.chrome_id = Some(id);
        self
    }

    #[must_use]
    pub fn with_chrome_tooltip(mut self, tooltip: &'static str) -> Self {
        self.chrome_tooltip = Some(tooltip);
        self
    }

    #[must_use]
    pub fn with_role(mut self, role: RibbonRole) -> Self {
        self.role = Some(role);
        self
    }

    #[must_use]
    pub fn as_panel_button(mut self) -> Self {
        self.role = Some(RibbonRole::Panel);
        self
    }

    #[must_use]
    pub fn as_icon_button(mut self) -> Self {
        self.role = Some(RibbonRole::Icon);
        self
    }

    #[must_use]
    pub fn with_child_ribbon(mut self, child: &'static str) -> Self {
        self.child_ribbon = Some(child);
        self
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

/// Slot-based ribbon declaration. This is the single public ribbon
/// declaration; featureful chrome fields let it keep drag, panel,
/// and fullscreen behavior without exposing a second ribbon type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonSlotDef {
    pub id: Id,
    /// Optional stable id for featureful chrome.
    pub chrome_id: Option<&'static str>,
    pub scope: RibbonScope,
    pub edge: RibbonEdge,
    pub role: RibbonRole,
    pub mode: RibbonMode,
    pub cluster: RibbonCluster,
    pub draggable: bool,
    pub accepts: &'static [&'static str],
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
            chrome_id: None,
            scope,
            edge,
            role: RibbonRole::Panel,
            mode: RibbonMode::ThreeSided,
            cluster,
            draggable: false,
            accepts: &[],
            slots,
        }
    }

    /// Construct a slot ribbon that is immediately eligible for the
    /// featureful chrome while still participating in scope/layer
    /// resolution.
    #[must_use]
    pub fn featureful(
        id: &'static str,
        scope: RibbonScope,
        edge: RibbonEdge,
        cluster: RibbonCluster,
        slots: Vec<RibbonSlot>,
    ) -> Self {
        Self::new(Id::new(id), scope, edge, cluster, slots).with_chrome_id(id)
    }

    #[must_use]
    pub fn with_chrome_id(mut self, id: &'static str) -> Self {
        self.chrome_id = Some(id);
        self
    }

    #[must_use]
    pub fn with_role(mut self, role: RibbonRole) -> Self {
        self.role = role;
        self
    }

    #[must_use]
    pub fn with_mode(mut self, mode: RibbonMode) -> Self {
        self.mode = mode;
        self
    }

    #[must_use]
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    #[must_use]
    pub fn accepts(mut self, accepts: &'static [&'static str]) -> Self {
        self.accepts = accepts;
        self
    }
}
