//! # egui_frost — plain-egui facade for the frost UI kit.
//!
//! Mirrors `bevy_frost`, minus the Bevy bits. Re-exports every
//! public item from [`corekit`] verbatim and adds a single
//! convenience helper ([`apply_theme_now`]) so `eframe` apps can
//! one-line the per-frame theme refresh that `bevy_frost`'s
//! `ThemePlugin` does automatically.
//!
//! ```ignore
//! use eframe::egui;
//! use egui_frost::prelude::*;
//!
//! struct App {
//!     accent: AccentColor,
//!     glass:  GlassOpacity,
//! }
//!
//! impl eframe::App for App {
//!     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!         apply_theme_now(ctx, self.accent, self.glass);
//!         egui::CentralPanel::default().show(ctx, |ui| {
//!             let mut on = false;
//!             toggle(ui, "power", &mut on, self.accent.0);
//!         });
//!     }
//! }
//! ```
//!
//! Plain-egui hosts don't get the Bevy-side input firewall
//! (`bevy_frost::EguiInputAbsorbPlugin`) — they don't need it,
//! since `eframe` doesn't have a 3D scene competing for the same
//! pointer events.

pub use corekit::*;

/// Per-frame theme refresh — wraps [`corekit::style::set_glass_opacity`]
/// and [`corekit::style::apply_theme`] so eframe `update` methods
/// can stay one-liners. Idempotent; safe to call every frame.
pub fn apply_theme_now(
    ctx: &egui::Context,
    accent: corekit::style::AccentColor,
    glass: corekit::style::GlassOpacity,
) {
    corekit::style::set_glass_opacity(glass.0);
    corekit::style::apply_theme(ctx, accent, glass);
}

/// Glob-import. Mirrors `bevy_frost::prelude` — apps that flip
/// between Bevy and eframe hosts get the same module surface
/// from `<facade>::prelude::*` and only the `main` differs.
pub mod prelude {
    pub use corekit::*;
    pub use super::apply_theme_now;
}
