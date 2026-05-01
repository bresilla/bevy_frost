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

/// Default flow-axis size of a container (height for horizontal-strip
/// containers, width for vertical-strip). Used the very first time a
/// container renders, before the user has dragged any inter-container
/// resize handle. Subsequent frames use whatever was persisted via
/// [`set_container_flow`].
pub const CONTAINER_DEFAULT_FLOW: f32 = 200.0;
/// Lower bound on a container's persisted flow size — keeps the
/// user from collapsing a container down to nothing on drag.
pub const CONTAINER_MIN_FLOW: f32 = crate::style::UNIT;
/// Upper bound on a container's persisted flow size — beyond this
/// the layout breaks down and the user can't see what they're
/// doing.
pub const CONTAINER_MAX_FLOW: f32 = 1200.0;

fn container_flow_key(cid: Id) -> Id {
    cid.with("frost_container_flow")
}

/// Read the persisted flow-axis size for the container identified
/// by `cid`. Defaults to [`CONTAINER_DEFAULT_FLOW`]. Always
/// returned clamped to `[CONTAINER_MIN_FLOW, CONTAINER_MAX_FLOW]`.
pub fn container_flow(ctx: &egui::Context, cid: Id) -> f32 {
    ctx.data_mut(|d| d.get_persisted::<f32>(container_flow_key(cid)))
        .unwrap_or(CONTAINER_DEFAULT_FLOW)
        .clamp(CONTAINER_MIN_FLOW, CONTAINER_MAX_FLOW)
}

/// Persist the flow-axis size for the container identified by
/// `cid`. Clamped to `[CONTAINER_MIN_FLOW, CONTAINER_MAX_FLOW]`
/// before writing.
pub fn set_container_flow(ctx: &egui::Context, cid: Id, value: f32) {
    let v = value.clamp(CONTAINER_MIN_FLOW, CONTAINER_MAX_FLOW);
    ctx.data_mut(|d| d.insert_persisted(container_flow_key(cid), v));
}
