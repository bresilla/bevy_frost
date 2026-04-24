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

.PHONY: build b compile c run r test t check fmt bench clean help h

build:
	@$(CARGO) build -p bevy_frost --example $(EXAMPLE)

b: build

compile:
	@$(CARGO) clean
	@$(MAKE) build

c: compile

run:
	@DISPLAY=$(DISPLAY) $(RUN_WITH) $(CARGO) run --release -p bevy_frost --example $(EXAMPLE)

# Plain-egui (no Bevy) demo — `eframe` with the `wgpu` backend,
# same Vulkan path Bevy uses. Runs under the `nixVulkan` wrapper
# out of the box on nix systems; override with `RUN_WITH=` on
# distros with a native Vulkan driver.
run-egui:
	@DISPLAY=$(DISPLAY) $(RUN_WITH) $(CARGO) run --release -p egui_frost --example simple

r: run

test:
	@$(CARGO) test

t: test

check:
	@$(CARGO) check --all-targets

fmt:
	@$(CARGO) fmt --all

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
	@echo "  test         Run the test suite"
	@echo "  check        Run cargo check on all targets (lib + examples)"
	@echo "  fmt          Format the crate"
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
