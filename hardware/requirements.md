# Hardware requirement subset

The canonical frozen requirements are in [`docs/requirements.md`](../docs/requirements.md). Hardware design work shall trace to these IDs rather than creating alternate values.

## Applicable IDs

- Product boundary: PRD-001 through PRD-003
- Power/electrical: PWR-001 through PWR-003
- Measurement: MEAS-001 through MEAS-006
- Mechanical/environment: MECH-001 through MECH-004 and SAFE-001 through SAFE-003
- Connectivity/test access constraints: CONN-003, SEC-002
- Cost/source: COST-001, SRC-001

## Frozen design inputs

- One load-cell channel and one environmental channel.
- Centered gross operating range: 0–2.5 kg.
- Load-cell rated capacity: at least 5 kg, with the complete force path still requiring mechanical verification.
- Product environment target: 10–35 °C and 10–80% RH, non-condensing, passive dry box only.
- USB 5 V SELV only; no battery, charger, mains, heater, fan, relay, printer power/control, or safety output.
- Exact parts, input tolerance, current budget, sensor performance, packages, pinouts, and footprints remain issue #2 datasheet decisions and must not be invented here.

See [`docs/architecture.md`](../docs/architecture.md) for power and force-path diagrams, [`docs/risk-register.md`](../docs/risk-register.md) for controls, and [`docs/verification-matrix.md`](../docs/verification-matrix.md) for required evidence.
