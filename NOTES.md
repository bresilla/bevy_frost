# NOTES.md

Gotchas worth remembering. Each entry is one numbered paragraph; if you
hit a weird bug whose root cause was far from the symptom, add it here.

1. **Flex paint-only items + `ui.available_size_before_wrap()` = wrong
   rects.** `egui_flex::Flex::show` runs each `flex.add_ui` closure
   twice per frame — once for the intrinsic-size measurement pass and
   once for the final layout/paint pass. If the closure reads its size
   from the inner `ui` (`available_size_before_wrap`, `max_rect`,
   `cursor`), the intrinsic pass sees the whole parent's size while the
   final pass sees the slot flex actually assigned, and paint runs
   twice with different rects. Symptoms range from animations looking
   half-ghosted (caution stripes, scramble text, blinking pip) to title
   rects collapsing to 25×25 squares to "almost-right-but-off-by-some-
   pixels" anchors that tempt you to add per-anchor push tables. Fix:
   pass the closure a size computed *outside* (e.g. from
   `TITLE_STRIP_THICKNESS` and the pane's `inner` rect) and use
   `ui.allocate_exact_size(that_size, …)` so both passes paint the same
   rect. Bit us in `crates/core/src/pane/mod.rs::lay_out_flex` and cost
   us hours of `far_flags` push-table debugging before we found it.
