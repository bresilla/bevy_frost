//! In-pane container widgets — visual blocks the caller drops into a
//! [`crate::pane::Pane2`] body to organise content. A container holds
//! [`crate::pod::Pod`]s and (between consecutive pods) separators —
//! see [`SeparatorStyle`].
//!
//! Variants:
//!
//! * [`normal`] — single title bar above a single body. The default
//!   building block; matches frostcore's `Section`.
//! * [`tabbed`] — multiple labelled bodies behind a tab strip
//!   (placeholder; not yet implemented).

pub mod body;
pub mod normal;
pub mod separator;
pub mod tabbed;

pub use body::Body;
pub use normal::Normal;
pub use separator::{paint_separator, paint_separator_resize, SeparatorOrient, SeparatorStyle};

use egui::Id;

/// First-frame default flow-axis size, used before any content has
/// been measured AND before any drag has set an explicit value.
pub const CONTAINER_DEFAULT_FLOW: f32 = 200.0;
/// Hard minimum on a *vertically-stacked* container's flow
/// (= height in horizontal-strip panes — TM/BM). Content drives
/// height; this is just the absolute floor so the user can't
/// collapse it to nothing.
pub const CONTAINER_MIN_FLOW: f32 = crate::style::UNIT;
/// Upper bound on any container's persisted flow size.
pub const CONTAINER_MAX_FLOW: f32 = 1200.0;
/// Auto-fit cap for vertically-stacked containers. While in
/// untouched "auto-fit" mode (no explicit drag yet), the flow
/// size tracks the previous frame's measured content height BUT
/// never exceeds this — past it the body's `ScrollArea` takes
/// over.
pub const CONTAINER_AUTOFIT_CAP: f32 = 8.0 * crate::style::UNIT;
/// Hard minimum for *horizontally-stacked* containers (= width
/// in vertical-strip panes — LM/RM). Their content doesn't drive
/// width (the pods stack vertically inside, so width is just
/// "available width"), so we pin a deliberate 12U default and
/// floor — wide enough for a search field + chrome to read
/// without cramping. The user CAN'T drag below this.
pub const CONTAINER_HORIZONTAL_MIN_FLOW: f32 = 12.0 * crate::style::UNIT;
/// First-frame default for horizontally-stacked containers, same
/// as the floor — they start "comfortable" and grow on demand.
pub const CONTAINER_HORIZONTAL_DEFAULT_FLOW: f32 =
    CONTAINER_HORIZONTAL_MIN_FLOW;

fn container_flow_key(cid: Id) -> Id {
    // Persisted ONLY when `set_container_flow` is called (= the
    // user has dragged the resize handle). Presence of this key
    // means "user has overridden the auto-fit"; absence means
    // "auto-fit from measured content".
    cid.with("frost_container_flow")
}

fn container_intrinsic_key(cid: Id) -> Id {
    // Measured content size of the container's body, written every
    // frame after the body renders (see
    // [`record_container_intrinsic`]). Read by [`container_flow`]
    // when no explicit user value is persisted, capped at
    // [`CONTAINER_AUTOFIT_CAP`] so very tall content scrolls
    // rather than ballooning the container.
    cid.with("frost_container_intrinsic")
}

/// Read the flow-axis size the container should render at. The
/// resolution depends on the parent pane's orientation —
/// `is_horizontal_strip == true` means the container lives in a
/// horizontal-strip pane (Top/Bottom title), where containers
/// stack VERTICALLY and flow = height; `false` means a vertical-
/// strip pane (Left/Right title), where containers stack
/// HORIZONTALLY and flow = width.
///
/// **Vertically-stacked (`is_horizontal_strip == true`)** —
/// content height varies, so the size is content-driven:
///
/// 1. Explicit user drag (persisted via [`set_container_flow`])
///    if any, clamped `[CONTAINER_MIN_FLOW, CONTAINER_MAX_FLOW]`.
/// 2. Otherwise auto-fit: previous frame's measured content height,
///    capped at [`CONTAINER_AUTOFIT_CAP`] (`8U`). Past the cap,
///    the body's `ScrollArea` takes over.
/// 3. First frame, no measurement yet: [`CONTAINER_DEFAULT_FLOW`].
///
/// **Horizontally-stacked (`is_horizontal_strip == false`)** —
/// content doesn't drive width (pods stack vertically inside, so
/// width is just available-width), so the size is fixed:
///
/// 1. Explicit user drag, clamped
///    `[CONTAINER_HORIZONTAL_MIN_FLOW, CONTAINER_MAX_FLOW]`.
/// 2. Default [`CONTAINER_HORIZONTAL_DEFAULT_FLOW`] (= 12U).
///    The user cannot drag below 12U.
pub fn container_flow(ctx: &egui::Context, cid: Id, is_horizontal_strip: bool) -> f32 {
    let (min_v, max_v) = container_flow_bounds(is_horizontal_strip);
    if let Some(user) =
        ctx.data_mut(|d| d.get_persisted::<f32>(container_flow_key(cid)))
    {
        return user.clamp(min_v, max_v);
    }
    if is_horizontal_strip {
        if let Some(intrinsic) =
            ctx.data_mut(|d| d.get_persisted::<f32>(container_intrinsic_key(cid)))
        {
            return intrinsic.min(CONTAINER_AUTOFIT_CAP).clamp(min_v, max_v);
        }
        CONTAINER_DEFAULT_FLOW.clamp(min_v, max_v)
    } else {
        CONTAINER_HORIZONTAL_DEFAULT_FLOW.clamp(min_v, max_v)
    }
}

/// Persist an explicit user override for the container's flow
/// size — called from the inter-container drag handler. Clamped to
/// the orientation-specific bounds before writing, so the user can't
/// drag below 12U on horizontally-stacked containers.
pub fn set_container_flow(
    ctx: &egui::Context,
    cid: Id,
    value: f32,
    is_horizontal_strip: bool,
) {
    let (min_v, max_v) = container_flow_bounds(is_horizontal_strip);
    let v = value.clamp(min_v, max_v);
    ctx.data_mut(|d| d.insert_persisted(container_flow_key(cid), v));
}

/// `(min, max)` bounds for a container's flow size based on the
/// parent pane's orientation. Vertically-stacked containers can
/// shrink to `UNIT`; horizontally-stacked containers have a hard
/// `12U` floor.
pub fn container_flow_bounds(is_horizontal_strip: bool) -> (f32, f32) {
    if is_horizontal_strip {
        (CONTAINER_MIN_FLOW, CONTAINER_MAX_FLOW)
    } else {
        (CONTAINER_HORIZONTAL_MIN_FLOW, CONTAINER_MAX_FLOW)
    }
}

/// Persist the measured intrinsic body content size for `cid`.
/// Called by [`Normal::show`] every frame after the body renders.
/// Read by [`container_flow`]'s auto-fit path on subsequent frames.
pub fn record_container_intrinsic(ctx: &egui::Context, cid: Id, height: f32) {
    let v = height.max(0.0);
    ctx.data_mut(|d| d.insert_persisted(container_intrinsic_key(cid), v));
}
