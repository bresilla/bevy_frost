//! `Tab` — one labelled body inside a [`super::normal::Normal::show_tabs`]
//! tabbed container. Each tab carries its own title, icon, and pod
//! list; the tab strip renders one button per tab projecting from
//! the title-facing edge of the container, folder-style — the active
//! tab merges into the container body (same fill, no seam) while
//! inactive tabs are outlined empty boxes the parent pane bg shows
//! through.
//!
//! ```ignore
//! Normal::new("Transform", anchor, accent, cid).show_tabs(ui, vec![
//!     Tab::new("Position", "arrow-move").pods(vec![pod_x, pod_y, pod_z]),
//!     Tab::new("Rotation", "arrow-rotate-clockwise").pods(vec![pod_rx]),
//!     Tab::new("Scale",    "maximize").pods(vec![pod_sx, pod_sy, pod_sz]),
//! ]);
//! ```

use crate::icons::Icon;
use crate::pod::Pod;

pub struct Tab {
    pub(crate) title: String,
    pub(crate) icon: Icon<'static>,
    pub(crate) pods: Vec<Pod>,
}

impl Tab {
    pub fn new(title: impl Into<String>, icon: impl Into<Icon<'static>>) -> Self {
        Self { title: title.into(), icon: icon.into(), pods: Vec::new() }
    }

    pub fn pods(mut self, pods: impl IntoIterator<Item = Pod>) -> Self {
        self.pods = pods.into_iter().collect();
        self
    }
}
