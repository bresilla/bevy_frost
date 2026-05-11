//! Built-in themes for the frost UI kit.
//!
//! Each theme is a single file under `core_crates/core/src/themes/`
//! that exports `pub const fn theme_<name>(mode: Mode) -> Theme`. The
//! [`crate::style::Theme`] struct (defined in `style.rs`) is the engine
//! — fields, helpers, global state, the de-dup cache, theme apply.
//! This module is the *catalogue*.
//!
//! # Adding a new theme
//!
//! 1. **Copy** an existing theme file as a template — e.g. start
//!    from [`pro`] (the canonical baseline) and tweak whichever
//!    fields you want to change. Every field has a doc comment on
//!    [`Theme`](crate::style::Theme) explaining what it does and
//!    what range it expects:
//!
//!    ```ignore
//!    // core_crates/core/src/themes/neon.rs
//!    use crate::style::{Mode, Theme, ColorMode, TextColorMode};
//!    use super::pro::theme_pro;
//!
//!    pub const fn theme_neon(mode: Mode) -> Theme {
//!        Theme {
//!            name: if matches!(mode, Mode::Dark) { "NEON_DARK" }
//!                  else { "NEON_LIGHT" },
//!            // … your overrides …
//!            ..theme_pro(mode)
//!        }
//!    }
//!    ```
//!
//!    The `..theme_pro(mode)` tail means you only have to spell out
//!    the fields that differ from PRO. Or, for a fully fresh theme,
//!    copy [`pro::theme_pro`] verbatim and rewrite every field.
//!
//! 2. **Register** the file by adding one line at the bottom of
//!    this module:
//!
//!    ```ignore
//!    pub mod neon;
//!    pub use neon::theme_neon;
//!    ```
//!
//! 3. **Activate** at runtime with the same API every other theme
//!    uses:
//!
//!    ```ignore
//!    frost_core::style::set_theme(theme_neon(Mode::Dark));
//!    ```
//!
//!    The de-dup cache in `apply_theme` keys on `Theme::name`, so
//!    make sure each Mode variant returns a unique `name` string.
//!
//! # What `name` does
//!
//! `Theme::name` is the de-dup key. `apply_theme` early-returns when
//! `(name, glass_opacity, accent)` matches the cached tuple — without
//! a unique `name` for each `Mode` variant, switching from `Dark` to
//! `Light` of the SAME theme would be a silent no-op. Convention:
//! suffix with `_DARK` / `_LIGHT` (or your own mode tokens).

pub mod pro;
pub mod game;

pub use pro::{
    theme_pro,
    PRO_LIGHT_BG_HOVER, PRO_LIGHT_BG_INPUT, PRO_LIGHT_BG_PANEL,
    PRO_LIGHT_BG_RAISED, PRO_LIGHT_BG_WINDOW, PRO_LIGHT_BORDER_INNER,
    PRO_LIGHT_BORDER_SUBTLE,
};
pub use game::{
    theme_game,
    GAME_LIGHT_BG_HOVER, GAME_LIGHT_BG_INPUT, GAME_LIGHT_BG_PANEL,
    GAME_LIGHT_BG_RAISED, GAME_LIGHT_BG_WINDOW,
};
