//! # `Pod` — a configurable widget host that lives inside a container
//!
//! A pod is the *only* thing a container's body accepts. It hosts
//! one or more widgets (`text_input`, button, list, …) and arranges
//! them into a single visual unit. Concrete examples:
//!
//! * a search-only pod (one `text_input` widget),
//! * a file picker (search + breadcrumb + list + a few buttons),
//! * a colour ramp editor (multiple sliders + a swatch row),
//!
//! …all are the same `Pod` struct, just with different builder
//! methods called on construction.
//!
//! ## Widgets vs. pods
//!
//! Widgets are leaf nodes — they paint a single primitive (a text
//! input, a button, a slider). Pods compose widgets into a useful
//! cluster. A search bar by itself is a widget; a "search and
//! results" pair is a pod.
//!
//! ## Lifecycle
//!
//! A pod is built fresh each frame (egui-style); per-widget state
//! (search query, scroll position, …) lives in `ui.ctx().data(...)`
//! keyed off the pod's `Id`. Destroying and rebuilding the pod
//! struct between frames does NOT lose state.
//!
//! ## Building one
//!
//! ```ignore
//! let pod = Pod::new(cid.with("settings_pod"))
//!     .with_search("type something…", accent_col);
//! Normal::new(title, anchor, accent_col, cid)
//!     .show(ui, pod);
//! ```

use egui::{Color32, Id, Ui};

use crate::widget::text_input;

/// What a [`Pod`] surfaces to the caller per frame. Each field is
/// a list of per-widget responses in declaration order — the first
/// `with_search` call's response sits at index 0, the second at
/// index 1, and so on.
#[derive(Clone, Debug, Default)]
pub struct PodResponse {
    /// One [`SearchResponse`] per search widget the pod was built
    /// with (via [`Pod::with_search`]). Empty when no search
    /// widget was added.
    pub searches: Vec<SearchResponse>,
}

/// Per-frame summary from a search widget hosted in a [`Pod`].
#[derive(Clone, Debug, Default)]
pub struct SearchResponse {
    /// Current query string (the persisted buffer at the end of the
    /// frame).
    pub query: String,
    /// `true` when the user typed / pasted / cleared this frame.
    pub changed: bool,
}

#[derive(Clone)]
struct SearchConfig {
    placeholder: String,
    accent: Color32,
}

/// A widget host that goes inside a container's body. Build with
/// [`Pod::new`], add widgets via the builder methods (`with_*`),
/// then hand to a container's `show`. Builder calls accumulate —
/// calling `with_search` three times produces a pod with three
/// search widgets stacked in declaration order.
pub struct Pod {
    id: Id,
    searches: Vec<SearchConfig>,
}

impl Pod {
    /// `id` scopes the per-widget persisted state (search query,
    /// scroll, …) and the debug-inspector label. Pass a stable
    /// value (e.g. derived from the container's id) so widget
    /// state survives across frames.
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            id: id.into(),
            searches: Vec::new(),
        }
    }

    /// The pod's id. Exposed so callers (e.g.
    /// [`crate::container::Normal`]) can label the pod's outer
    /// rect in the debug inspector even after the pod has been
    /// consumed by `show`.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Add a search widget (single-line `text_input`) to the pod.
    /// Multiple calls stack — each call adds another search bar
    /// after the previous one. Each search's query buffer is
    /// keyed off the pod's id + its index, so they persist
    /// independently across frames.
    pub fn with_search(mut self, placeholder: impl Into<String>, accent: Color32) -> Self {
        self.searches.push(SearchConfig {
            placeholder: placeholder.into(),
            accent,
        });
        self
    }

    /// Render the pod into `ui`. Returns a [`PodResponse`] with
    /// per-widget summaries; entries are `None` when the
    /// corresponding widget wasn't configured.
    ///
    /// The pod itself does NOT register a debug tag for its outer
    /// rect — that's the caller's job (typically the container)
    /// because the meaningful Pod rect includes the container's
    /// pod-padding Frame, which the pod has no reference to from
    /// the inside. Per-widget tags ARE registered here.
    pub fn show(self, ui: &mut Ui) -> PodResponse {
        let pod_id = self.id;
        let mut response = PodResponse::default();
        // Small breathing space between successive widgets so multi-
        // widget pods don't stack flush against each other.
        const WIDGET_SPACING: f32 = 4.0;
        for (i, cfg) in self.searches.into_iter().enumerate() {
            if i > 0 {
                ui.add_space(WIDGET_SPACING);
            }
            let buf_key = pod_id.with(("frost_pod_search_buf", i));
            let mut buf: String = ui
                .ctx()
                .data(|d| d.get_temp::<String>(buf_key))
                .unwrap_or_default();
            let resp = text_input(ui, &mut buf, &cfg.placeholder, cfg.accent);
            let changed = resp.changed();
            if changed {
                ui.ctx().data_mut(|d| d.insert_temp(buf_key, buf.clone()));
            }
            crate::debug::tag(
                ui,
                resp.rect,
                format!("widget[text_input/search #{}]", i),
            );
            response.searches.push(SearchResponse {
                query: buf,
                changed,
            });
        }
        response
    }
}
