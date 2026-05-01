# corekit — third-party acknowledgements

Vendored or otherwise embedded code in this crate, with upstream
attribution. All sources are MIT or compatible.

## Vendored crates (`src/features/`)

### `egui_flex` 0.6.0
- **Path:** `src/features/flex/`
- **Upstream:** <https://github.com/lucasmerlin/hello_egui/tree/main/crates/egui_flex>
- **Author:** Lucas Meurer ([@lucasmerlin](https://github.com/lucasmerlin))
- **License:** MIT — full text at `src/features/flex/LICENSE`.
- **Why vendored:** flex is small enough that we own it directly to
  avoid an extra Cargo dep and to allow in-place edits.

## Bundled assets (`src/fonts/`)

### Iosevka (9 weights)
- **Files:** `iosevka-thin.ttf`, `iosevka-extralight.ttf`,
  `iosevka-light.ttf`, `iosevka-regular.ttf`, `iosevka-medium.ttf`,
  `iosevka-semibold.ttf`, `iosevka-bold.ttf`, `iosevka-extrabold.ttf`,
  `iosevka-heavy.ttf`.
- **Upstream:** <https://github.com/be5invis/Iosevka>
- **License:** SIL Open Font License 1.1.
- **Loaded by:** `style.rs::install_fonts` via `include_bytes!`.

## Inherited from `frostcore`

`style.rs`, `icons.rs`, and the `ribbon/` tree were copied verbatim
from `frostcore` as the starting point for the `corekit`
migration (PLAN_NEWUI.md). They will diverge as we iterate.
