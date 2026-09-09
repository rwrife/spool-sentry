# Firmware domain core (`spool-sentry-domain`)

Host-testable, allocation-free (`no_std`) domain logic for the ESP32-C3
spool-sentry device (issue #4). All hardware interaction happens through
injected adapter traits, so every behavior below is exercised on the host
with fakes before any board exists.

## What lives here

| Module | Responsibility |
|---|---|
| `json` | Bounded parser/writer for the frozen command grammar (size/depth caps, strict number grammar, no exponent/NaN input) |
| `rpc` | Transport-neutral command router shared by LAN HTTP and USB CDC; identical semantics on both (PROTO-003); request rate budget |
| `faults` | Stable machine-readable fault codes matching `^[a-z0-9_]+$`; disconnect/saturation/out-of-range never merged |
| `fresh` | Freshness/staleness thresholds and classification |
| `sensors` | Reading envelopes (`Ok`/`Disconnected`/`Saturated`/`OutOfRange`/`ImpossibleValue`) and adapter traits |
| `filter` | Median-of-window mass filter with stability detection; resets on disconnect |
| `mass` | Gross/net arithmetic, user-entered empty-spool mass, negative-net fault |
| `calibration` | Two-point tare/reference lifecycle with single-use random tokens, expiry, and invalidation |
| `observation` | Deterministic observation JSON conforming to `docs/schemas/observation-v0.1.schema.json` |
| `csv` | CSV export rows with explicit units, time basis, quality, age, calibration state, uncertainty, faults |
| `store` | Dual-slot objects with FNV-1a checksums, interrupted-write recovery (staging/primary), capacity-exceeded behavior, retention-bounded event log, deletion scopes |
| `backup` | Validate → preview → commit backup envelope; credentials are structurally excluded |
| `presence` | Physical-presence sessions: button opens a nonce session; mutations require a recent session; expiry is automatic |
| `timefmt` | RFC 3339 UTC formatting/parsing without `alloc`; wall-clock changes never mutate stored history |
| `core` | `DeviceCore::observe` sampling pipeline tying it all together |

## Reproduce

From the repository root: `./scripts/verify.sh` runs both crates. To work
on this crate alone (rustup discovers the pin in this directory):

```sh
cd firmware/domain
cargo fmt --all -- --check
HOST="$(rustc -vV | sed -n 's/^host: //p')"
cargo clippy --lib --tests --target "$HOST" --locked -- -D warnings
cargo test --target "$HOST" --locked          # host: 74 unit + 3 fixture-contract tests
cargo clippy --release --locked -- -D warnings # ESP32-C3 no_std clippy
cargo build --release --locked                 # links the ESP32-C3 image
size -A target/riscv32imc-unknown-none-elf/release/spool-sentry-domain
```

The host test profile (`cfg(test)`) may use `std`/`alloc`; the firmware
profile is `no_std`, allocation-free, and depends only on `heapless`.
`tests/observation_fixtures.rs` validates the emitted bytes against the
real repository schema and fixture files, so firmware and companion-app
data cannot drift apart.

## Flashing, serial, erase/recovery (pinned tools)

Pinned host tool: `espflash 4.5.0` (installed with
`cargo install espflash --locked --version 4.5.0`). `.cargo/config.toml`
wires the runner so `cargo run --release` flashes and monitors. The exact
USB topology (native USB-Serial-JTAG vs. UART bridge) is finalized in
issue #6 bring-up; commands below are the selected toolchain's documented
interfaces.

```sh
# Flash the release image and open the serial monitor
cargo run --release                       # = espflash flash --monitor --chip esp32c3 <image>
espflash flash --monitor --chip esp32c3 \
  target/riscv32imc-unknown-none-elf/release/spool-sentry-domain

# Serial log (monitor without flashing)
espflash monitor --chip esp32c3

# Factory reset: erase all device data (configuration, history, calibration)
espflash erase-flash --chip esp32c3

# USB recovery: hold BOOT (IO9) while resetting to enter the ROM bootloader,
# then re-flash over USB; espflash auto-detects the bootloader:
espflash flash --chip esp32c3 target/riscv32imc-unknown-none-elf/release/spool-sentry-domain
```

The current bin (`src/bin/main.rs`) is a tool/link-path proof for the
domain core on the real target; driver integration (NAU7802 ADC, SHT40,
flash, USB CDC, Wi-Fi/HTTP server) lands with issue #6 bring-up.

## Evidence boundary (honesty rule)

- Host tests prove domain behavior against simulated hardware only.
- The release build proves the domain links for `riscv32imc-unknown-none-elf`;
  the linked ELF size is reported by CI.
- Nothing here has been flashed, booted, measured, or bench-tested. No
  on-device timing, accuracy, connectivity, or battery claims are made.
- Uncertainty is reported as `null` with the `uncertainty_not_characterized`
  fault until issue #6 produces calibrated bench evidence.
