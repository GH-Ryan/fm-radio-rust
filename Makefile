# Build entry points for the Tauri app.
#
# The frontend is embedded into the binary at compile time, so it has to be
# built first -- a stale dist/ silently ships the previous UI.
#
# --features custom-protocol is not optional either. Tauri decides between the
# bundled assets and devUrl from that feature (tauri-macros: `dev:
# cfg!(not(feature = "custom-protocol"))`), not from the profile, so a plain
# `cargo build --release` produces a binary that tries to reach
# http://localhost:1420 and dies with "Could not connect to localhost".
#
# These are thin wrappers over cargo and npm on purpose. `tauri build` and
# `tauri dev` would also work but need the CLI installed (cargo-tauri, or
# @tauri-apps/cli as a devDependency) and the CLI's own bundling step is a
# second thing that can drift from Cargo.toml. Driving cargo directly keeps one
# source of truth for the dependency set.

FEATURE  := custom-protocol
BIN      := target/release/fm-radio

.PHONY: all frontend build dev-run test lint fmt clean size

all: build

frontend:
	cd frontend && npm run build

build: frontend
	cargo build --release --features $(FEATURE)
	@stat -c '%s %n' $(BIN) | awk '{printf "built %s (%.2f MB)\n", $$2, $$1/1048576}'

# Layout iteration. The release profile carries lto=fat and codegen-units=1,
# which turns a CSS-only change into a two minute rebuild; the dev profile
# recompiles in seconds and renders the same layout. Use this while iterating
# and build only for the artefact.
dev-run: frontend
	cargo run --features $(FEATURE)

test:
	cargo test --features $(FEATURE)

# The usual Rust pairing: rustfmt first, because a formatting-only diff is the
# one thing clippy will not tell you about.
lint:
	cargo fmt --check
	cargo clippy --features $(FEATURE) --all-targets -- -D warnings

fmt:
	cargo fmt

size: build
	@size -A $(BIN) | sort -k2 -nr | head -6

# Frontend output goes to the repo root dist/ (vite outDir ../dist), not to
# frontend/dist.
clean:
	cargo clean
	rm -rf dist
