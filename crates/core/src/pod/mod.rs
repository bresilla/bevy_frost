//! # `Pod` — a configurable widget host that lives inside a container
//!
//! A pod is the *only* thing a container's body accepts. It hosts
//! one or more widgets (`text_input`, button, toggle, …) and
//! arranges them into a single visual unit.
//!
//! Widgets stack vertically in declaration order. Each widget's
//! per-frame response is collected into [`PodResponse`].
//!
//! ## Building one
//!
//! ```ignore
//! let pod = Pod::new(cid.with("settings"))
//!     .with_search("type something…", accent)
//!     .with_toggle("enabled", &mut toggle_state, accent)
//!     .with_button("apply", accent);
//! let resp = Normal::new(title, anchor, accent, cid).show(ui, vec![pod]);
//! if resp[0].buttons.first().map_or(false, |b| b.clicked) { ... }
//! ```

use egui::{Color32, Id, Ui};

use crate::container::SeparatorStyle;
use crate::style::UNIT;
use crate::widget::{
    button::card_button, button_h, drag_value_h, progressbar_h, slider_h, text_input_h,
    toggle_h,
};

// ─── Per-widget responses ─────────────────────────────────────────

/// What a [`Pod`] surfaces to the caller per frame. One vec per
/// widget kind, in declaration order within that kind.
#[derive(Clone, Debug, Default)]
pub struct PodResponse {
    pub searches: Vec<SearchResponse>,
    pub buttons: Vec<ButtonResponse>,
    pub card_buttons: Vec<ButtonResponse>,
    pub toggles: Vec<ToggleResponse>,
    pub progress: Vec<ProgressResponse>,
    pub sliders: Vec<SliderResponse>,
    pub drag_values: Vec<DragValueResponse>,
}

#[derive(Clone, Debug, Default)]
pub struct SearchResponse {
    pub query: String,
    pub changed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ButtonResponse {
    pub clicked: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ToggleResponse {
    pub on: bool,
    pub changed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ProgressResponse;

#[derive(Clone, Debug, Default)]
pub struct SliderResponse {
    pub value: f64,
    pub changed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct DragValueResponse {
    pub value: f64,
    pub changed: bool,
}

// ─── Widget specs ─────────────────────────────────────────────────

#[derive(Clone)]
struct SearchConfig {
    placeholder: String,
    accent: Color32,
}

#[derive(Clone)]
struct ButtonConfig {
    label: String,
    accent: Color32,
}

#[derive(Clone)]
struct ToggleConfig {
    label: String,
    accent: Color32,
    /// If `Some`, override the persisted state with this value.
    /// Caller can use this to drive the toggle from external state
    /// instead of relying on the ctx-data cache.
    initial: Option<bool>,
}

#[derive(Clone)]
struct ProgressConfig {
    label: String,
    fraction: f32,
    text: String,
    accent: Color32,
}

#[derive(Clone)]
struct SliderConfig {
    label: String,
    value: f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    suffix: String,
    accent: Color32,
}

#[derive(Clone)]
struct DragValueConfig {
    label: String,
    value: f64,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
    decimals: usize,
    suffix: String,
}

#[derive(Clone)]
struct CardButtonConfig {
    glyph: String,
    name: String,
    subtitle: String,
    accent: Color32,
}

/// One ordered slot in the pod's widget stack. Painted in
/// declaration order; response indices match the order each widget
/// kind was added (e.g. the third `with_button` shows up at
/// `response.buttons[2]`).
#[derive(Clone)]
enum WidgetSpec {
    Search(SearchConfig),
    Button(ButtonConfig),
    CardButton(CardButtonConfig),
    Toggle(ToggleConfig),
    Progress(ProgressConfig),
    Slider(SliderConfig),
    DragValue(DragValueConfig),
}

impl WidgetSpec {
    /// Number of 1U row-heights this widget consumes (for
    /// proportional resize accounting). Single-row widgets (search,
    /// 1U button, toggle, drag-value) → 1; the chunky `card_button`
    /// → ~1.7 (32 px ≈ 1.7U at default heights); 2-row widgets
    /// (progressbar, slider) → 2.
    fn unit_count(&self) -> usize {
        match self {
            WidgetSpec::Search(_) => 1,
            WidgetSpec::Button(_) => 1,
            WidgetSpec::CardButton(_) => 2,
            WidgetSpec::Toggle(_) => 1,
            WidgetSpec::Progress(_) => 2,
            WidgetSpec::Slider(_) => 2,
            WidgetSpec::DragValue(_) => 1,
        }
    }
}

// ─── Pod ──────────────────────────────────────────────────────────

/// A widget host that goes inside a container's body. Build with
/// [`Pod::new`], add widgets via the builder methods (`with_*`),
/// then hand to a container's `show`. Builder calls accumulate;
/// every widget paints in declaration order, top to bottom.
pub struct Pod {
    id: Id,
    widgets: Vec<WidgetSpec>,
    separator: SeparatorStyle,
    resizable: bool,
}

/// Lower bound on the per-widget height of a [`Pod::resizable`]
/// pod. Pinned to [`crate::style::UNIT`] — a widget can never
/// shrink below 1U regardless of how aggressively the user drags
/// the resize handle.
pub const POD_MIN_WIDGET_H: f32 = UNIT;
/// Upper bound on the per-widget height of a [`Pod::resizable`]
/// pod. Roughly 11U.
pub const POD_MAX_WIDGET_H: f32 = 240.0;

impl Pod {
    /// `id` scopes the per-widget persisted state and the debug-
    /// inspector label. Pass a stable value (e.g. derived from the
    /// container's id) so widget state survives across frames.
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            id: id.into(),
            widgets: Vec::new(),
            separator: SeparatorStyle::Line,
            resizable: false,
        }
    }

    /// Mark this pod resizable. The inter-pod separator below it
    /// becomes a vertical drag handle that grows / shrinks every
    /// widget inside the pod uniformly.
    pub fn resizable(mut self) -> Self {
        self.resizable = true;
        self
    }

    pub fn is_resizable(&self) -> bool {
        self.resizable
    }

    /// Persistence key for the resizable per-widget height.
    pub fn widget_height_key(id: Id) -> Id {
        id.with("frost_pod_widget_height")
    }

    /// Number of widgets the pod will paint.
    pub fn widget_count(&self) -> usize {
        self.widgets.len()
    }

    /// Total number of 1U row-heights the pod's widgets occupy.
    /// Single-row widgets (search / button / toggle) contribute 1
    /// each; multi-row widgets (`progressbar` = caption + bar)
    /// contribute their row count. Used by the inter-pod
    /// drag-resize handler to divide drag delta proportionally.
    pub fn unit_count(&self) -> usize {
        self.widgets.iter().map(|w| w.unit_count()).sum()
    }

    /// Override the separator painted AFTER this pod.
    pub fn with_separator(mut self, style: SeparatorStyle) -> Self {
        self.separator = style;
        self
    }

    pub fn separator_style(&self) -> SeparatorStyle {
        self.separator
    }

    pub fn id(&self) -> Id {
        self.id
    }

    /// Add a search widget (single-line [`crate::widget::text_input`]).
    /// Each search's query buffer is keyed off the pod's id + its
    /// index across the search slots, so multiple searches in the
    /// same pod persist independently.
    pub fn with_search(mut self, placeholder: impl Into<String>, accent: Color32) -> Self {
        self.widgets.push(WidgetSpec::Search(SearchConfig {
            placeholder: placeholder.into(),
            accent,
        }));
        self
    }

    /// Add a button widget. `label` is the centred caption.
    /// Click status is reported in `PodResponse::buttons[i]`.
    pub fn with_button(mut self, label: impl Into<String>, accent: Color32) -> Self {
        self.widgets.push(WidgetSpec::Button(ButtonConfig {
            label: label.into(),
            accent,
        }));
        self
    }

    /// Add a labelled toggle widget. Label sits left, pill track +
    /// knob sit right on the same row (1U). State persists in
    /// `ctx().data` keyed off the pod's id + toggle slot index.
    pub fn with_toggle(mut self, label: impl Into<String>, accent: Color32) -> Self {
        self.widgets.push(WidgetSpec::Toggle(ToggleConfig {
            label: label.into(),
            accent,
            initial: None,
        }));
        self
    }

    /// Add a toggle initialised to `initial` if no persisted state
    /// exists for that slot yet. Once the user clicks, the
    /// persisted value takes over.
    pub fn with_toggle_initial(
        mut self,
        label: impl Into<String>,
        accent: Color32,
        initial: bool,
    ) -> Self {
        self.widgets.push(WidgetSpec::Toggle(ToggleConfig {
            label: label.into(),
            accent,
            initial: Some(initial),
        }));
        self
    }

    /// Add a labelled progress bar (read-only). 2 rows.
    pub fn with_progress(
        mut self,
        label: impl Into<String>,
        fraction: f32,
        text: impl Into<String>,
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::Progress(ProgressConfig {
            label: label.into(),
            fraction,
            text: text.into(),
            accent,
        }));
        self
    }

    /// Add a labelled slider. 2 rows (caption + interactive bar).
    /// Initial value is read from `value`; user drags update the
    /// pod's persisted slot. Read the resolved value back from
    /// `PodResponse::sliders[i].value`.
    pub fn with_slider(
        mut self,
        label: impl Into<String>,
        value: f64,
        range: std::ops::RangeInclusive<f64>,
        decimals: usize,
        suffix: impl Into<String>,
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::Slider(SliderConfig {
            label: label.into(),
            value,
            range,
            decimals,
            suffix: suffix.into(),
            accent,
        }));
        self
    }

    /// Add a labelled `egui::DragValue` numeric input. 1 row.
    pub fn with_drag_value(
        mut self,
        label: impl Into<String>,
        value: f64,
        speed: f64,
        range: std::ops::RangeInclusive<f64>,
        decimals: usize,
        suffix: impl Into<String>,
    ) -> Self {
        self.widgets.push(WidgetSpec::DragValue(DragValueConfig {
            label: label.into(),
            value,
            speed,
            range,
            decimals,
            suffix: suffix.into(),
        }));
        self
    }

    /// Add a `card_button` — accent glyph on the left, primary
    /// `name` + small `subtitle` stacked on the right. Click status
    /// is reported in `PodResponse::card_buttons[i]`.
    pub fn with_card_button(
        mut self,
        glyph: impl Into<String>,
        name: impl Into<String>,
        subtitle: impl Into<String>,
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::CardButton(CardButtonConfig {
            glyph: glyph.into(),
            name: name.into(),
            subtitle: subtitle.into(),
            accent,
        }));
        self
    }

    /// Render the pod into `ui`. Returns a [`PodResponse`] with
    /// per-widget summaries grouped by kind.
    pub fn show(self, ui: &mut Ui) -> PodResponse {
        let pod_id = self.id;
        let mut response = PodResponse::default();
        // Inter-widget breathing space.
        const WIDGET_SPACING: f32 = 4.0;
        // Resolve the per-widget height: resizable pods read
        // persisted size (written by Normal::show on drag of the
        // inter-pod handle), defaulting to 1U.
        let widget_h: f32 = if self.resizable {
            ui.ctx()
                .data_mut(|d| d.get_persisted::<f32>(Self::widget_height_key(pod_id)))
                .unwrap_or(UNIT)
                .clamp(POD_MIN_WIDGET_H, POD_MAX_WIDGET_H)
        } else {
            UNIT
        };
        // Per-kind stable indices: the Nth `with_search` keeps its
        // own ctx-data key independent of any buttons / toggles /
        // progress bars declared between them.
        let mut search_idx = 0usize;
        let mut button_idx = 0usize;
        let mut card_button_idx = 0usize;
        let mut toggle_idx = 0usize;
        let mut progress_idx = 0usize;
        let mut slider_idx = 0usize;
        let mut drag_value_idx = 0usize;
        for (slot_idx, spec) in self.widgets.into_iter().enumerate() {
            if slot_idx > 0 {
                ui.add_space(WIDGET_SPACING);
            }
            // Each widget slot gets its own pushed id chain. This
            // is what keeps an explicit id derivation like
            // `ui.id().with(("frost_toggle", label))` from
            // colliding across pods that happen to share the same
            // label — the pushed id (= pod_id ⊕ slot_idx) is
            // unique per (pod, widget slot), so every child id
            // inherits uniqueness.
            ui.push_id((pod_id, slot_idx), |ui| match spec {
                WidgetSpec::Search(cfg) => {
                    let buf_key = pod_id.with(("frost_pod_search_buf", search_idx));
                    let mut buf: String = ui
                        .ctx()
                        .data(|d| d.get_temp::<String>(buf_key))
                        .unwrap_or_default();
                    let resp =
                        text_input_h(ui, &mut buf, &cfg.placeholder, cfg.accent, widget_h);
                    let changed = resp.changed();
                    if changed {
                        ui.ctx().data_mut(|d| d.insert_temp(buf_key, buf.clone()));
                    }
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[text_input/search #{}]", search_idx),
                    );
                    response.searches.push(SearchResponse {
                        query: buf,
                        changed,
                    });
                    search_idx += 1;
                }
                WidgetSpec::Button(cfg) => {
                    let resp = button_h(ui, &cfg.label, cfg.accent, widget_h);
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[button #{}]", button_idx),
                    );
                    response.buttons.push(ButtonResponse {
                        clicked: resp.clicked(),
                    });
                    button_idx += 1;
                }
                WidgetSpec::Toggle(cfg) => {
                    let state_key = pod_id.with(("frost_pod_toggle_state", toggle_idx));
                    let mut on: bool = ui.ctx().data_mut(|d| {
                        if let Some(stored) = d.get_persisted::<bool>(state_key) {
                            stored
                        } else {
                            let v = cfg.initial.unwrap_or(false);
                            d.insert_persisted(state_key, v);
                            v
                        }
                    });
                    let resp = toggle_h(ui, &cfg.label, &mut on, cfg.accent, widget_h);
                    let changed = resp.changed();
                    if changed {
                        ui.ctx().data_mut(|d| d.insert_persisted(state_key, on));
                    }
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!(
                            "widget[toggle #{}{}]",
                            toggle_idx,
                            if cfg.label.is_empty() {
                                String::new()
                            } else {
                                format!(" \"{}\"", cfg.label)
                            }
                        ),
                    );
                    response.toggles.push(ToggleResponse { on, changed });
                    toggle_idx += 1;
                }
                WidgetSpec::Progress(cfg) => {
                    let resp = progressbar_h(
                        ui,
                        &cfg.label,
                        cfg.fraction,
                        &cfg.text,
                        cfg.accent,
                        widget_h,
                    );
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[progress #{}]", progress_idx),
                    );
                    response.progress.push(ProgressResponse);
                    progress_idx += 1;
                }
                WidgetSpec::Slider(cfg) => {
                    // Persist the current value so user drags
                    // accumulate across frames without the caller
                    // having to thread state.
                    let val_key = pod_id.with(("frost_pod_slider_val", slider_idx));
                    let mut val: f64 = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<f64>(val_key))
                        .unwrap_or(cfg.value);
                    let resp = slider_h(
                        ui,
                        &cfg.label,
                        &mut val,
                        cfg.range.clone(),
                        cfg.decimals,
                        &cfg.suffix,
                        cfg.accent,
                        widget_h,
                    );
                    let changed = resp.changed();
                    if changed {
                        ui.ctx().data_mut(|d| d.insert_persisted(val_key, val));
                    }
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[slider #{}]", slider_idx),
                    );
                    response.sliders.push(SliderResponse {
                        value: val,
                        changed,
                    });
                    slider_idx += 1;
                }
                WidgetSpec::DragValue(cfg) => {
                    let val_key =
                        pod_id.with(("frost_pod_drag_value_val", drag_value_idx));
                    let mut val: f64 = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<f64>(val_key))
                        .unwrap_or(cfg.value);
                    let resp = drag_value_h(
                        ui,
                        &cfg.label,
                        &mut val,
                        cfg.speed,
                        cfg.range.clone(),
                        cfg.decimals,
                        &cfg.suffix,
                        widget_h,
                    );
                    let changed = resp.changed();
                    if changed {
                        ui.ctx().data_mut(|d| d.insert_persisted(val_key, val));
                    }
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[drag_value #{}]", drag_value_idx),
                    );
                    response.drag_values.push(DragValueResponse {
                        value: val,
                        changed,
                    });
                    drag_value_idx += 1;
                }
                WidgetSpec::CardButton(cfg) => {
                    let resp = card_button(
                        ui,
                        &cfg.glyph,
                        &cfg.name,
                        &cfg.subtitle,
                        cfg.accent,
                    );
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[card_button #{}]", card_button_idx),
                    );
                    response.card_buttons.push(ButtonResponse {
                        clicked: resp.clicked(),
                    });
                    card_button_idx += 1;
                }
            });
        }
        response
    }
}
