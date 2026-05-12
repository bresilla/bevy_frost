use crate::ribbon::{RibbonOverrideLayer, RibbonSlotDef};

use super::{ViewCtx, ViewId};

/// Top-level routable L0 screen/mode.
///
/// A view is selected by root/permanent chrome. It owns the L0
/// workspace for that screen. If something is also embeddable, it
/// can implement both this trait and [`crate::module::FrostModule`].
pub trait FrostView {
    fn id(&self) -> ViewId;
    fn title(&self) -> &str;
    fn icon(&self) -> &'static str;

    fn ribbons(&mut self) -> Vec<RibbonSlotDef> {
        Vec::new()
    }

    fn ribbon_overrides(&mut self) -> RibbonOverrideLayer {
        RibbonOverrideLayer::default()
    }

    fn show(&mut self, ctx: &mut ViewCtx<'_>);
}
