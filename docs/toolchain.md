# Firmware toolchain decision and proof

## Decision

The v0.1 primary firmware path is bare-metal stable Rust for ESP32-C3:

- Rust `1.98.0` pinned by `firmware/toolchain-proof/rust-toolchain.toml`
- target `riscv32imc-unknown-none-elf`
- `esp-hal` `1.1.2` with its `esp32c3` feature
- `esp-bootloader-esp-idf` `0.5.0`
- edition 2024 and committed `Cargo.lock`

This target is supported by upstream stable Rust; no custom Xtensa compiler is required for ESP32-C3. The proof initializes the ESP32-C3 HAL and links an ESP-IDF-compatible image. Its `no_std` library exercises the planned host-test boundary with quality-state logic.

This is **target-build evidence only**. No board was connected, flashed, booted, or measured.

## Reproduce

From the repository root:

```sh
./scripts/verify.sh
```

Or run the stages directly (the working directory is significant because rustup discovers the pin there):

```sh
python3 scripts/check_docs.py
cd firmware/toolchain-proof
cargo fmt --all -- --check
HOST="$(rustc -vV | sed -n 's/^host: //p')"
cargo clippy --lib --target "$HOST" --locked -- -D warnings
cargo test --lib --target "$HOST" --locked
cargo clippy --release --locked -- -D warnings
cargo build --release --locked
```

Issue #4 added the host-testable domain crate at `firmware/domain`
(see its [README](../firmware/domain/README.md) for module map, exact
test/build/flash/erase/recovery commands, and evidence limits). The
pinned tool automatically installs `rust-src` and the ESP32-C3 target
through rustup.

Flashing is pinned to `espflash 4.5.0`
(`cargo install espflash --locked --version 4.5.0`), wired as the cargo
runner in each crate's `.cargo/config.toml`. The exact USB topology
(native USB-Serial-JTAG vs. UART bridge) is finalized in issue #6
bring-up; no board has been flashed in this repository to date.

## Architecture rule

Domain logic shall remain in a host-testable `no_std` library. Hardware-specific sensor, clock, storage, USB, and network adapters may depend on ESP crates. Protocol fixtures are shared with the companion app; neither target adapters nor a fallback may redefine protocol semantics.

## Companion toolchain (issue #5)

- Node.js `22.23.1` / npm `10.9.8` pinned in CI via `actions/setup-node` (SHA-pinned v5); local development may use any Node 22.x, but CI is authoritative.
- `app/package.json` + committed `app/package-lock.json` pin every dev dependency (TypeScript 5.6.3, Vite 5.4.11, Vitest 2.1.8, Ajv 8.17.1 + ajv-formats 3.0.1 for schema contract tests, jsdom 25.0.1, Prettier 3.3.3).
- Build is `tsc --noEmit` followed by `vite build`; tests run under Vitest on Node.
- `./scripts/verify.sh` runs the companion gates (lint, tests, build, loopback serve smoke) after the firmware stages and skips them with an explicit message when npm is missing.
- The bundle is dependency-free vanilla TypeScript so the device can host `app/dist/` verbatim; the smoke script enforces a raw size budget (<50 kB JS, <10 kB CSS).

Smoke evidence to date is Linux/node + jsdom only. The browser-matrix claims (Android Chrome, iOS Safari, Windows Edge, macOS Safari) remain not-performed until real devices are exercised in issue #6.

## Bounded fallback gate

Rust remains the default unless issue #4 records a reproducible blocker in the exact selected module path after a time-boxed integration attempt. A fallback may be invoked only if one of these blocks the required target/CI evidence:

1. selected-module Wi-Fi/USB support cannot build on the pinned stable toolchain;
2. a required maintained driver cannot satisfy the selected part's documented interface;
3. a toolchain defect prevents a clean target image and has no bounded patch/upstream release.

If triggered, use a pinned Espressif ESP-IDF C/C++ toolchain only for target adapters and orchestration. Preserve the versioned protocol schemas/fixtures, fault semantics, storage/export behavior, and equivalent host tests. Record the blocker, versions, minimal reproducer, decision date, and migration cost in an architecture decision record before changing implementation language. The fallback does not authorize cloud, actuation, weaker validation, or unsupported hardware claims.

## Hardware verification toolchain (issue #3)

- `kicad-cli` on this machine is a Docker wrapper for the pinned image
  `parts-tally-kicad:9-arm64` (KiCad 9.0.9); all pcbnew-API scripts run in
  that image via `docker run --user $(id -u):$(id -g)` (see
  `scripts/*.py` usage strings).
- Autorouter: FreeRouter `2.1.0` headless with Temurin JDK 21
  (class-file 65; the JDK 17 copy fails `UnsupportedClassVersionError`).
  Incremental passes use `-hf true` (fanout mode converges to natural exit
  and writes the SES session; full-mode plateau never flushes).
- Mechanics: OpenSCAD `2021.01` via Docker image
  `parts-tally-openscad:arm64` (`scripts/export_mechanical_stl.sh`);
  STL outputs are review-only previews, not fabricated parts.
- BOM tooling unchanged (issue #2); no new schematic inputs were added in
  issue #3, so `bom/bom.csv` is re-exported from the same properties.
