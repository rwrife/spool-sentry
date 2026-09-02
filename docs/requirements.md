# Spool Sentry v0.1 requirements

**Status:** frozen planning baseline for v0.1. Values below are design targets until the verification matrix records matching evidence. This document is the canonical requirement set; subsystem documents link back to these stable IDs.

## Product boundary

| ID | Requirement |
|---|---|
| PRD-001 | The MVP observes exactly one removable spool using one load-cell channel and one dry-box environmental channel. Multi-spool inventory is out of scope. |
| PRD-002 | The device is observation-only and shall expose no heater, fan, relay, printer-power, printer-control, battery, or charger interface. |
| PRD-003 | Core use shall be local/offline-first with no account, cloud backend, telemetry, advertising, or required internet service. |

## Power and electrical behavior

| ID | Requirement |
|---|---|
| PWR-001 | Accept nominal USB 5 V SELV input only from a suitable enclosed supply; exact connector protection and acceptable input tolerance require datasheet verification in issue #2. |
| PWR-002 | The exact-parts design shall document worst-case load and inrush and retain at least 20% continuous-current margin in the supply, protection, regulator, connector, and PCB path. |
| PWR-003 | Power-up, sensor disconnect, ADC saturation, invalid calibration, and storage faults shall become explicit states; no missing or failed input may be represented as a valid zero. |

## Measurement semantics and targets

| ID | Requirement |
|---|---|
| MEAS-001 | Report temperature and relative humidity with sample age, status, and the selected sensor's documented accuracy limits over the product target of 10–35 °C and 10–80% RH, non-condensing. |
| MEAS-002 | Support a centered 0–2.5 kg gross platform/spool operating range; select a load cell with rated capacity at least 5 kg and verify the complete mechanical overload path. |
| MEAS-003 | After warm-up and valid calibration, target repeatability within the greater of 5 g or 1% of applied mass for centered references from 100 g through 2.0 kg. This is a bench target, not a current claim. |
| MEAS-004 | Display resolution shall not imply more precision than calibration and repeatability evidence supports; raw ADC counts remain diagnostic data. |
| MEAS-005 | Net filament estimate equals stable gross mass minus the user-entered empty-spool mass. It shall include units, calibration state, freshness, and uncertainty; negative results remain visible as invalid/configuration faults rather than being silently clamped. |
| MEAS-006 | Calibration records shall include schema version, timestamp or explicit unknown-time state, reference mass, scale factor, zero offset, uncertainty metadata, and invalidation reason. Relevant sensor/configuration changes invalidate calibration. |

Terminology is normative:

- **Accuracy** is closeness to a traceable reference and is not claimed before bench comparison.
- **Repeatability** is agreement among repeated readings under stated conditions and uses MEAS-003.
- **Display resolution** is the smallest shown increment; it is not accuracy.
- **Uncertainty** is the stated interval/estimate derived from sensor, calibration, repeatability, drift, and quantization evidence. Until measured, the UI says **uncertainty not characterized**.

## Mechanical and environmental boundary

| ID | Requirement |
|---|---|
| MECH-001 | The platform shall support a centered 2.5 kg static working load without the moving load path touching the PCB; overload and off-center limits require bench evidence before publication. |
| MECH-002 | Load cell, environmental sensor, USB cable, and electronics carrier shall be replaceable with ordinary hand tools, with strain relief and unambiguous orientation. |
| MECH-003 | Place the vented environmental sensor away from MCU/regulator heat and filament contact; document distance, airflow assumptions, and bring-up comparison. |
| MECH-004 | Release editable mechanical source for the force path and common-spool fit envelope without claiming universal spool compatibility. |

## Data ownership, retention, and recovery

| ID | Requirement |
|---|---|
| DATA-001 | Store at least 30 complete days of hourly aggregates. The v0.1 default target is a rolling 90 days; final capacity and flash wear shall be measured before release. High-rate raw samples are not retained by default. |
| DATA-002 | User-triggered exports shall provide versioned CSV and JSON with explicit units, quality/fault fields, time-basis metadata, and no credentials or stable hardware/network identifiers. |
| DATA-003 | Backups shall be versioned and integrity checked. Restore is a validate → preview → explicit commit transaction; validation failure or interrupted commit preserves the prior committed state. |
| DATA-004 | Users shall be able to delete history, spool metadata, calibration, network credentials, or all user data by explicit scope. Destructive operations require fresh confirmation and report completion/failure. |
| DATA-005 | Configuration/history storage shall be bounded and versioned, detect corruption, recover from interrupted writes, expose storage-full behavior, and apply retention without silently discarding the newest unexported data. |

## Connectivity, privacy, and protocol

| ID | Requirement |
|---|---|
| CONN-001 | The v0.1 LAN transport is local HTTP/1.1 JSON plus one-way Server-Sent Events (SSE) for live observations. WebSocket and remote-internet access are not part of v0.1. |
| CONN-002 | Wi-Fi onboarding and sensitive mutation require a recent, short-lived physical-presence session initiated by the setup button and expiring automatically. |
| CONN-003 | Without Wi-Fi or internet, USB CDC shall provide version/capability status, current snapshot, diagnostics, provisioning/recovery, user-data export/backup, and confirmed scoped deletion/factory reset. The web UI itself need not be served over USB. |
| CONN-004 | The product shall not implement telemetry, cloud relay, automatic port mapping, internet discovery, remote administration, or baseline phone sensor permissions. |
| SEC-001 | Documentation shall disclose that unencrypted local HTTP assumes a trusted LAN. Implementations shall reject unexpected Host/Origin values and enforce bounded body sizes, parser depth, rates, and timeouts. |
| SEC-002 | Wi-Fi credentials are write-only after provisioning and shall not appear in normal logs, diagnostics, backups, exports, or support bundles. Device IDs are random and resettable rather than raw chip identifiers. |
| PROTO-001 | Protocol version `0.1` uses major-version rejection and additive-minor compatibility. Every response carries `protocol_version`; request/response operations carry bounded `request_id` values. |
| PROTO-002 | Observation payloads shall conform to `docs/schemas/observation-v0.1.schema.json`; value availability, freshness, calibration, uncertainty, and faults are explicit and non-color-only. |
| PROTO-003 | LAN and USB operations share semantic fixtures and error codes. Invalid, oversized, unsupported, or malformed input fails closed without mutation or loss of current committed data. |

## Firmware, UI, evidence, and source

| ID | Requirement |
|---|---|
| FW-001 | The primary firmware toolchain is pinned stable Rust targeting ESP32-C3 with `esp-hal`; a host-testable `no_std` domain core is separated from hardware adapters. `firmware/toolchain-proof` is the build proof. |
| FW-002 | Filters, calibration, retention, migration, protocol, and fault-state logic shall run in host tests through injected sensor, clock, storage, USB, and network boundaries. |
| UX-001 | Primary web flows shall support keyboard use, screen-reader semantics, visible focus, 200% text scaling, reduced motion, high contrast, and textual chart summaries/data tables. |
| UX-002 | Fresh, stale, invalid, uncalibrated, saturated, disconnected, and storage-fault states shall use text/icon semantics in addition to color and shall never reuse stale values without age/status. |
| SAFE-001 | Intended use is indoor hobby/workshop observation in a passive, non-condensing dry box within the target range in MEAS-001. |
| SAFE-002 | Documentation shall prohibit heated dryers/ovens, printer enclosures/power/control, mains, batteries/charging, wet locations, people/hazardous loads, and safety-critical use. |
| SAFE-003 | The product shall not claim certified metrology, certified humidity, filament condition/dryness, fire prevention, print success, or safety-alarm behavior. |
| REL-001 | Reports and UI documentation shall distinguish static analysis, simulation, host tests, target build, bench measurement, and field testing. Unperformed evidence stays explicit. |
| COST-001 | Target one-prototype cost is USD 35–60 with a USD 75 ceiling, excluding tools, shipping, tax, phone/computer/printer, and calibration reference; only dated live evidence may populate prices. |
| SRC-001 | Release editable KiCad/mechanical/firmware/app sources. Exact Manufacturer/MPN/supplier/BOM comments live in KiCad symbol properties and export to tracked `bom/bom.csv`; planning placeholders are not sourced parts. |
