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

The pinned toolchain automatically installs `rust-src` and the ESP32-C3 target through rustup. Flashing is deliberately not part of this proof; issue #4 must pin and document a compatible flashing tool after issue #2 chooses the exact controller module and USB topology.

## Architecture rule

Domain logic shall remain in a host-testable `no_std` library. Hardware-specific sensor, clock, storage, USB, and network adapters may depend on ESP crates. Protocol fixtures are shared with the companion app; neither target adapters nor a fallback may redefine protocol semantics.

## Bounded fallback gate

Rust remains the default unless issue #4 records a reproducible blocker in the exact selected module path after a time-boxed integration attempt. A fallback may be invoked only if one of these blocks the required target/CI evidence:

1. selected-module Wi-Fi/USB support cannot build on the pinned stable toolchain;
2. a required maintained driver cannot satisfy the selected part's documented interface;
3. a toolchain defect prevents a clean target image and has no bounded patch/upstream release.

If triggered, use a pinned Espressif ESP-IDF C/C++ toolchain only for target adapters and orchestration. Preserve the versioned protocol schemas/fixtures, fault semantics, storage/export behavior, and equivalent host tests. Record the blocker, versions, minimal reproducer, decision date, and migration cost in an architecture decision record before changing implementation language. The fallback does not authorize cloud, actuation, weaker validation, or unsupported hardware claims.
