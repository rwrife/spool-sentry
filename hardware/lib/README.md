# Project-local KiCad library

This folder contains project-specific symbol/footprint assets required by
`hardware/spool-sentry.kicad_sch`.

## XIAO ESP32-C3 assets

- Symbol: `spool-sentry.kicad_sym` (`XIAO ESP32C3`)
- Footprint: `spool-sentry.pretty/XIAO_ESP32C3.kicad_mod`

These assets were adapted from the public `VectorSpaceHQ/XIAO_ESP32C3` KiCad
library and then adjusted for this project (local library references, metadata
cleanup). Upstream source:

- https://github.com/VectorSpaceHQ/XIAO_ESP32C3

The upstream GPL-3.0 license text is preserved in `XIAO_ESP32C3_LICENSE`.
Pin use was cross-checked against Seeed's XIAO ESP32-C3 manufacturer
documentation during issue #2 verification.

## NAU7802 symbol

`NAU7802SGI` in `spool-sentry.kicad_sym` is a project-authored symbol based on
Nuvoton NAU7802 Rev. 2.6 pin descriptions and uses KiCad's standard
`Package_SO:SOIC-16_3.9x9.9mm_P1.27mm` footprint.
