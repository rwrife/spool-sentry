# Firmware plan

## Responsibilities

- Initialize and supervise the environmental sensor, load-cell ADC, button, status output, storage, USB, and Wi-Fi.
- Sample at bounded rates; expose raw diagnostic values separately from filtered/stable observations.
- Track freshness, settling, saturation, disconnect, calibration, and storage faults explicitly.
- Compute gross and user-derived net mass with honest precision and uncertainty metadata.
- Maintain bounded, versioned configuration/history with migration and interrupted-write recovery.
- Serve the companion web bundle and versioned local API; support USB CDC setup, snapshot, diagnostics, reset, and recovery.

## Interfaces and protocol

The canonical draft lives in `docs/protocol.md`. Firmware and app will share generated or validated schema fixtures rather than duplicate ad-hoc payload definitions. Mutation requires a short-lived session established by physical proof of presence. Internet connectivity is not required.

Hardware drivers will sit behind narrow interfaces so filters, calibration, retention, protocol serialization, and fault behavior can run on a host without physical hardware.

## Provisioning and updates

- First setup: hold the physical button, then use a short-lived local onboarding page or USB CDC.
- Credentials must never appear in normal logs or exports.
- USB flashing/recovery is mandatory and documented before any over-the-air update is considered.
- Updates must identify compatible hardware/protocol versions and preserve or explicitly migrate data.
- Failed update/recovery behavior must be tested; no claim of safe OTA exists in the scaffold.

## Test strategy

- Host unit/property tests: calibration arithmetic, stable-reading gate, stale/invalid transitions, bounded queues, migrations, import rejection, and protocol encoding.
- Simulated drivers: sensor disconnect, impossible values, ADC saturation/noise, button timing, Wi-Fi loss, storage-full/interrupted writes.
- Reproducible target build and size report in CI.
- On-device checks: flash/recovery, boot/current measurements, radio coexistence/noise, sampling timing, repeated reference loads, and environmental comparison.
- Label evidence as host simulation, target build, or bench measurement; an unbuilt board is not hardware-tested.
