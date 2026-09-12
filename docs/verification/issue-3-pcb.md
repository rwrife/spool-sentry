# Issue #3 — Carrier PCB + mechanical force path verification

Date (UTC): 2026-09-12

Evidence classes in this document are strictly separated:
**static analysis** (KiCad DRC/ERC, analyzers), **target build**, **host
tests**, and explicit **not-performed** gaps. NOTHING here is bench,
environmental, or fabrication evidence: no board has been ordered, built,
or measured, and no mechanical part has been printed or load-tested.

## Scope completed in this issue

- Editable carrier PCB: `hardware/spool-sentry.kicad_pcb` (66.0 × 75.0 mm,
  two copper layers), generated deterministically by `scripts/build_pcb.py`
  through the pcbnew API (KiCad 9.0.9 image `parts-tally-kicad:9-arm64`) and
  re-runnable from a clean clone.
- Net classes `Power` (0.40 mm / 0.25 mm clearance) and `Analog`
  (bridge/ADC nets, default widths) are declared in `spool-sentry.kicad_pro`
  with explicit `netclass_patterns` binding: Power = VBUS, VBUS_FUSED,
  +5V_XIAO, +3V3; Analog = AIN+/AIN−, LC_S+/LC_S−, AVDD_3V0, VBG,
  PGA_CFILTER.
- Antenna keepout enforced as a real KiCad rule area (copper pour carve-out,
  both copper layers, priority 1 so it cuts the pours):
  `AntennaKeepout` 16–31 × 25–40.5 mm wraps the XIAO ESP32-C3 PCB antenna
  end with ≥3 mm margin; silkscreen legend "ANTENNA KEEPOUT - NO COPPER -
  NO METAL". Two additional rule areas protect the edge connector and
  load-cell cable exit. `scripts/probe_zones.py` prints every zone's layer,
  net, rule-area flag, priority, and filled area.
- Ground strategy: filled GND pours on B.Cu (primary) and F.Cu (secondary,
  ~3.0–4.0 k mm² filled per layer after final refill). Analog bridge/ADC
  front-end (U2 + passives, J2) sits in the right/bottom column away from
  U1/USB switching; the SHT40 (U3) sits in the far top-right corner ≥ ~38 mm
  from U1 and the power chain, at the vented enclosure corner (MECH-003
  placement decision; bench heat-bias comparison remains issue #6 work).
- Ten test points (TP1–TP10) expose VBUS, +5V_XIAO, +3V3, GND, AVDD, I2C
  SDA/SCL, AIN+/AIN−; TP references and net labels stay on silkscreen.
- Silkscreen documentation: polarity/pin-1 marks from footprints, plus
  readable board text: board name/rev ("SPOOL SENTRY CARRIER REV A0"),
  "USB 5V SELV ONLY - OBSERVATION ONLY - MIT" license/usage mark, USB 5V,
  LOAD CELL, UART pinout, TARE, per-rail labels, and the antenna legend.
- Routing: FreeRouter 2.1.0 (headless, JDK 21) incremental SES passes with
  per-pass DSN export/import loop persisted under
  `hardware/reports/routing/` (pass1…pass7 `.dsn/.ses/logs/-drc.json`).
- Schematic↔PCB pin/net cross-check: `scripts/crosscheck_pin_net.py`
  compares every netlist node against the board pad nets
  (117 non-testpoint pin checks, 0 mismatches; the 15 schematic
  `unconnected-(...)` stubs are single-pin nets on the board).
- Editable mechanical source: `mechanical/spool-sentry-platform.scad`
  (OpenSCAD) — base plate, removable platform with centering spigot,
  bolted load-cell bracket, overload stop posts, and mechanically isolated
  carrier tray with USB/harness slots and SHT40-corner vents. Parametric
  load-cell envelope with explicit ASSUMED markers (no MPN frozen; cell
  geometry must be set from the selected part's drawing before fabrication).
  STL previews exported via `scripts/export_mechanical_stl.sh`
  (`mechanical/*.stl`) — review-only render evidence, not printed parts.
- Fabrication preview (review-only, NOT for ordering without the open
  items below): `hardware/reports/fab-preview/` holds Gerber + drill set
  (`ger/`), pick-and-place CSV, and a PDF layer preview
  (`board-preview-rev-a0.pdf`).

## Executed verification commands

```bash
# board generation + zones (KiCad 9.0.9 image; pcbnew API)
python3 scripts/build_pcb.py
python3 scripts/add_zones.py hardware/spool-sentry.kicad_pcb   # idempotent; raises rule-area priority, refills
python3 scripts/probe_zones.py hardware/spool-sentry.kicad_pcb

# routing loop (repeat per pass; DRC adjudicates, never the router log)
python3 scripts/export_dsn.py hardware/spool-sentry.kicad_pcb hardware/reports/routing/passN.dsn
timeout 780 java -Djava.awt.headless=true -jar freerouting-2.1.0.jar \
  -de passN.dsn -do passN.ses -hf true -gui.enabled=false
python3 scripts/import_ses.py hardware/spool-sentry.kicad_pcb hardware/reports/routing/passN.ses
python3 scripts/add_zones.py hardware/spool-sentry.kicad_pcb   # refill after every SES import (pitfall #37)
kicad-cli pcb drc --format json --output hardware/reports/routing/passN-drc.json hardware/spool-sentry.kicad_pcb
python3 scripts/unconn_summary.py hardware/reports/routing/passN-drc.json

# final adjudication
kicad-cli pcb drc --format json --output hardware/reports/kicad-drc-final.json hardware/spool-sentry.kicad_pcb
python3 scripts/crosscheck_pin_net.py docs/verification/spool-sentry.net hardware/spool-sentry.kicad_pcb
python3 .../kicad/scripts/analyze_pcb.py hardware/spool-sentry.kicad_pcb --output docs/verification/pcb-analysis.json
python3 .../kicad/scripts/cross_analysis.py --schematic docs/verification/schematic-analysis.json \
  --pcb docs/verification/pcb-analysis.json --output docs/verification/cross-analysis.json
python3 .../emc/scripts/analyze_emc.py --schematic docs/verification/schematic-analysis.json \
  --pcb docs/verification/pcb-analysis.json --output docs/verification/emc-analysis.json
kicad-cli sch erc --format report --output docs/verification/kicad-erc.rpt hardware/spool-sentry.kicad_sch
kicad-cli pcb export gerbers --output hardware/reports/fab-preview/ger/ hardware/spool-sentry.kicad_pcb
kicad-cli pcb export drill --output hardware/reports/fab-preview/ger/ hardware/spool-sentry.kicad_pcb
kicad-cli pcb export pos --format csv --output hardware/reports/fab-preview/placement-pos.csv hardware/spool-sentry.kicad_pcb
kicad-cli pcb export pdf --layers "F.Cu,B.Cu,F.Silkscreen,B.Silkscreen,Edge.Cuts" \
  --output hardware/reports/fab-preview/board-preview-rev-a0.pdf hardware/spool-sentry.kicad_pcb
./scripts/export_mechanical_stl.sh
./scripts/verify.sh
python3 scripts/check_docs.py
```

Observed results (exact):

- `kicad-cli pcb drc` (final): see `hardware/reports/kicad-drc-final.json` —
  hard-violation count 0; ratsnest remainder documented below.
- Pin/net cross-check: `netlist pins checked: 117, NC stubs verified
  single-pin on board: 15, board pads mapped: 128, mismatches: 0` (exit 0).
- ERC stays clean: `0 Errors, 0 Warnings` (`docs/verification/kicad-erc.rpt`).
- Zones after final fill: GND pours F.Cu ≈ 3.0 k mm², B.Cu ≈ 4.0 k mm²;
  six antenna/edge rule areas at priority 1 carve the pours.
- `scripts/verify.sh`: `VERIFY: PASS` (docs checks, firmware host tests +
  clippy `-D warnings` + ESP32-C3 release links, app lint/tests/build/smoke).
- `scripts/check_docs.py`: `CHECK_DOCS: PASS`.

## Ratsnest residual (documented exception, NOT claimed routed)

FreeRouter fanout-mode passes plateaued; each pass was adjudicated with a
fresh `kicad-cli pcb drc --format json` and only the best board state was
kept (90 → 85 → 65 → 49 → 44 → 33 → 30; final DRC: 0 hard violations,
30 ratsnest pairs — see `hardware/reports/kicad-drc-final.json`). The final
unconnected list decomposes as:

1. **USB-C shell/anchor + GND pad↔zone pseudo-ratsnest (11 pairs: J1
   S1×6, J2 MP, zone pairs).** Shell/anchor pads are GND-bound and the
   filled pour touches them; the rat lines are KiCad pseudo-ratsnest
   artifacts against zero-clearance zone items. Electrically bonded.
2. **True unrouted stubs (19 pairs):** CC1/CC2 (J1 fanout), AIN− fan-in,
   +3V3→U3, +5V_XIAO pad 14, BUTTON_N, AVDD J2–TP5, LC_S± J2→R3/R4,
   VBG, I2C SDA/SCL at U3, VBUS J1→D1, plus intra-J1 GND/A4-B4 pairs.
   A deterministic stub closer (`scripts/close_stubs.py`, capsule-collision
   recipe) was run on a sandbox copy against the final DRC: of 18 non-pseudo
   pairs it could lawfully close only 1 (a 27 mm LC_S− diagonal) — the rest
   are blocked by foreign-copper collision or layer-change requirements.
   At the default 0.2 mm/0.2 mm rules around the fine-pitch USB-C fanout
   and antenna-keepout carve, these are a physics gate (skill pitfall #30),
   so they are carried as a **documented residual with named fix**: local
   clearance-relaxation net class over the J1 fanout region, bottom-layer
   fanout around J2, and manual routing in pcbnew during issue #6 prep —
   not silently ignored.

Acceptance criterion "rout(ing) ... with named net classes and ground
strategy" is therefore met *except* for this explicit residual: the board is
not fab-ready-clean on connectivity until item 3 is closed. The DRC JSON is
the authoritative count; do not cite the router log.

## Analyzer findings and triage

`analyze_pcb.py` (docs/verification/pcb-analysis.json) — post-triage:

- `FD-001` no fiducials (F.B, 13–28 SMD parts): real fabrication gap —
  fiducials are added in the fab-readiness pass before ordering, not
  silently shipped; tracked as the open item below.
- `KO-001` "J1/J2 inside keepout zone": **expected by design.** The
  edge-connector/load-cell rule areas intentionally permit the connectors
  they protect (pads/footprints allowed; only copper pour is carved —
  pitfall #33). Documented exception, not a violation.
- `RT-001` unrouted nets: the ratsnest residual above.
- Silkscreen/placement audit: no courtyard overlaps; silk kept readable
  (test-point and control refs visible at DRC-legal size).

`cross_analysis.py`: single `XV-002` warning (U1 value mismatch "ESP32C3"
vs schematic "Seeed XIAO ESP32-C3") — fixed via `scripts/sync_values.py`
and the generator override; re-run confirms.

EMC (`analyze_emc.py`, `docs/verification/emc-analysis.json`): risk score
76/100, 7 findings (3 error / 0 warning / 4 info), triaged:

- `DC-002` "No decoupling cap found near U3" — **real.** The SHT40's 100 nF
  bypass (C4) currently sits on the U2 side of the board. Fix named: relocate
  or add a local 100 nF at U3 in the issue #6 layout revision; not silently
  shipped.
- `DP-001` differential pair skew (AIN+/AIN−) — triaged as **expected**:
  this is a DC bridge-measurement pair, not a high-speed differential
  interface; spec skew limits do not apply. Documented exception.
- `IO-001` no EMC filtering at J1/J2/J3 — **accepted scope decision:** the
  design is a USB 5 V SELV observation-only device in a passive dry box;
  common-mode filtering is a fab-readibility item for the rev-A1 pass and is
  recorded as an open item, not a blocker for review-only artifacts.
- `GP-001` return-path data unavailable — analyzer limitation (connectivity
  graph requires a fully routed board; see ratsnest residual). `EE-001`
  cavity resonance — info, enclosure-dependent; revisit with issue #6
  enclosure geometry.

## Mechanical evidence (review-only)

- OpenSCAD source builds cleanly (OpenSCAD 2021.01, `--export_stl` for all
  parts); assembly preview STL renders the full force path. Geometry notes:
  cell-envelope numbers are ASSUMED placeholders (marked in-file); the
  overload stop gap (deflection + 3 mm) is a design intent for bench
  verification under MECH-001, not a validated overload path.
- Spool fit envelope: Ø205 × 70 mm "common spool" envelope only — explicit
  MECH-004 boundary: **no universal-spool claim.**
- Not performed: printing, fit checks, deflection/strain measurement,
  off-center or overload testing. All bench items land in issue #6.

## Open items before fabrication release (carry to #6/#7)

1. Close the true unrouted stubs (named fix above) and re-DRC to 0 ratsnest
   real gaps.
2. Add a local 100 nF bypass at U3/SHT40 (EMC DC-002) and re-run decoupling
   analysis.
3. Add fiducials (FD-001) once panelization is known.
4. Freeze load-cell MPN + drawing, update SCAD parameters, re-export.
5. Fabrication quote + stackup freeze (bom/non-schematic-items.csv still
   says TBD; no quote exists).
6. Decide whether USB/J2/J3 common-mode filtering (EMC IO-001) enters rev A1.

## Issue #3 evidence boundary

Completed here: editable two-layer carrier PCB (generated, placed, net-class
bound, keepout-enforced, GND-poured, silkscreened, DRC-adjudicated with 0
hard violations and an explicit ratsnest residual), schematic↔PCB pad/net
cross-check, ERC re-confirm, Gerber/drill/PnP/PDF review previews, editable
parametric mechanical source + STL previews, analyzer + EMC runs with
false-positive triage.

Still not performed here: any physical fabrication or assembly; bench
electrical, mechanical, thermal, or EMC measurement; load calibration;
spool fit trials; the residual copper stubs in the named exception above;
fiducial/panel decisions; and any claim that this design is tested,
calibrated, safe-certified, or guaranteed to detect filament conditions or
save prints. Observation-only USB 5 V SELV boundary preserved: no mains,
battery charging, actuation, or safety-alarm interface exists anywhere in
schematic, PCB, or mechanical sources.
