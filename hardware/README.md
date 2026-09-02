# Hardware plan

## System block description

A suitable USB 5 V supply feeds protected low-voltage power and a 3.3 V rail. An ESP32-C3-class module reads one digital temperature/humidity sensor and one bridge load-cell ADC, controls a setup/tare button and status indicator, stores bounded data, exposes USB CDC, and serves the local web interface over Wi-Fi.

## Controller choice

The provisional platform is an ESP32-C3 module because it combines local Wi-Fi, adequate GPIO, broad community tooling, and low module cost. The exact module, antenna variant, USB/programming topology, and Rust toolchain remain selection tasks. Manufacturer datasheets and module integration guidance are required before schematic capture.

## Interfaces

- One digital temperature/humidity sensor bus with power and serviceable connector or board placement
- One load-cell ADC interface and keyed load-cell connector
- USB 2.0 device/serial setup and recovery path as supported by the selected module/topology
- One physical setup/tare input and non-color-only status indication
- Programming/debug header or test pads and named power/signal test points
- 2.4 GHz Wi-Fi with a documented antenna keepout

## Power plan

USB 5 V SELV only. The carrier will include a datasheet-backed connector/input-protection strategy, regulated 3.3 V as required, local decoupling/bulk capacitance, and current measurement points. No battery, charger, heater, fan, printer-power connection, or mains circuit is allowed. Power/current budgets and protection values remain TBD until exact parts are selected.

## Enclosure and assembly concept

A printed or fabricated low-profile platform transfers spool load to a replaceable bar load cell without loading the PCB. The sensor sits in vented air away from MCU/regulator heat and filament contact. The carrier uses hand-assemblable parts where practical, accessible connectors, mounting holes, visible orientation markings, and replaceable modules/sensors. Editable mechanical source is expected before release; photos/renders remain placeholders until real artifacts exist.

## Safety limits

Observation-only passive dry boxes; suitable enclosed USB supply; no heated chamber integration; no mains, batteries, relays, printers, safety interlocks, hazardous loads, or certified environmental/metrology claims. Faults must become explicit stale/invalid states rather than reassuring values.

## Expected KiCad deliverables

Planned editable sources (currently absent):

- `hardware/spool-sentry.kicad_pro`
- `hardware/spool-sentry.kicad_sch`
- `hardware/spool-sentry.kicad_pcb`

The mature design must include power/protection, controller/module, sensors/connectors, programming/debug, test points, mounting, silkscreen, antenna/sensor constraints, net classes, ERC/DRC evidence, schematic PDF, Gerber/drill, BOM/CPL where applicable, and source licensing. Image/PDF exports supplement but never replace KiCad sources.

Final part identity belongs in KiCad symbol properties and exports to `bom/bom.csv`. The current preliminary BOM is planning-only.
