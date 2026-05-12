use egui::Id;

/// Edge where a workspace-level bar is attached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkspaceBarEdge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Cluster along the selected edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkspaceBarCluster {
    Start,
    Middle,
    End,
}

/// Semantic bar item kind. Painting and layout stay theme-owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkspaceBarItemKind {
    Command,
    Toggle,
    Mode,
    Separator,
}

#[derive(Clone, Debug)]
pub struct WorkspaceBarItem {
    pub id: Id,
    pub label: String,
    pub icon: Option<&'static str>,
    pub kind: WorkspaceBarItemKind,
    pub active: bool,
}

impl WorkspaceBarItem {
    #[must_use]
    pub fn command(
        id: impl Into<Id>,
        label: impl Into<String>,
        icon: Option<&'static str>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            kind: WorkspaceBarItemKind::Command,
            active: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceBar {
    pub id: Id,
    pub edge: WorkspaceBarEdge,
    pub cluster: WorkspaceBarCluster,
    pub items: Vec<WorkspaceBarItem>,
}

impl WorkspaceBar {
    #[must_use]
    pub fn new(id: impl Into<Id>, edge: WorkspaceBarEdge, cluster: WorkspaceBarCluster) -> Self {
        Self {
            id: id.into(),
            edge,
            cluster,
            items: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_item(mut self, item: WorkspaceBarItem) -> Self {
        self.items.push(item);
        self
    }
}
