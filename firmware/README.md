# Firmware plan and toolchain baseline

The frozen firmware requirements are [FW-001 and FW-002](../docs/requirements.md), and the transport/data contract is [`docs/protocol.md`](../docs/protocol.md).

## Primary toolchain

Issue #1 proved a pinned stable Rust/ESP32-C3 path in [`toolchain-proof`](toolchain-proof):

```sh
./scripts/verify.sh
```

That command runs documentation/schema checks, formatting, host clippy/tests, target clippy, and an ESP32-C3 release link. See [`docs/toolchain.md`](../docs/toolchain.md) for exact versions, evidence limits, and the bounded ESP-IDF fallback gate.

## Responsibilities

- Initialize and supervise environmental sensor, load-cell ADC, button/status, storage, USB, and Wi-Fi adapters.
- Sample at bounded rates; separate raw diagnostics from filtered/stable observations.
- Track freshness, settling, saturation, disconnect, calibration, and storage faults explicitly.
- Compute gross and user-derived net mass with honest precision and uncertainty metadata.
- Maintain bounded versioned configuration/history with migration and interrupted-write recovery.
- Serve the companion bundle and versioned local HTTP/SSE API; support the required USB CDC offline subset.

Domain logic stays in host-testable `no_std` code behind sensor, monotonic/wall clock, storage, USB, and network interfaces. Exact hardware drivers wait for issue #2 part/pin decisions.

## Evidence boundary

The current proof builds and host-tests but has not been flashed or run on a board. Issue #4 must add pinned flash/recovery commands, serial evidence, full host tests, target size reporting, and selected-part drivers. Issue #6 owns bench fault/calibration/current evidence.
