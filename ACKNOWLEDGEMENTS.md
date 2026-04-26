# Acknowledgements

Parts of this project are derived from third-party open-source
crates, vendored into
[`crates/frostcore/src/features/`](crates/frostcore/src/features/)
so we can modify them without upstreaming. Full copies of each
upstream license live alongside the vendored sources.

---

## `crates/frostcore/src/features/snarl/`

Derived from **egui-snarl** v0.9.0 — a node-graph widget for
`egui`.

- Upstream: <https://github.com/zakarumych/egui-snarl>
- Author: [@zakarumych](https://github.com/zakarumych)
- License: MIT OR Apache-2.0
- License files (verbatim copies):
  - [`crates/frostcore/src/features/snarl/LICENSE-MIT`](crates/frostcore/src/features/snarl/LICENSE-MIT)
  - [`crates/frostcore/src/features/snarl/LICENSE-APACHE`](crates/frostcore/src/features/snarl/LICENSE-APACHE)

---

## `crates/frostcore/src/features/code_editor/`

Derived from **egui_code_editor** v0.2.21 — a syntax-highlighting
multi-line text editor for `egui`.

- Upstream: <https://github.com/p4ymak/egui_code_editor>
- Author: Roman Chumak
  ([@p4ymak](https://github.com/p4ymak))
- License: MIT
- License file (verbatim copy):
  - [`crates/frostcore/src/features/code_editor/LICENSE`](crates/frostcore/src/features/code_editor/LICENSE)

---

## `crates/frostcore/src/features/flex/`

Derived from **egui_flex** v0.6.0 — a flexbox-style layout
container for `egui`.

- Upstream: <https://github.com/lucasmerlin/hello_egui/tree/main/crates/egui_flex>
- Author: Lucas Meurer
  ([@lucasmerlin](https://github.com/lucasmerlin))
- License: MIT
- License file (verbatim copy):
  - [`crates/frostcore/src/features/flex/LICENSE`](crates/frostcore/src/features/flex/LICENSE)

---

## Why vendored instead of depending directly

All three crates are excellent upstream, but we expect to modify
them heavily for project-specific needs (per-node colour, custom
syntax rules, editor behaviour changes, layout primitives that
plug directly into the frost theme tokens, …) and have no
intention of upstreaming those changes. Vendoring lets us iterate
without forks / PR roundtrips and keeps every dependency visible
in this repo's source tree.

If you contribute a change to the vendored code that could
benefit upstream too, please send it to the original repo first
— see the links above.
