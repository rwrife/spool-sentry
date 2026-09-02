# Spool Sentry implementation plan

## Frozen scope

Build a one-spool, observation-only passive dry-box monitor with editable low-voltage hardware, embedded firmware, and a device-hosted accessible companion. It measures environment and platform mass, stores bounded local history, supports explicit calibration/spool metadata, and provides user-owned export/backup/deletion. It never actuates a dryer or printer.

The authoritative v0.1 planning set is:

- [frozen requirements](docs/requirements.md)
- [editable architecture diagrams](docs/architecture.md)
- [protocol v0.1](docs/protocol.md)
- [risk register](docs/risk-register.md)
- [verification matrix](docs/verification-matrix.md)
- [firmware toolchain decision](docs/toolchain.md)

## Milestones and dependencies

1. **M1 — requirements/architecture:** freeze measurable ranges, semantics, retention, offline behavior, protocol, risk controls, and toolchain proof. No dependency.
2. **M2 — datasheet-backed parts/schematic:** exact manufacturer parts, KiCad properties, pin/package/footprint checks, ERC, and exported `bom/bom.csv`. Depends on M1.
3. **M3 — PCB/mechanics:** antenna/analog/thermal-aware layout, force path, DRC/cross-domain analysis, and review-only fabrication preview. Depends on M2.
4. **M4 — firmware:** host-tested domain logic, selected-part adapters, local HTTP/SSE and USB CDC, storage, target build/flash/recovery. Depends on M1; hardware integration depends on M2.
5. **M5 — companion:** fixture-first accessible local UI, calibration/history/events, export/restore/deletion. Depends on M1 and coordinates with M4 fixtures.
6. **M6 — integration:** assembled bring-up, calibration/repeatability/fault evidence, documentation, and limitations. Depends on M2–M5.
7. **M7 — release:** inspected fabrication/source/BOM/binaries/licenses/archive from a tagged revision. Depends on M6.

## Verification policy

Static analysis, simulation, host tests, target builds, bench measurements, and field observations are separate evidence classes. A clean target build does not mean firmware booted; an unbuilt board is not tested; one bench prototype does not establish field durability. Every requirement remains planned until the [verification matrix](docs/verification-matrix.md) links adequate evidence.

From the repository root, run the currently available checks with:

```sh
./scripts/verify.sh
```

Future milestones must extend that command/CI rather than replacing evidence with prose.

## Packaging direction

A release eventually includes editable KiCad/mechanical source, schematic PDF, inspected Gerbers/drills/BOM/CPL, pinned firmware source/binaries/checksums, compressed companion assets, assembly/bring-up/recovery docs, and hardware/firmware/software/third-party notices. No app store, cloud account, telemetry, or internet service is required.
