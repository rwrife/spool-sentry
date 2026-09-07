# Hardware design artifacts

This directory now carries the schematic source-of-truth for the Spool Sentry carrier board.

## Included files

- `spool-sentry.kicad_pro` / `spool-sentry.kicad_sch`: KiCad 9 project + schematic
- `sym-lib-table`, `fp-lib-table`: local library bindings used by this schematic
- `lib/spool-sentry.kicad_sym`: project-local custom symbols (XIAO ESP32-C3 module, NAU7802 ADC)
- `lib/spool-sentry.pretty/XIAO_ESP32C3.kicad_mod`: project-local XIAO module footprint
- `lib/spool-sentry.dcm`: symbol documentation metadata

## What this schematic covers

- USB 5V SELV power entry and protection path (USB-C UFP + PPTC + TVS + reverse-current block)
- Controller module (`U1`: Seeed XIAO ESP32-C3)
- Bridge ADC path (`U2`: NAU7802) and load-cell connector (`J2`)
- Environmental sensor path (`U3`: SHT40) on shared I2C
- Dedicated user button (`SW1`) for tare/calibration/recovery interactions
- Status RGB indicator (`D3` + current-limit resistors)
- Test points for safe bring-up/diagnostics

## Regenerating verification artifacts

From the repository root:

```bash
# Schematic ERC (text report)
kicad-cli sch erc --format report \
  --output docs/verification/kicad-erc.rpt \
  hardware/spool-sentry.kicad_sch

# Netlist + BOM exports from schematic symbol properties
./scripts/export_schematic_bom.py hardware/spool-sentry.kicad_sch

# Optional external schematic analyzer outputs (JSON + text)
# (path depends on local Hermes skill checkout)
# python3 <path-to-analyze_schematic.py> hardware/spool-sentry.kicad_sch --output docs/verification/schematic-analysis.json
# python3 <path-to-analyze_schematic.py> hardware/spool-sentry.kicad_sch --output docs/verification/schematic-analysis.txt --text
```

The issue-specific validation summary is in
`docs/verification/issue-2-hardware-schematic.md`.