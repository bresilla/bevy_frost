//! # egui_frost — plain-egui facade for the frost UI kit.
//!
//! Re-exports every public item from [`frostcore`] verbatim. Use
//! this crate when you're driving egui from `eframe` (or any
//! standalone egui host) and don't want to pull Bevy into your
//! dependency tree.
//!
//! ```ignore
//! use egui_frost::prelude::*;
//!
//! impl eframe::App for MyApp {
//!     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!         // Re-apply the frost theme every frame — the function
//!         // de-dupes internally, so it's cheap.
//!         apply_theme(ctx, self.accent, self.glass);
//!
//!         egui::CentralPanel::default().show(ctx, |ui| {
//!             // frost widgets work with plain egui `Ui` refs.
//!             toggle(ui, "power", &mut self.power, self.accent.0);
//!         });
//!     }
//! }
//! ```

pub use frostcore::*;
