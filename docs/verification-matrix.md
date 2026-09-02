# Requirements verification matrix

Evidence states: **planned**, **pass**, **fail**, **blocked**, or **not applicable**. This baseline intentionally records mostly planned evidence. Target-build proof is not bench evidence.

| Requirement | Planned method | Acceptance evidence | Current state |
|---|---|---|---|
| PRD-001 | Static architecture + schematic/PCB inspection | One load-cell channel and one environmental channel; no inventory channel | planned |
| PRD-002 | Static schematic/PCB/protocol review | No actuator/power-control interface or command | planned |
| PRD-003 | Static code/network review + offline integration | Core flows work without account/cloud/internet; no telemetry | planned |
| PWR-001 | Datasheet review, ERC/DRC, bench | Protected SELV-only input and measured rails | planned |
| PWR-002 | Calculation + PCB current analysis + bench | Worst-case/inrush budget and ≥20% path margin | planned |
| PWR-003 | Host fixtures + bench fault injection | Every listed failure emits explicit invalid/fault state | planned |
| MEAS-001 | Datasheet review + environmental comparison | Range/status/age/accuracy metadata and recorded comparison | planned |
| MEAS-002 | Datasheet/mechanical review + bench loads | 0–2.5 kg centered operation with ≥5 kg rated cell | planned |
| MEAS-003 | Bench repeats | Raw trials from 100 g–2.0 kg meet greater-of limit | planned |
| MEAS-004 | UI/unit tests + bench-derived precision review | Display increment no finer than supporting evidence | planned |
| MEAS-005 | Host/UI tests | Formula, assumptions, uncertainty, and negative fault behavior | planned |
| MEAS-006 | Schema/migration/fault tests | Complete record and invalidation transitions | planned |
| MECH-001 | CAD/static review + progressive bench load | No PCB/enclosure contact at centered 2.5 kg | planned |
| MECH-002 | Assembly inspection | Listed items replaceable with ordinary tools | planned |
| MECH-003 | PCB/mechanical/thermal review + bench comparison | Documented spacing/airflow and heat-bias observations | planned |
| MECH-004 | Source/release inspection | Editable mechanics and bounded spool envelope | planned |
| DATA-001 | Capacity/wear calculation + host/target storage test | ≥30 days; measured 90-day default target behavior | planned |
| DATA-002 | Schema/privacy/round-trip tests | Valid versioned CSV/JSON with no prohibited identifiers | planned |
| DATA-003 | Host property/fault tests | Validate/preview/commit and interruption preserve prior state | planned |
| DATA-004 | Host/UI/USB tests | Each scope deletes exactly requested data after confirmation | planned |
| DATA-005 | Host fault injection + target storage test | Corruption/full/interruption/retention are bounded and explicit | planned |
| CONN-001 | Protocol contract + target integration | Local HTTP JSON and SSE; no WebSocket/remote endpoint | planned |
| CONN-002 | Host timing/security + bench button tests | Presence session is random, recent, bounded, and expires | planned |
| CONN-003 | USB fixtures + target/bench integration | Required offline command subset works without Wi-Fi/internet | planned |
| CONN-004 | Static dependency/network/permission review | No prohibited network service or phone permission | planned |
| SEC-001 | Protocol fuzz/bounds tests + LAN inspection | Host/Origin rejection and enforced size/rate/time limits | planned |
| SEC-002 | Log/export/backup inspection | No credentials/stable hardware/network IDs; resettable ID | planned |
| PROTO-001 | Schema/contract compatibility tests | 0.1 payloads accepted; unsupported major rejected | planned |
| PROTO-002 | `scripts/check_docs.py` + UI fixtures | Observation schema/fixture valid; quality always explicit | pass (static schema fixture) |
| PROTO-003 | Host parser/import tests | LAN/USB semantics match; malformed input causes no mutation | planned |
| FW-001 | Clean host test + ESP32-C3 release build | Pinned toolchain, host tests, linked target image | pass (host/target build; see issue #1 PR) |
| FW-002 | Host unit/property tests with fakes | Domain behaviors run without physical hardware | planned (proof boundary exists) |
| UX-001 | Automated/manual accessibility + browser smoke | Keyboard/SR/focus/zoom/motion/contrast/table evidence | planned |
| UX-002 | Component/contract tests | Every status has text/icon/age and no silent stale reuse | planned |
| SAFE-001 | Documentation + environmental bench review | Passive-box target conditions documented and observed | planned |
| SAFE-002 | Docs/schematic/PCB review | Prohibitions visible and no prohibited interface | planned |
| SAFE-003 | Copy/UI/release review | No prohibited guarantee/certification language | planned |
| REL-001 | Release evidence audit | Every result labeled by evidence class; gaps explicit | planned |
| COST-001 | Dated BOM/fabrication quote | Included/excluded costs and ceiling evaluated without invention | planned |
| SRC-001 | Release/source/BOM audit | Editable sources and KiCad-property-exported tracked BOM | planned |

## Issue #1 evidence boundary

Completed here: requirement reconciliation, architecture diagrams, protocol decision/schema fixture, risk analysis, and a clean host/ESP32-C3 target build. Not performed: schematic/PCB analysis, simulation, flashing, bench measurement, calibration, environmental comparison, accessibility/browser tests, field testing, fabrication, sourcing, or cost validation.
