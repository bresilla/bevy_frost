//! Vendored third-party crates we want to modify heavily without
//! upstreaming — copied in at specific revisions, then re-exported
//! through sibling frostcore modules (`crate::snarl`, `crate::code`)
//! which wrap them in the frost theme language.
//!
//! See `ACKNOWLEDGEMENTS.md` at the repo root for licenses +
//! upstream attribution (MIT / Apache-2.0).
//!
//! ## What's here
//!
//! * [`snarl`] — node-graph widget, originally `egui-snarl` 0.9.0.
//!   Upstream: <https://github.com/zakarumych/egui-snarl>.
//! * [`code_editor`] — syntax-highlighting text editor, originally
//!   `egui_code_editor` 0.2.21.
//!   Upstream: <https://github.com/p4ymak/egui_code_editor>.
//!
//! Neither vendor is meant to stay faithful to upstream. Feel free
//! to edit in place.

pub mod code_editor;
pub mod snarl;
