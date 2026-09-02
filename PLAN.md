# Spool Sentry implementation plan

## Scope

Build a one-spool, observation-only dry-box monitor with reproducible low-voltage hardware, embedded firmware, and a device-hosted local web companion. The MVP measures box temperature/humidity and platform mass, stores bounded history, supports explicit calibration and spool metadata, and exports user-owned data. It never actuates a dryer or printer.

## Architecture

```text
USB 5 V
  -> input protection + 3.3 V rail
      -> ESP32-C3-class module
          -> digital temperature/humidity sensor
          -> load-cell ADC -> replaceable load cell/platform
          -> setup/tare button + non-color-only status LED
          -> USB CDC provisioning/recovery
          -> local Wi-Fi HTTP/WebSocket API
              -> device-hosted TypeScript/Vite responsive web app
          -> bounded flash config/history
```

Boundaries:

- **Hardware:** protected USB input, controller module, sensor connectors, button/status, programming/debug access, test points, and mechanical mounting.
- **Firmware:** sampling, validity/freshness, filtering, calibration, retention, protocol, provisioning, recovery, and static web asset serving.
- **Web app:** setup, live readings, calibration workflow, spool metadata, history/events, export/backup/restore, retention/deletion, accessibility.
- **Protocol:** versioned schemas shared by firmware, web app, fixtures, and host-side tests.

## Technology choices

- **ESP32-C3-class module:** inexpensive Wi-Fi/BLE-capable RISC-V platform with USB-capable development paths and broad open tooling. BLE is not required for MVP.
- **Digital humidity/temperature sensor family:** avoids analog calibration complexity; exact part waits for datasheet, package, lifecycle, and availability validation.
- **Bridge load cell plus dedicated ADC family:** common replaceable mechanics and sufficient resolution for trend/remaining-mass estimates; exact ADC and cell are not selected yet.
- **Rust embedded firmware:** memory-safe core logic and testable domain code; toolchain support must be proven in the skeleton issue before commitment is irreversible.
- **TypeScript + Vite PWA-style UI:** small device-hosted static bundle, responsive on Android/iOS/desktop browsers, no app-store dependency.
- **KiCad 9+:** editable schematic/PCB sources, symbol-property BOM, ERC/DRC, and reproducible fabrication export.

## Milestones and dependency order

### M1 — Requirements and architecture

Freeze measurement ranges, accuracy/uncertainty language, retention, mechanical load envelope, power limits, protocol boundary, and safety/risk exclusions. Dependency: none.

### M2 — Datasheet-backed parts and schematic

Select exact controller module, sensor, ADC, protection, connectors, and passives from manufacturer evidence. Populate KiCad Manufacturer/MPN properties, complete the schematic, run ERC, and export `bom/bom.csv`. Dependency: M1.

### M3 — PCB and mechanical interface

Define outline and mounting, preserve antenna/sensor/load-cell noise constraints, place/rout/test, run DRC/analyzers, and review fabrication files. Dependency: M2.

### M4 — Firmware and protocol core

Create reproducible build/flash/recovery; implement simulated sensor interfaces, filtering, calibration state, retention, and versioned API. Dependency: M1; hardware integration depends on M2.

### M5 — Local web companion

Implement onboarding, live/status, calibration, history/events, export/restore, deletion, and accessibility against protocol fixtures before hardware. Dependency: M1/M4 protocol fixtures.

### M6 — Integration and release candidate

Assemble, record expected measurements, calibrate, test fault states and offline recovery, document assembly/troubleshooting, and publish inspected fabrication/release outputs. Dependencies: M2–M5.

## Testing strategy

- **Static hardware checks:** KiCad ERC/DRC, schematic/PCB analyzers, pad/net cross-checks, antenna keepout and sensor-placement review.
- **Datasheet verification:** voltage/current/pinout/package/lifecycle checks cited to manufacturer documents; no exact part is accepted from search snippets alone.
- **Firmware:** host unit tests for filtering, calibration arithmetic, stale/invalid states, retention, schema migration, and protocol serialization; target build and flash checks; hardware abstraction fakes.
- **Web app:** unit tests for reducers/formatting/import validation, accessibility checks, component tests, protocol fixtures, and production bundle size.
- **Integration:** replayable serial/API fixtures first; then bench measurements for rail voltage/current, sensor plausibility, calibration repeatability, drift, load-step response, disconnect/reconnect, Wi-Fi loss, storage-full behavior, and factory reset.
- **Evidence labels:** static analysis, simulation, bench testing, and field observation remain separate. An unbuilt prototype is never called tested.

## Packaging and distribution

- Versioned firmware binaries plus source/build metadata and checksums
- Device-hosted compressed web bundle built from source
- KiCad source, schematic PDF, Gerber/drill, BOM/CPL where applicable, board renders, and fabrication notes
- Versioned source/archive release with hardware, firmware, software, and third-party licenses
- No app store or cloud service required; optional installable web-app behavior remains browser-dependent

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Load-cell creep, off-center loading, vibration, or temperature drift | Stable-reading gate, calibration age/state, mechanical guidance, repeatability measurements, visible uncertainty |
| Humidity sensor self-heating or poor placement | Low duty cycle, vented placement away from regulator/MCU, datasheet timing, comparison during bring-up |
| Flash wear or history corruption | Bounded append strategy, checksums/versioning, retention, interrupted-write tests |
| Wi-Fi onboarding exposes credentials | Short-lived proof-of-presence mode, local-only endpoints, no logs/exports containing secrets, documented reset |
| ESP32-C3 Rust support blocks progress | Prove build/flash/CI in M1; retain a narrow C/C++ fallback decision gate without changing protocol/domain tests |
| Mass estimate mistaken for guaranteed usable filament | Label estimates and calibration assumptions; expose gross/net/uncertainty and never predict print success |
| Electronics placed near hot dryer hardware | Project explicitly supports passive boxes only; no heater integration or thermal-control claims |

## Explicit non-goals

Mains/battery power, heating/fan actuation, printer control, safety alarms, certified metrology, guaranteed material condition, multi-spool/RFID/camera inventory, cloud sync, remote access, AI classification, and commercial fleet management are outside the MVP.
