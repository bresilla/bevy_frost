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
    button::{Button, FillStyle},
    color_rgb, color_rgba, drag_value, dropdown, hybrid_select_row, progressbar, readout,
    select_row, slider, text_input, toggle,
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
    pub dropdowns: Vec<DropdownResponse>,
    pub selects: Vec<SelectResponse>,
    pub hybrid_selects: Vec<HybridSelectPodResponse>,
    pub colors: Vec<ColorResponse>,
    pub readouts: Vec<ReadoutResponse>,
    pub select_lists: Vec<SelectListResponse>,
    pub hybrid_select_lists: Vec<HybridSelectListResponse>,
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

#[derive(Clone, Debug, Default)]
pub struct DropdownResponse {
    pub selected: usize,
    pub changed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SelectResponse {
    pub clicked: bool,
    pub double_clicked: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, Default)]
pub struct HybridSelectPodResponse {
    pub body_clicked: bool,
    pub body_double_clicked: bool,
    pub radio_clicked: bool,
    pub selected: bool,
    pub radio_on: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ColorResponse {
    /// RGBA in 0.0..=1.0. For `with_color_rgb`, alpha is always 1.0.
    pub rgba: [f32; 4],
    pub changed: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ReadoutResponse;

#[derive(Clone, Debug, Default)]
pub struct SelectListResponse {
    /// Index of the row that was clicked this frame, if any.
    pub clicked: Option<usize>,
    /// Index of the row that was double-clicked this frame, if any.
    pub double_clicked: Option<usize>,
    /// Persisted "currently selected" index for the list.
    pub selected: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct HybridSelectListResponse {
    /// Body click target — same as `SelectListResponse::clicked`.
    pub body_clicked: Option<usize>,
    pub body_double_clicked: Option<usize>,
    /// Right-edge radio click — independent from body.
    pub radio_clicked: Option<usize>,
    pub selected: Option<usize>,
    /// Persisted "pinned" radio index — at most one row pinned at a
    /// time (the radio is single-select, like a real radio group).
    pub pinned: Option<usize>,
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
    /// Optional second-row caption beneath the label (small dim text).
    /// When `Some`, the button paints in the 2-row "card" shape and
    /// reports its result in `PodResponse::card_buttons` instead of
    /// `buttons`, so callers that rely on the index split keep
    /// matching the right wire.
    subtitle: Option<String>,
    /// Optional leading icon glyph painted in `accent`.
    glyph: Option<String>,
    /// Optional CSS-style hover-fill animation. `None` falls back to
    /// the standard hover/press tint.
    animation: Option<FillStyle>,
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
struct DropdownConfig {
    options: Vec<String>,
    initial: usize,
    accent: Color32,
}

#[derive(Clone)]
struct SelectConfig {
    label: String,
    trailing: Option<String>,
    selected_initial: bool,
    accent: Color32,
}

#[derive(Clone)]
struct HybridSelectConfig {
    label: String,
    trailing: Option<String>,
    selected_initial: bool,
    radio_initial: bool,
    accent: Color32,
}

#[derive(Clone)]
struct ColorConfig {
    label: String,
    initial: [f32; 4],
    /// `true` shows the alpha slider in the picker (RGBA);
    /// `false` keeps it opaque (RGB).
    alpha: bool,
    accent: Color32,
}

#[derive(Clone)]
struct ReadoutConfig {
    label: String,
    value: String,
}

#[derive(Clone)]
struct SelectListConfig {
    items: Vec<String>,
    /// Optional trailing text per item (e.g. `#3`, `(2.4 MB)`).
    /// When `Some`, length must match `items.len()`. When `None`,
    /// rows render with no trailing column.
    trailing: Option<Vec<String>>,
    accent: Color32,
}

#[derive(Clone)]
struct HybridSelectListConfig {
    items: Vec<String>,
    trailing: Option<Vec<String>>,
    accent: Color32,
}

/// One ordered slot in the pod's widget stack. Painted in
/// declaration order; response indices match the order each widget
/// kind was added (e.g. the third `with_button` shows up at
/// `response.buttons[2]`).
///
/// Not `Clone` — the [`WidgetSpec::Custom`] variant carries a move-only
/// closure (`Box<dyn FnOnce>`). Pod consumes its widget vec on
/// `show(self, ui)`, so cloning was never needed in the first place;
/// removing the derive lets the custom variant exist without
/// special-casing.
enum WidgetSpec {
    Search(SearchConfig),
    Button(ButtonConfig),
    Toggle(ToggleConfig),
    Progress(ProgressConfig),
    Slider(SliderConfig),
    DragValue(DragValueConfig),
    Dropdown(DropdownConfig),
    Select(SelectConfig),
    HybridSelect(HybridSelectConfig),
    Color(ColorConfig),
    Readout(ReadoutConfig),
    /// Multi-row select list — ONE widget that paints N
    /// `select_row`s. Use this instead of stacking N
    /// [`WidgetSpec::Select`] entries when "the list IS the widget"
    /// (the conceptual unit is the whole roster).
    SelectList(SelectListConfig),
    /// Multi-row hybrid select list — body click + right-edge radio
    /// pin per row. Body selection is independent per row; the radio
    /// is single-select across the list (only one row pinned at a
    /// time), matching the "active layer / current camera target"
    /// pattern.
    HybridSelectList(HybridSelectListConfig),
    /// Caller-supplied paint closure. Used as the integration point
    /// for widgets that don't fit a flat config (recursive trees,
    /// node graphs, code editors, …) — the closure draws into the
    /// pod's `Ui`, allocating whatever vertical space it needs.
    /// `unit_count` for `Custom` defaults to `1`; pass an explicit
    /// hint via [`Pod::with_custom_units`] when the closure paints
    /// significantly more rows.
    Custom {
        units: usize,
        paint: Box<dyn FnOnce(&mut Ui) + Send + Sync>,
    },
}

impl WidgetSpec {
    /// Number of 1U row-heights this widget consumes (for
    /// proportional resize accounting). Single-row widgets (search,
    /// 1U button, toggle, drag-value, dropdown, select) → 1; the
    /// chunky button-with-subtitle → 2 (32 px ≈ 1.7U at default
    /// heights, rounded up); 2-row widgets (progressbar, slider) → 2.
    /// `Custom` returns its caller-supplied hint so the resize-handle
    /// math still adds up.
    fn unit_count(&self) -> usize {
        match self {
            WidgetSpec::Search(_) => 1,
            WidgetSpec::Button(cfg) => {
                if cfg.subtitle.is_some() {
                    2
                } else {
                    1
                }
            }
            WidgetSpec::Toggle(_) => 1,
            WidgetSpec::Progress(_) => 2,
            WidgetSpec::Slider(_) => 2,
            WidgetSpec::DragValue(_) => 1,
            WidgetSpec::Dropdown(_) => 1,
            WidgetSpec::Select(_) => 1,
            WidgetSpec::HybridSelect(_) => 1,
            WidgetSpec::Color(_) => 1,
            WidgetSpec::Readout(_) => 1,
            WidgetSpec::SelectList(cfg) => cfg.items.len().max(1),
            WidgetSpec::HybridSelectList(cfg) => cfg.items.len().max(1),
            WidgetSpec::Custom { units, .. } => *units,
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

    /// Add a plain button widget. `label` is the centred caption.
    /// Click status is reported in `PodResponse::buttons[i]`.
    pub fn with_button(mut self, label: impl Into<String>, accent: Color32) -> Self {
        self.widgets.push(WidgetSpec::Button(ButtonConfig {
            label: label.into(),
            accent,
            subtitle: None,
            glyph: None,
            animation: None,
        }));
        self
    }

    /// Add a button with a small dim caption underneath the primary
    /// label (chunky 2-row look). Click status is reported in
    /// `PodResponse::card_buttons[i]` so callers can split the wires
    /// from plain buttons.
    pub fn with_button_subtitle(
        mut self,
        label: impl Into<String>,
        subtitle: impl Into<String>,
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::Button(ButtonConfig {
            label: label.into(),
            accent,
            subtitle: Some(subtitle.into()),
            glyph: None,
            animation: None,
        }));
        self
    }

    /// Add a button with a CSS-style hover-fill animation overlay.
    /// At rest the button paints the same as `with_button`; on hover
    /// it paints `style` over a darker-accent fill.
    pub fn with_button_animated(
        mut self,
        label: impl Into<String>,
        accent: Color32,
        style: FillStyle,
    ) -> Self {
        self.widgets.push(WidgetSpec::Button(ButtonConfig {
            label: label.into(),
            accent,
            subtitle: None,
            glyph: None,
            animation: Some(style),
        }));
        self
    }

    /// Add a fully-configured button — combine any of subtitle,
    /// glyph (Fluent icon name or literal), and animation in one
    /// call. The simpler `with_button*` shortcuts cover the common
    /// cases; reach for this when you need (e.g.) "icon + 2-row
    /// label + animated hover" all together. Subtitle bumps the
    /// height to 2U automatically.
    pub fn with_button_styled(
        mut self,
        label: impl Into<String>,
        accent: Color32,
        subtitle: Option<impl Into<String>>,
        glyph: Option<impl Into<String>>,
        animation: Option<FillStyle>,
    ) -> Self {
        self.widgets.push(WidgetSpec::Button(ButtonConfig {
            label: label.into(),
            accent,
            subtitle: subtitle.map(Into::into),
            glyph: glyph.map(Into::into),
            animation,
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

    /// Add a "card" button — leading glyph + primary `name` + small
    /// `subtitle`. Click status is reported in
    /// `PodResponse::card_buttons[i]`.
    pub fn with_card_button(
        mut self,
        glyph: impl Into<String>,
        name: impl Into<String>,
        subtitle: impl Into<String>,
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::Button(ButtonConfig {
            label: name.into(),
            accent,
            subtitle: Some(subtitle.into()),
            glyph: Some(glyph.into()),
            animation: None,
        }));
        self
    }

    /// Add a single-select dropdown. `options` is the menu list;
    /// `initial` is the default index until the user picks something
    /// (subsequent selections persist in the pod's ctx-data slot).
    /// Result lands in `PodResponse::dropdowns[i]`.
    pub fn with_dropdown(
        mut self,
        options: impl IntoIterator<Item = impl Into<String>>,
        initial: usize,
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::Dropdown(DropdownConfig {
            options: options.into_iter().map(Into::into).collect(),
            initial,
            accent,
        }));
        self
    }

    /// Add a select row (single click target on the body). The
    /// `selected` paint state persists in the pod's ctx-data slot —
    /// each click toggles it. `trailing` is rendered dim-right.
    /// Result lands in `PodResponse::selects[i]`.
    pub fn with_select(
        mut self,
        label: impl Into<String>,
        trailing: Option<impl Into<String>>,
        selected_initial: bool,
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::Select(SelectConfig {
            label: label.into(),
            trailing: trailing.map(Into::into),
            selected_initial,
            accent,
        }));
        self
    }

    /// Add a hybrid-select row (body click + right-edge radio
    /// pin). The radio's `radio_on` state persists in its own
    /// ctx-data slot. Result lands in `PodResponse::hybrid_selects[i]`.
    pub fn with_hybrid_select(
        mut self,
        label: impl Into<String>,
        trailing: Option<impl Into<String>>,
        selected_initial: bool,
        radio_initial: bool,
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::HybridSelect(HybridSelectConfig {
            label: label.into(),
            trailing: trailing.map(Into::into),
            selected_initial,
            radio_initial,
            accent,
        }));
        self
    }

    /// Add an opaque sRGB colour swatch. Click expands the picker
    /// inline below the row. Result lands in `PodResponse::colors[i]`
    /// (alpha is fixed at 1.0 in the result).
    pub fn with_color_rgb(
        mut self,
        label: impl Into<String>,
        initial_rgb: [f32; 3],
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::Color(ColorConfig {
            label: label.into(),
            initial: [initial_rgb[0], initial_rgb[1], initial_rgb[2], 1.0],
            alpha: false,
            accent,
        }));
        self
    }

    /// Add an sRGBA colour swatch (alpha slider in the picker).
    /// Result lands in `PodResponse::colors[i]`.
    pub fn with_color_rgba(
        mut self,
        label: impl Into<String>,
        initial_rgba: [f32; 4],
        accent: Color32,
    ) -> Self {
        self.widgets.push(WidgetSpec::Color(ColorConfig {
            label: label.into(),
            initial: initial_rgba,
            alpha: true,
            accent,
        }));
        self
    }

    /// Add a read-only readout row — label on the left, monospace
    /// value on the right. Use for surfaces that just *display* a
    /// piece of data (selected node path, current speed, active
    /// tool, …). Result is reported in `PodResponse::readouts[i]`,
    /// though the response carries no state — re-render the pod with
    /// a new `value` to update what's shown.
    pub fn with_readout(
        mut self,
        label: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.widgets.push(WidgetSpec::Readout(ReadoutConfig {
            label: label.into(),
            value: value.into(),
        }));
        self
    }

    /// Add a multi-row select list as ONE widget. Each item becomes a
    /// `select_row`; selection state persists per-list (a single
    /// "current row" index, like a single-select listbox) in
    /// `PodResponse::select_lists[i].selected`. Pass `trailing` to
    /// add a dim-right column per row; length must equal `items` or
    /// the list is rendered without trailing.
    pub fn with_select_list(
        mut self,
        items: impl IntoIterator<Item = impl Into<String>>,
        trailing: Option<Vec<String>>,
        accent: Color32,
    ) -> Self {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        let trailing = trailing.filter(|t| t.len() == items.len());
        self.widgets.push(WidgetSpec::SelectList(SelectListConfig {
            items,
            trailing,
            accent,
        }));
        self
    }

    /// Add a multi-row hybrid select list — body click + radio pin
    /// per row, all bundled as ONE widget. Body select is single-row
    /// (current selection); radio pin is also single-row (only one
    /// row pinned at a time, like a real radio group). Result
    /// indices land in `PodResponse::hybrid_select_lists[i]`.
    pub fn with_hybrid_select_list(
        mut self,
        items: impl IntoIterator<Item = impl Into<String>>,
        trailing: Option<Vec<String>>,
        accent: Color32,
    ) -> Self {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        let trailing = trailing.filter(|t| t.len() == items.len());
        self.widgets
            .push(WidgetSpec::HybridSelectList(HybridSelectListConfig {
                items,
                trailing,
                accent,
            }));
        self
    }

    /// Add a caller-supplied paint closure as a widget slot. Use for
    /// custom rendering that doesn't fit one of the flat configs —
    /// the canonical case being [`crate::widget::tree_row`], which is
    /// recursive and needs the caller to walk its model.
    ///
    /// The closure runs inside the pod's per-slot `push_id` scope so
    /// any `ui.id().with(...)` derivations stay unique across pods.
    /// Allocate vertical space normally (`ui.allocate_exact_size` /
    /// child uis); the pod's flow accounting resizes around whatever
    /// the closure paints.
    ///
    /// `Custom` widgets surface no per-frame response back through
    /// [`PodResponse`] — the closure owns its own state and reactions.
    /// The unit hint defaults to `1`; call [`Pod::with_custom_units`]
    /// when the closure paints more than one 1U row's worth so the
    /// inter-pod drag-resize math remains proportional.
    pub fn with_custom(
        mut self,
        paint: impl FnOnce(&mut Ui) + Send + Sync + 'static,
    ) -> Self {
        self.widgets.push(WidgetSpec::Custom {
            units: 1,
            paint: Box::new(paint),
        });
        self
    }

    /// Like [`Pod::with_custom`] but with an explicit "this slot
    /// occupies N units of 1U row height" hint, used by the
    /// inter-pod resize-handle to share drag delta across pods
    /// proportionally to their content size.
    pub fn with_custom_units(
        mut self,
        units: usize,
        paint: impl FnOnce(&mut Ui) + Send + Sync + 'static,
    ) -> Self {
        self.widgets.push(WidgetSpec::Custom {
            units: units.max(1),
            paint: Box::new(paint),
        });
        self
    }

    /// Render the pod into `ui`. Returns a [`PodResponse`] with
    /// per-widget summaries grouped by kind.
    pub fn show(self, ui: &mut Ui) -> PodResponse {
        let pod_id = self.id;
        let mut response = PodResponse::default();
        if self.resizable {
            // Compute the pod's natural total height — sum of widget
            // unit_count × UNIT + inter-widget spacing — and resolve
            // the viewport from the persisted handle (default =
            // natural sum so a fresh pod shows everything; drag
            // clamps to [POD_MIN, POD_MAX]).
            let natural_units: usize =
                self.widgets.iter().map(|w| w.unit_count()).sum();
            let spacing_total = if self.widgets.len() > 1 {
                (self.widgets.len() - 1) as f32 * POD_WIDGET_SPACING
            } else {
                0.0
            };
            let natural_h = (natural_units as f32) * UNIT + spacing_total;
            let viewport_h = ui
                .ctx()
                .data_mut(|d| d.get_persisted::<f32>(Self::widget_height_key(pod_id)))
                .unwrap_or(natural_h)
                .clamp(POD_MIN_WIDGET_H, POD_MAX_WIDGET_H);
            let avail_w = ui.available_width().max(1.0);
            let (slot_rect, _) = ui.allocate_exact_size(
                egui::vec2(avail_w, viewport_h),
                egui::Sense::hover(),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(slot_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            // `shrink_clip_rect` (= intersect with current clip) so a
            // pod inside an already-clipped container can never grow
            // its own clip. Hierarchy stays intact:
            //   widget rect ⊆ pod slot ⊆ container body ⊆ pane.
            child.shrink_clip_rect(slot_rect);
            // Wrap the iteration in a vertical `ScrollArea` so when
            // content exceeds the viewport, the user can SCROLL
            // through the hidden rows — pods become first-class
            // nested scrollable areas. `auto_shrink([false, false])`
            // keeps the area filling the slot regardless of content
            // size; `min_scrolled_height(0.0)` disables egui's
            // default 64px floor that otherwise inflates inner_size
            // for short pods. Bar visibility is `VisibleWhenNeeded`
            // (the default) so the bar appears only when content
            // overflows.
            let widgets = self.widgets;
            egui::ScrollArea::vertical()
                .id_salt(pod_id.with("frost_pod_scroll"))
                .auto_shrink([false, false])
                .min_scrolled_height(0.0)
                .show(&mut child, |inner| {
                    paint_widgets(widgets, inner, &mut response, pod_id);
                });
        } else {
            paint_widgets(self.widgets, ui, &mut response, pod_id);
        }
        response
    }
}

/// Inter-widget vertical breathing space inside a pod. Used both
/// when laying out widgets in [`paint_widgets`] and when computing a
/// resizable pod's natural height in [`Pod::show`].
const POD_WIDGET_SPACING: f32 = 4.0;

/// Paint every widget in `widgets` into `ui`, accumulating responses
/// into `response`. Shared between [`Pod::show`]'s plain (parent ui)
/// and resizable (clipped child + ScrollArea) paths so the per-widget
/// rendering logic lives in exactly one place.
fn paint_widgets(
    widgets: Vec<WidgetSpec>,
    ui: &mut egui::Ui,
    response: &mut PodResponse,
    pod_id: Id,
) {
        const WIDGET_SPACING: f32 = POD_WIDGET_SPACING;
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
        let mut dropdown_idx = 0usize;
        let mut select_idx = 0usize;
        let mut hybrid_select_idx = 0usize;
        let mut color_idx = 0usize;
        let mut readout_idx = 0usize;
        let mut select_list_idx = 0usize;
        let mut hybrid_select_list_idx = 0usize;
        for (slot_idx, spec) in widgets.into_iter().enumerate() {
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
                        text_input(ui, &mut buf, &cfg.placeholder, cfg.accent);
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
                    let has_subtitle = cfg.subtitle.is_some();
                    // Card-shaped button (subtitle and/or glyph) gets
                    // its own height + result wire so callers can
                    // index them independently of plain buttons.
                    let mut builder = Button::new(&cfg.label);
                    if let Some(s) = &cfg.subtitle {
                        builder = builder.subtitle(s);
                    }
                    if let Some(g) = &cfg.glyph {
                        builder = builder.glyph(g);
                    }
                    if let Some(a) = cfg.animation {
                        builder = builder.animation(a);
                    }
                    // No `.height()` override — let the Button
                    // builder pick its natural default (24 px plain
                    // / 39 px with subtitle). The pod's resize
                    // handle is no longer allowed to scale the
                    // button; if the pod's viewport is smaller than
                    // the button's natural size, the button gets
                    // clipped instead.
                    let resp = builder.show(ui, cfg.accent);
                    if has_subtitle {
                        crate::debug::tag(
                            ui,
                            resp.rect,
                            format!("widget[card_button #{}]", card_button_idx),
                        );
                        response.card_buttons.push(ButtonResponse {
                            clicked: resp.clicked(),
                        });
                        card_button_idx += 1;
                    } else {
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
                    let resp = toggle(ui, &cfg.label, &mut on, cfg.accent);
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
                    let resp = progressbar(
                        ui,
                        &cfg.label,
                        cfg.fraction,
                        &cfg.text,
                        cfg.accent,
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
                    let resp = slider(
                        ui,
                        &cfg.label,
                        &mut val,
                        cfg.range.clone(),
                        cfg.decimals,
                        &cfg.suffix,
                        cfg.accent,
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
                    let resp = drag_value(
                        ui,
                        &cfg.label,
                        &mut val,
                        cfg.speed,
                        cfg.range.clone(),
                        cfg.decimals,
                        &cfg.suffix,
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
                WidgetSpec::Dropdown(cfg) => {
                    let val_key =
                        pod_id.with(("frost_pod_dropdown_idx", dropdown_idx));
                    let mut sel: usize = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<usize>(val_key))
                        .unwrap_or(cfg.initial)
                        .min(cfg.options.len().saturating_sub(1));
                    let opts: Vec<&str> =
                        cfg.options.iter().map(String::as_str).collect();
                    let resp = dropdown(
                        ui,
                        ("frost_pod_dropdown", dropdown_idx),
                        &mut sel,
                        &opts,
                        cfg.accent,
                    );
                    let changed = resp.changed();
                    if changed {
                        ui.ctx().data_mut(|d| d.insert_persisted(val_key, sel));
                    }
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[dropdown #{}]", dropdown_idx),
                    );
                    response.dropdowns.push(DropdownResponse {
                        selected: sel,
                        changed,
                    });
                    dropdown_idx += 1;
                }
                WidgetSpec::Select(cfg) => {
                    let sel_key =
                        pod_id.with(("frost_pod_select_sel", select_idx));
                    let mut selected: bool = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<bool>(sel_key))
                        .unwrap_or(cfg.selected_initial);
                    let resp = select_row(
                        ui,
                        ("frost_pod_select", select_idx),
                        &cfg.label,
                        cfg.trailing.as_deref(),
                        selected,
                        cfg.accent,
                    );
                    if resp.clicked() {
                        selected = !selected;
                        ui.ctx().data_mut(|d| d.insert_persisted(sel_key, selected));
                    }
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[select #{}]", select_idx),
                    );
                    response.selects.push(SelectResponse {
                        clicked: resp.clicked(),
                        double_clicked: resp.double_clicked(),
                        selected,
                    });
                    select_idx += 1;
                }
                WidgetSpec::HybridSelect(cfg) => {
                    let sel_key = pod_id
                        .with(("frost_pod_hybrid_sel", hybrid_select_idx));
                    let radio_key = pod_id
                        .with(("frost_pod_hybrid_radio", hybrid_select_idx));
                    let mut selected: bool = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<bool>(sel_key))
                        .unwrap_or(cfg.selected_initial);
                    let mut radio_on: bool = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<bool>(radio_key))
                        .unwrap_or(cfg.radio_initial);
                    let resp = hybrid_select_row(
                        ui,
                        ("frost_pod_hybrid", hybrid_select_idx),
                        &cfg.label,
                        cfg.trailing.as_deref(),
                        selected,
                        radio_on,
                        cfg.accent,
                    );
                    if resp.body.clicked() {
                        selected = !selected;
                        ui.ctx().data_mut(|d| d.insert_persisted(sel_key, selected));
                    }
                    if resp.radio.clicked() {
                        radio_on = !radio_on;
                        ui.ctx().data_mut(|d| d.insert_persisted(radio_key, radio_on));
                    }
                    crate::debug::tag(
                        ui,
                        resp.body.rect,
                        format!("widget[hybrid_select #{}]", hybrid_select_idx),
                    );
                    response.hybrid_selects.push(HybridSelectPodResponse {
                        body_clicked: resp.body.clicked(),
                        body_double_clicked: resp.body.double_clicked(),
                        radio_clicked: resp.radio.clicked(),
                        selected,
                        radio_on,
                    });
                    hybrid_select_idx += 1;
                }
                WidgetSpec::Color(cfg) => {
                    let val_key = pod_id.with(("frost_pod_color_val", color_idx));
                    let mut rgba: [f32; 4] = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<[f32; 4]>(val_key))
                        .unwrap_or(cfg.initial);
                    let changed = if cfg.alpha {
                        let resp =
                            color_rgba(ui, &cfg.label, &mut rgba, cfg.accent);
                        crate::debug::tag(
                            ui,
                            resp.rect,
                            format!("widget[color_rgba #{}]", color_idx),
                        );
                        resp.changed()
                    } else {
                        let mut rgb = [rgba[0], rgba[1], rgba[2]];
                        let resp = color_rgb(ui, &cfg.label, &mut rgb, cfg.accent);
                        rgba[0] = rgb[0];
                        rgba[1] = rgb[1];
                        rgba[2] = rgb[2];
                        rgba[3] = 1.0;
                        crate::debug::tag(
                            ui,
                            resp.rect,
                            format!("widget[color_rgb #{}]", color_idx),
                        );
                        resp.changed()
                    };
                    if changed {
                        ui.ctx().data_mut(|d| d.insert_persisted(val_key, rgba));
                    }
                    response.colors.push(ColorResponse { rgba, changed });
                    color_idx += 1;
                }
                WidgetSpec::Readout(cfg) => {
                    let resp = readout(ui, &cfg.label, &cfg.value);
                    crate::debug::tag(
                        ui,
                        resp.rect,
                        format!("widget[readout #{}]", readout_idx),
                    );
                    response.readouts.push(ReadoutResponse);
                    readout_idx += 1;
                }
                WidgetSpec::SelectList(cfg) => {
                    let sel_key = pod_id
                        .with(("frost_pod_select_list_sel", select_list_idx));
                    let mut selected: Option<usize> = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<Option<usize>>(sel_key))
                        .unwrap_or(None);
                    let mut clicked: Option<usize> = None;
                    let mut double_clicked: Option<usize> = None;
                    for (i, label) in cfg.items.iter().enumerate() {
                        let trailing = cfg.trailing.as_ref().map(|t| t[i].as_str());
                        let resp = select_row(
                            ui,
                            ("frost_pod_select_list", select_list_idx, i),
                            label,
                            trailing,
                            selected == Some(i),
                            cfg.accent,
                        );
                        if resp.clicked() {
                            clicked = Some(i);
                            selected = Some(i);
                        }
                        if resp.double_clicked() {
                            double_clicked = Some(i);
                        }
                    }
                    if clicked.is_some() {
                        ui.ctx()
                            .data_mut(|d| d.insert_persisted(sel_key, selected));
                    }
                    response.select_lists.push(SelectListResponse {
                        clicked,
                        double_clicked,
                        selected,
                    });
                    select_list_idx += 1;
                }
                WidgetSpec::HybridSelectList(cfg) => {
                    let sel_key = pod_id.with((
                        "frost_pod_hybrid_select_list_sel",
                        hybrid_select_list_idx,
                    ));
                    let pin_key = pod_id.with((
                        "frost_pod_hybrid_select_list_pin",
                        hybrid_select_list_idx,
                    ));
                    let mut selected: Option<usize> = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<Option<usize>>(sel_key))
                        .unwrap_or(None);
                    let mut pinned: Option<usize> = ui
                        .ctx()
                        .data_mut(|d| d.get_persisted::<Option<usize>>(pin_key))
                        .unwrap_or(None);
                    let mut body_clicked: Option<usize> = None;
                    let mut body_double_clicked: Option<usize> = None;
                    let mut radio_clicked: Option<usize> = None;
                    for (i, label) in cfg.items.iter().enumerate() {
                        let trailing = cfg.trailing.as_ref().map(|t| t[i].as_str());
                        let resp = hybrid_select_row(
                            ui,
                            (
                                "frost_pod_hybrid_select_list",
                                hybrid_select_list_idx,
                                i,
                            ),
                            label,
                            trailing,
                            selected == Some(i),
                            pinned == Some(i),
                            cfg.accent,
                        );
                        if resp.body.clicked() {
                            body_clicked = Some(i);
                            selected = Some(i);
                        }
                        if resp.body.double_clicked() {
                            body_double_clicked = Some(i);
                        }
                        if resp.radio.clicked() {
                            radio_clicked = Some(i);
                            // Single-select radio: clicking an
                            // unpinned row pins it; clicking the
                            // currently-pinned row unpins.
                            pinned = if pinned == Some(i) { None } else { Some(i) };
                        }
                    }
                    if body_clicked.is_some() {
                        ui.ctx()
                            .data_mut(|d| d.insert_persisted(sel_key, selected));
                    }
                    if radio_clicked.is_some() {
                        ui.ctx()
                            .data_mut(|d| d.insert_persisted(pin_key, pinned));
                    }
                    response
                        .hybrid_select_lists
                        .push(HybridSelectListResponse {
                            body_clicked,
                            body_double_clicked,
                            radio_clicked,
                            selected,
                            pinned,
                        });
                    hybrid_select_list_idx += 1;
                }
                WidgetSpec::Custom { paint, .. } => {
                    paint(ui);
                }
            });
        }
}
