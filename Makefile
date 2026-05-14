SHELL := /bin/bash

PROJECT_NAME := $(shell sed -n '/^[[:space:]]*[^#\[[:space:]]/p' PROJECT | head -1 | tr -d '[:space:]')
PROJECT_VERSION := $(shell sed -n '/^[[:space:]]*[^#\[[:space:]]/p' PROJECT | sed -n '2p' | tr -d '[:space:]')
ifeq ($(PROJECT_NAME),)
    $(error Error: PROJECT file not found or invalid)
endif

TOP_DIR := $(CURDIR)
CARGO := cargo
# DISPLAY pins which X server receives the window (matches the Nvidia GL
# display when running inside WSL / multi-X setups). Override if you need
# `:0` or similar: `make run DISPLAY=:0`.
DISPLAY ?= :1
# Wrapper that forwards GPU/display access. `nixVulkan` = Bevy/wgpu path.
# Override with `make run RUN_WITH=nixGL` or `RUN_WITH=` for native.
RUN_WITH ?= nixVulkan
# Example binary that `make run` targets. Override with `EXAMPLE=other`.
EXAMPLE ?= demo

$(info ------------------------------------------)
$(info Project: $(PROJECT_NAME) v$(PROJECT_VERSION))
$(info ------------------------------------------)

.PHONY: build b compile c run r smoke-gui test t check fmt harden harden-gui bench clean help h

build:
	@$(CARGO) build -p bevy_frost --example $(EXAMPLE)

b: build

compile:
	@$(CARGO) clean
	@$(MAKE) build

c: compile

run:
	@DISPLAY=$(DISPLAY) $(RUN_WITH) $(CARGO) run -p bevy_frost --example $(EXAMPLE)

smoke-gui:
	@set -euo pipefail; \
	log="$${TMPDIR:-/tmp}/bevy_frost_gui_smoke.log"; \
	rm -f "$$log"; \
	( DISPLAY=$(DISPLAY) $(RUN_WITH) $(CARGO) run -p bevy_frost --example $(EXAMPLE) >"$$log" 2>&1 ) & \
	pid=$$!; \
	trap 'kill $$pid >/dev/null 2>&1 || true; wait $$pid >/dev/null 2>&1 || true' EXIT; \
	deadline=$$((SECONDS + 20)); \
	found=""; \
	while [ $$SECONDS -lt $$deadline ]; do \
		if DISPLAY=$(DISPLAY) xwininfo -root -tree 2>/dev/null | grep -F "bevy_frost — $(EXAMPLE)" >/dev/null; then \
			found=1; \
			break; \
		fi; \
		if ! kill -0 $$pid >/dev/null 2>&1; then \
			break; \
		fi; \
		sleep 1; \
	done; \
	if [ -z "$$found" ]; then \
		echo "GUI smoke failed: window not found for example $(EXAMPLE)"; \
		cat "$$log"; \
		exit 1; \
	fi; \
	sleep 5; \
	if ! kill -0 $$pid >/dev/null 2>&1; then \
		echo "GUI smoke failed: example exited before stability window"; \
		cat "$$log"; \
		exit 1; \
	fi; \
	if grep -E "panicked at|Encountered a panic|\\bERROR\\b|\\bWARN\\b|Failed to find replacement characters" "$$log" >/dev/null; then \
		echo "GUI smoke failed: fatal log output detected"; \
		cat "$$log"; \
		exit 1; \
	fi; \
	echo "GUI smoke passed: bevy_frost — $(EXAMPLE) window appeared and stayed alive"; \
	cat "$$log"

# Phase 2 of `PLAN_NEWUI.md` — flex-based pane2 example. Empty
# panes (title strip + empty body) at all 12 anchor positions plus
# theme/mode cycle buttons. Doesn't touch the existing demo.
run-newui:
	@DISPLAY=$(DISPLAY) $(RUN_WITH) $(CARGO) run -p bevy_frost --example newui

# Plain-egui (no Bevy) demo — `eframe` with the `wgpu` backend,
# same Vulkan path Bevy uses. Runs under the `nixVulkan` wrapper
# out of the box on nix systems; override with `RUN_WITH=` on
# distros with a native Vulkan driver.
run-egui:
	@DISPLAY=$(DISPLAY) $(RUN_WITH) $(CARGO) run -p egui_frost --example egui_demo

r: run

test:
	@$(CARGO) test --all-targets

t: test

check:
	@$(CARGO) check --all-targets

fmt:
	@$(CARGO) fmt --all

harden:
	@git diff --check
	@$(CARGO) fmt --all -- --check
	@$(CARGO) check --workspace --no-default-features
	@$(CARGO) test --workspace --no-default-features
	@$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings
	@$(CARGO) test --workspace --all-targets --all-features

harden-gui: harden smoke-gui

bench:
	@$(CARGO) bench

clean:
	@$(CARGO) clean

help:
	@echo
	@echo "Usage: make [target]"
	@echo
	@echo "Available targets:"
	@echo "  build        Build the $(EXAMPLE) example"
	@echo "  compile      Clean and rebuild"
	@echo "  run          Run the example: DISPLAY=$(DISPLAY) $(RUN_WITH) cargo run --example $(EXAMPLE)"
	@echo "  smoke-gui    Run the example briefly and verify its X11 window appears"
	@echo "  test         Run the all-target test suite (libs + examples + doctests)"
	@echo "  check        Run cargo check on all targets (lib + examples)"
	@echo "  fmt          Format the crate"
	@echo "  harden       Run diff whitespace check + fmt/check + strict clippy + all-feature tests"
	@echo "  harden-gui   Run harden, then GUI smoke (requires X display)"
	@echo "  bench        Run benchmarks"
	@echo "  clean        Remove Cargo build artifacts"
	@echo
	@echo "Examples:"
	@echo "  make run"
	@echo "  make run EXAMPLE=other        # run a different example"
	@echo "  make run DISPLAY=:0           # target a different X server"
	@echo "  make run RUN_WITH=nixGL       # OpenGL wrapper instead of Vulkan"
	@echo "  make run RUN_WITH=            # no wrapper (native run)"
	@echo

h: help
