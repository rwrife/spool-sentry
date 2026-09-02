# Hardware and system requirements

All values are design targets to be validated; they are not measured results.

## Electrical

| ID | Requirement |
|---|---|
| E-01 | Accept USB 5 V SELV power only; no mains, battery, charger, heater, fan, relay, or printer-power interface. |
| E-02 | Define a worst-case current budget with at least 20% design margin after exact component selection. |
| E-03 | Expose protected, labeled test access for input power, regulated rail, ground, programming/debug, sensor bus, and load-cell ADC signals. |
| E-04 | Power-up, sensor disconnect, ADC saturation, corrupt calibration, and storage faults shall produce explicit invalid/fault states. |
| E-05 | Preserve the selected ESP32-C3 module's manufacturer antenna keepout and placement requirements. |
| E-06 | Keep load-cell analog routing and return paths away from radio/digital switching; validate noise and settling on the bench. |

## Measurement

| ID | Requirement |
|---|---|
| M-01 | Report temperature and relative humidity with timestamp, freshness, sensor status, and documented sensor accuracy limits. |
| M-02 | Target a 0–2.5 kg spool/platform operating range with a mechanically selected load cell carrying at least 2× the rated working load. |
| M-03 | Target repeatability within the greater of 5 g or 1% of applied mass from 100 g to 2.0 kg after warm-up and calibration; verify with documented references. |
| M-04 | Never display more precision than calibration/repeatability evidence supports. |
| M-05 | Net filament estimate equals stable gross mass minus user-entered empty-spool/platform tare; expose assumptions and uncertainty. |
| M-06 | Store calibration timestamp, reference mass, factor, zero offset, and invalidation reason; require recalibration after relevant configuration changes. |

## Mechanical

| ID | Requirement |
|---|---|
| ME-01 | Platform shall support a centered 2.5 kg static load without contact between moving load structure and PCB. |
| ME-02 | Load cell, environmental sensor, USB cable, and carrier shall be replaceable with ordinary hand tools. |
| ME-03 | Sensor shall be vented and separated from MCU/regulator heat; location and airflow assumptions shall be documented. |
| ME-04 | Carrier shall include mounting holes, connector strain relief, polarity/pin-1 markings, and accessible setup/tare control. |
| ME-05 | Enclosure/platform source shall be editable and accommodate common spool diameters without claiming universal fit. |

## Environmental and safety

| ID | Requirement |
|---|---|
| S-01 | Indoor hobby/workshop use in a passive dry box only; target 10–35 °C and 10–80% RH non-condensing pending component validation. |
| S-02 | No placement in heated dryers, ovens, printer enclosures, wet locations, or safety-critical systems. |
| S-03 | Device shall not claim fire prevention, certified humidity, filament dryness, or guaranteed print success. |
| S-04 | Failure or loss of connectivity shall not actuate anything and shall not affect printer operation. |

## Connectivity, privacy, and data

| ID | Requirement |
|---|---|
| C-01 | Core setup/recovery and current snapshot shall remain available over USB without internet or cloud service. |
| C-02 | Wi-Fi onboarding shall require recent physical proof of presence and expire automatically. |
| C-03 | Local API and data formats shall be versioned; malformed/imported data shall fail closed without erasing current data. |
| C-04 | Store at least 30 days of hourly aggregates within a bounded retention policy; exact capacity shall be measured. |
| C-05 | Provide user-triggered CSV/JSON export, versioned backup/restore, retention controls, and full measurement/config deletion. |
| C-06 | No telemetry, ads, required account, automatic remote access, or baseline phone sensor permissions. |

## Cost and buildability

| ID | Requirement |
|---|---|
| B-01 | Target USD 35–60 and a USD 75 ceiling for one prototype excluding printer, phone/computer, tools, shipping, and tax. |
| B-02 | Re-price from live distributor/fabrication evidence before ordering; preliminary BOM prices remain TBD. |
| B-03 | Prefer sourceable parts, documented alternates where safe, two-layer PCB, and hand-assembly-friendly packages where practical. |
| B-04 | Release real editable KiCad/mechanical sources, schematic-backed BOM, firmware, app, assembly/bring-up docs, and inspected fabrication outputs. |
