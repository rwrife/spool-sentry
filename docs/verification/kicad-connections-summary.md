# KiCad connection summary (Issue #2)

Source schematic: `hardware/spool-sentry.kicad_sch`

- Components: 42
- Nets: 25
- Power nets: `+3V3`, `+5V_XIAO`, `GND`, `VBUS`, `VBUS_FUSED`
- Major interfaces:
  - I2C: `I2C_SDA`, `I2C_SCL` (U1, U2, U3 with 3.3V pull-ups)
  - Load cell analog: `LC_S+`, `LC_S-`, `AIN+`, `AIN-`, `AVDD_3V0`
  - USB-C entry: `VBUS`, `VBUS_FUSED`, `CC1`, `CC2`
  - Status LED channels: `LED_R_GPIO/K`, `LED_G_GPIO/K`, `LED_B_GPIO/K`
  - Dedicated button: `BUTTON_N`

Detailed machine-readable net membership is in:
- `docs/verification/kicad-connections-summary.json`
- `docs/verification/spool-sentry.net`
