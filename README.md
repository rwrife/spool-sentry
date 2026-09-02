# Spool Sentry

> USB-powered ESP32-C3 monitor for 3D-printing hobbyists to track dry-box humidity, temperature, and filament spool mass through a private local dashboard without controlling heaters or printers.

## Overview

Spool Sentry is a small, repairable monitor for a passive filament dry box. A load cell under one spool records mass trends while a digital temperature/humidity sensor records the box environment. An ESP32-C3-class controller keeps bounded local history and serves a phone/desktop-friendly web interface on the local network. USB CDC remains the setup and recovery path.

The goal is practical awareness: see whether a box is staying dry, record desiccant changes, and estimate remaining filament after the user enters empty-spool mass. It is **not** a dryer controller, printer integration, certified hygrometer, or safety device.

## Motivation

Opaque boxes hide both environmental drift and dwindling material. Stick-on hygrometers show only the present, and manually weighing a spool interrupts storage. Spool Sentry combines two explainable observations without requiring a vendor account, subscription, or internet service.

## Target users

- Home 3D-printing hobbyists who store moisture-sensitive filament
- Small workshops that want a local history for one frequently used spool
- Makers who prefer editable hardware, replaceable sensors, and portable data

## Concrete use cases

- Check current box humidity and temperature from a phone on the same LAN.
- View hourly trends and annotate a desiccant recharge or spool swap.
- Tare/calibrate the scale, enter an empty-spool mass, and see gross/net mass with an uncertainty indicator.
- Export measurements and events as CSV/JSON before clearing local history.
- Recover configuration over USB if Wi-Fi credentials change.

## Intended workflow

1. Assemble the low-voltage carrier, sensor lead, load-cell platform, and enclosure using the bring-up guide that the backlog will create.
2. Power it from a suitable USB 5 V supply; no battery or mains wiring is part of the project.
3. Hold the physical setup button to open a short-lived onboarding session, then configure local Wi-Fi from the device-hosted page or USB CDC.
4. Calibrate with a documented reference mass, enter the spool's empty mass, and place the spool on the platform.
5. Review live values and local trends, add spool/desiccant events, and export or delete data at any time.

## MVP features

- One replaceable load-cell/ADC channel and one digital humidity/temperature channel
- Explicit sensor freshness, calibration state, and invalid/out-of-range statuses
- Gross mass, user-derived net filament estimate, and visible uncertainty—not false precision
- Bounded flash history with retention controls and event annotations
- Responsive local web UI with keyboard, screen-reader, reduced-motion, high-contrast, and non-color-only states
- Versioned CSV/JSON export and JSON backup/restore
- USB CDC provisioning, diagnostics, firmware recovery, and Wi-Fi-free snapshot access
- Physical setup/tare button and status indicator

## Non-goals

- Heater, fan, mains, battery-charging, relay, or printer control
- Fire, smoke, flood, food, medical, or life-safety monitoring
- Certified environmental measurement or guaranteed filament condition/print quality
- Automatic material-type diagnosis, purchasing, cloud accounts, remote internet access, or multi-site fleet management
- Multi-spool inventory, RFID, camera, or slicing software integration in the MVP

## Privacy, permissions, and data storage

Spool Sentry has no cloud backend, telemetry, advertising, or required account. Measurements, calibration data, annotations, and network configuration remain on the device; exports are created only when requested. The local web app requests no phone contacts, location, camera, microphone, Bluetooth, or notification permission. Wi-Fi access is limited to local setup and local device use. USB works without network access.

The baseline LAN protocol is not a promise of hostile-network security: deployment assumes a trusted home/workshop LAN, uses proof-of-presence onboarding, rejects cross-origin mutation, and keeps remote access disabled. Documentation will explain credential reset, export redaction, retention, deletion, and the limitations of unencrypted local HTTP if that remains the chosen transport.

## Safety limits

- SELV USB 5 V input only; use a suitable enclosed/certified USB supply.
- No connection to printer power, heaters, mains, batteries, or safety interlocks.
- Keep electronics away from hot zones, moving printer parts, liquids, and loose filament paths.
- Load readings are informational. Do not use the platform to support people, hazardous loads, or critical processes.
- Humidity/temperature readings and remaining-filament estimates can drift or fail. The device must surface stale/invalid data rather than imply safety or guaranteed print success.

## Hardware and source-of-truth plan

The planned editable KiCad project is:

- `hardware/spool-sentry.kicad_pro`
- `hardware/spool-sentry.kicad_sch`
- `hardware/spool-sentry.kicad_pcb`

These files **do not exist yet**; this repository is currently a documentation/backlog scaffold. The backlog requires real editable KiCad sources, ERC/DRC evidence, firmware/app builds, bring-up measurements, and fabrication outputs before any hardware-complete claim.

Final BOM data belongs in KiCad schematic symbol properties (`Manufacturer`, `MPN`, supplier fields, and notes) and is exported to tracked `bom/bom.csv`. `bom/preliminary-bom.csv` is planning-only and contains no validated sourcing or prices.

## Current status and milestones

**Status: documentation/backlog only.** There is no schematic, PCB, firmware, app build, calibrated instrument, fabricated board, assembly, ERC/DRC result, or bench/field test yet.

1. Freeze measurable requirements and risk boundaries.
2. Select exact components from manufacturer datasheets and create the KiCad schematic/BOM.
3. Lay out and verify the carrier PCB.
4. Build firmware and the local web companion against simulated interfaces.
5. Assemble, calibrate, integrate, and publish honest fabrication/release evidence.

See [PLAN.md](PLAN.md), [hardware/requirements.md](hardware/requirements.md), and the GitHub issue backlog.

## Development quickstart

The implementation workspace has not been created yet. Planned prerequisites are:

- KiCad 9+ for editable hardware sources and ERC/DRC
- Rust stable with an ESP32-C3-supported embedded toolchain for firmware
- Node.js 22 LTS with pnpm for the TypeScript/Vite local web app
- Python 3 for host-side protocol fixtures and hardware-free integration tests

For now, validate the scaffold by reviewing the documents and planning BOM. Do not expect build commands until the project-skeleton issue lands. Future quickstart commands must be pinned in-repository and exercised in CI before being presented as working.

## License

Initial documentation is available under the MIT License. A mature release will add explicit hardware/firmware/software license files and third-party notices before publishing fabrication outputs.
