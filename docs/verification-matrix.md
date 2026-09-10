# Requirements verification matrix

Evidence states: **planned**, **pass**, **fail**, **blocked**, or **not applicable**. This baseline intentionally records mostly planned evidence. Target-build proof is not bench evidence.

| Requirement | Planned method | Acceptance evidence | Current state |
|---|---|---|---|
| PRD-001 | Static architecture + schematic/PCB inspection | One load-cell channel and one environmental channel; no inventory channel | pass (schematic path implements one NAU7802 bridge channel + one SHT40 channel) |
| PRD-002 | Static schematic/PCB/protocol review | No actuator/power-control interface or command | pass (static schematic/protocol review; no actuator nets or control drivers) |
| PRD-003 | Static code/network review + offline integration | Core flows work without account/cloud/internet; no telemetry | planned |
| PWR-001 | Datasheet review, ERC/DRC, bench | Protected SELV-only input and measured rails | pass (static ERC + protection path present; bench rail measurements pending) |
| PWR-002 | Calculation + PCB current analysis + bench | Worst-case/inrush budget and ≥20% path margin | planned |
| PWR-003 | Host fixtures + bench fault injection | Every listed failure emits explicit invalid/fault state | planned |
| MEAS-001 | Datasheet review + environmental comparison | Range/status/age/accuracy metadata and recorded comparison | planned |
| MEAS-002 | Datasheet/mechanical review + bench loads | 0–2.5 kg centered operation with ≥5 kg rated cell | planned |
| MEAS-003 | Bench repeats | Raw trials from 100 g–2.0 kg meet greater-of limit | planned |
| MEAS-004 | UI/unit tests + bench-derived precision review | Display increment no finer than supporting evidence | planned |
| MEAS-005 | UI/unit tests + bench-derived precision review | Formula, assumptions, uncertainty, and negative fault behavior | pass (host: net=gross−tare with user-entered tare, null uncertainty + explicit fault; UI half now covered by `app/test/format.test.ts` host tests — no-zero nulls, uncertainty fault text; browser matrix deferred to #6) |
| MEAS-006 | Schema/migration/fault tests | Complete record and invalidation transitions | planned |
| MECH-001 | CAD/static review + progressive bench load | No PCB/enclosure contact at centered 2.5 kg | planned |
| MECH-002 | Assembly inspection | Listed items replaceable with ordinary tools | planned |
| MECH-003 | PCB/mechanical/thermal review + bench comparison | Documented spacing/airflow and heat-bias observations | planned |
| MECH-004 | Source/release inspection | Editable mechanics and bounded spool envelope | planned |
| DATA-001 | Capacity/wear calculation + host/target storage test | ≥30 days; measured 90-day default target behavior | planned |
| DATA-002 | Schema/privacy/round-trip tests | Valid versioned CSV/JSON with no prohibited identifiers | pass (host: backup envelope has no credential field and CSV rows carry version/time-basis/units/quality; target round-trip deferred to #6) |
| DATA-003 | Host property/fault tests | Validate/preview/commit and interruption preserve prior state | pass (host: validate→commit and interrupted staging fall back to committed copy) |
| DATA-004 | Host/UI/USB tests | Each scope deletes exactly requested data after confirmation | pass (host: confirmed deletion scopes erase exactly their keys; companion UI lists the exact protocol scopes, requires confirmation, and never fabricates the device token; USB/on-device halves deferred to #6) |
| DATA-005 | Host fault injection + target storage test | Corruption/full/interruption/retention are bounded and explicit | pass (host: checksum-corrupt read falls back, capacity-exceeded evicts oldest explicitly, retention window enforced; target storage deferred to #6) |
| CONN-001 | Protocol contract + target integration | Local HTTP JSON and SSE; no WebSocket/remote endpoint | planned |
| CONN-002 | Host timing/security + bench button tests | Presence session is random, recent, bounded, and expires | pass (host: nonce session, freshness window, auto-expiry; bench button timing deferred to #6) |
| CONN-003 | USB fixtures + target/bench integration | Required offline command subset works without Wi-Fi/internet | planned |
| CONN-004 | Static dependency/network/permission review | No prohibited network service or phone permission | planned |
| SEC-001 | Protocol fuzz/bounds tests + LAN inspection | Host/Origin rejection and enforced size/rate/time limits | pass (host: size/depth caps, rate budget, malformed-input non-mutation enforced by the shared router; Host/Origin binding lands with the LAN adapter in #6) |
| SEC-002 | Log/export/backup inspection | No credentials/stable hardware/network IDs; resettable ID | pass (host: backup structurally excludes credentials; error text is static safe text; device id random/resettable) |
| PROTO-001 | Schema/contract compatibility tests | 0.1 payloads accepted; unsupported major rejected | pass (host: protocol const enforced, unsupported version rejected before dispatch) |
| PROTO-002 | `scripts/check_docs.py` + UI fixtures | Observation schema/fixture valid; quality always explicit | pass (static schema fixture; companion contract tests validate repo + app fault fixtures against the same schema in `app/test/contract.test.ts`) |
| PROTO-003 | Host parser/import tests | LAN/USB semantics match; malformed input causes no mutation | pass (host: one shared router serves both transports; malformed frames return errors and never mutate state) |
| FW-001 | Clean host test + ESP32-C3 release build | Pinned toolchain, host tests, linked target image | pass (pinned toolchain, domain host tests, linked domain ELF; see `firmware/domain`) |
| FW-002 | Host unit/property tests with fakes | Domain behaviors run without physical hardware | pass (host: 74 unit + 3 fixture-contract tests in `firmware/domain` with injected adapter fakes) |
| UX-001 | Automated/manual accessibility + browser smoke | Keyboard/SR/focus/zoom/motion/contrast/table evidence | planned (host/jsdom structure tests exist in `app/test/ui.test.ts`: landmarks, skip link, labels, live region, text-not-color state; manual screen-reader, 200% zoom, contrast, and real-browser smoke remain not performed — needs issue #6 device + real browsers) |
| UX-002 | Component/contract tests | Every status has text/icon/age and no silent stale reuse | pass (host/jsdom: `app/test/state.test.ts` + `app/test/ui.test.ts` prove stale/offline readings stay visible with explicit text and are never reused as fresh; fault/calibration states render as sentences; real-browser rendering deferred to #6) |
| SAFE-001 | Documentation + environmental bench review | Passive-box target conditions documented and observed | planned |
| SAFE-002 | Docs/schematic/PCB review | Prohibitions visible and no prohibited interface | planned |
| SAFE-003 | Copy/UI/release review | No prohibited guarantee/certification language | planned |
| REL-001 | Release evidence audit | Every result labeled by evidence class; gaps explicit | planned |
| COST-001 | Dated BOM/fabrication quote | Included/excluded costs and ceiling evaluated without invention | pass (dated schematic-derived BOM + explicit non-schematic cost exclusions; no fabrication quote yet) |
| SRC-001 | Release/source/BOM audit | Editable sources and KiCad-property-exported tracked BOM | pass (KiCad sources committed; `bom/bom.csv` exported from symbol properties via `scripts/export_schematic_bom.py`) |

## Issue #1 evidence boundary

Completed here: requirement reconciliation, architecture diagrams, protocol decision/schema fixture, risk analysis, and a clean host/ESP32-C3 target build. Not performed: schematic/PCB analysis, simulation, flashing, bench measurement, calibration, environmental comparison, accessibility/browser tests, field testing, fabrication, sourcing, or cost validation.

## Issue #2 evidence boundary

Completed here: KiCad schematic/project sources, custom XIAO symbol/footprint library assets, schematic ERC, netlist/schematic analysis reports, and BOM/non-schematic BOM exports sourced from KiCad symbol properties.

Still not performed here: PCB layout, PCB DRC, signal/power-integrity simulation, target flashing, bench calibration, environmental/weight metrology validation, enclosure fabrication fit checks, and fabrication quote capture.

## Issue #4 evidence boundary

Completed here: pinned domain workspace (`firmware/domain`) with host tests, ESP32-C3 release link, and size report; allocation-free sampling/filtering, stability, gross/net, calibration lifecycle, freshness/disconnect/saturation/out-of-range states; bounded storage with checksums, interrupted-write recovery, capacity/retention behavior, export, and deletion; shared bounded LAN/USB command router with rate budget, presence sessions, and safe error text; pinned `espflash 4.5.0` flash/monitor/erase/recovery documentation.

Still not performed here: flashing, serial logs from hardware, bench measurement, driver integration (ADC/SHT40/flash/USB/Wi-Fi server), SSE streaming endpoint implementation, real LAN/USB adapter code, on-device rate/time enforcement inspection, and any accuracy claim. The linked image is target-build evidence only.

## Issue #5 evidence boundary

Completed here: device-hosted TypeScript/Vite companion (`app/`) with pinned toolchain; protocol v0.1 version gate with major rejection and additive-minor tolerance; connection/freshness/stale/fault/calibration status UI with text-only state semantics; guided tare/reference calibration and spool empty-mass entry flows; spool/desiccant/calibration event notes; bounded history with retention mirror, sequence dedup, and text data-table summaries; versioned CSV import validation (whole-file reject before mutation); backup validate → preview → digest-bound commit with credential-key rejection and FNV-1a parity with the firmware; scoped-deletion UI wired to the protocol scopes with on-device confirmation guidance; 42 host tests (contract against the canonical schema, state machine, backup/CSV parity, formatting, jsdom structure/label/live-region checks); loopback serve smoke with bundle size budgets; CI runs the app gates via `scripts/verify.sh`.

Still not performed here: real-browser smoke tests (Android Chrome, iOS Safari, Windows Edge, macOS Safari), manual screen-reader (TalkBack/VoiceOver) verification, 200%-zoom/contrast visual checks, live SSE interop with the firmware LAN adapter, USB-only workflow against hardware, on-device hosting of the built bundle, and any end-to-end mutation against a physical device. jsdom results are host-test evidence, not browser compatibility evidence.
