# Issue #2 — KiCad schematic + BOM verification

Date (UTC): 2026-09-07

## Scope completed in this issue

- Added KiCad project/schematic sources:
  - `hardware/spool-sentry.kicad_pro`
  - `hardware/spool-sentry.kicad_sch`
- Added project-local symbol/footprint assets for the XIAO module:
  - `hardware/lib/spool-sentry.kicad_sym`
  - `hardware/lib/spool-sentry.pretty/XIAO_ESP32C3.kicad_mod`
  - `hardware/lib/spool-sentry.dcm`
- Exported schematic-derived BOM artifacts:
  - `bom/bom.csv`
  - `docs/verification/component-properties.csv`
  - `docs/verification/bom-analysis.json`
- Captured non-schematic dependencies and excluded costs in:
  - `bom/non-schematic-items.csv`

## Executed verification commands

```bash
kicad-cli sch erc --format report --output docs/verification/kicad-erc.rpt hardware/spool-sentry.kicad_sch
./scripts/export_schematic_bom.py hardware/spool-sentry.kicad_sch
python3 /home/rwrife/repos/kicad-happy/skills/kicad/scripts/analyze_schematic.py hardware/spool-sentry.kicad_sch --output docs/verification/schematic-analysis.json
python3 /home/rwrife/repos/kicad-happy/skills/kicad/scripts/analyze_schematic.py hardware/spool-sentry.kicad_sch --output docs/verification/schematic-analysis.txt --text
./scripts/verify.sh
```

Observed results:
- ERC report: `0 Errors, 0 Warnings` (`docs/verification/kicad-erc.rpt`)
- BOM export script: `Exported 42 components and 23 physical BOM lines`
- Repository baseline verifier: exit code `0`

## Datasheet / manufacturer-document checks used for symbol+footprint mapping

1. **NAU7802 ADC**
   - Nuvoton NAU7802 Rev. 2.6, **PIN CONFIGURATION / PIN DESCRIPTION** (pp. 5–6) confirms mapping for `REFP(1), VIN1N(2), VIN1P(3), ... AVDD/LDO(16)`.
   - Rev. 2.6 **§2.4 16-pin Application Circuit** (pp. 23–24) supports the implemented bridge front-end pattern (47 Ω input series resistors, optional `Cfilter` at PGA output, local 0.1 µF / 1 µF decoupling).
2. **SHT40 environmental sensor**
   - Sensirion SHT4x v6.4 **§5.4 Pin Assignment & Laser Marking** (p. 16, Fig. 18) confirms `1=SDA, 2=SCL, 3=VDD, 4=VSS` used by `U3` and its DFN-4 footprint.
3. **USB-C receptacle**
   - GCT USB4105 drawing pin table confirms `A5=CC1`, `B5=CC2`, VBUS/GND pad groups and shell pad usage, matching `J1` wiring (UFP sink, dual 5.1 kΩ Rd, power-only).
4. **XIAO ESP32-C3 module pin usage**
   - Seeed manufacturer documentation (XIAO ESP32C3 Pin Map section) was used to bind `D4/D5` as I2C SDA/SCL, `D6/D7` as UART TX/RX, `D2/D3/D10` as RGB GPIO, and `D1` as button input.

## Analyzer exceptions (documented, narrow)

`docs/verification/schematic-analysis.txt` reports three schematic-level errors that are **tooling false positives** for this design snapshot:

1. `LR-001 LED D3 no current-limiting resistor`
   - False positive for a 3-channel RGB package symbol; resistor network exists as `R8/R9/R10` on `LED_R_K/LED_G_K/LED_B_K` nets.
2. `VM-001 I2C_SCL 5V/3.3V crossing`
3. `VM-001 I2C_SDA 5V/3.3V crossing`
   - False positives caused by mixed-power module symbol heuristics. Actual net membership for `I2C_SCL` and `I2C_SDA` is only U1 GPIO + U2 + U3 with pull-ups to `+3V3`; no VBUS node is present on these nets.

These exceptions are retained as analyzer limitations; KiCad ERC remains clean.

## BOM coverage status

- `docs/verification/bom-analysis.json` reports no missing required fields for non-testpoint components.
- `bom/bom.csv` includes quantity-grouped physical parts with per-line supplier/cost/date/lifecycle notes.
- `bom/non-schematic-items.csv` explicitly tracks items intentionally outside schematic scope (fabrication, enclosure, USB cable/supply, calibration mass reference, mating harness hardware, fasteners, load-cell mechanics).

## Not performed in this issue

- PCB placement/routing and PCB DRC
- Fabrication quote acquisition
- Bench calibration, environmental metrology, or load validation
- Hardware bring-up measurements on target boards
