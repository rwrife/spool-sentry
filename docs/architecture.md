# Spool Sentry v0.1 architecture

This architecture implements the frozen boundary in [requirements.md](requirements.md). Diagrams are Mermaid/text source and remain editable in git.

## End-to-end blocks

```mermaid
flowchart LR
  USB[USB 5 V SELV] --> PROTECT[Input protection]
  PROTECT --> RAIL[3.3 V rail]
  RAIL --> MCU[ESP32-C3 module]
  RAIL --> ENV[Temperature/RH sensor]
  RAIL --> ADC[Load-cell ADC]
  CELL[Replaceable bar load cell] --> ADC
  ENV --> MCU
  ADC --> MCU
  BUTTON[Setup/tare button] --> MCU
  MCU --> STATUS[Text-documented status indicator]
  MCU --> FLASH[Bounded config/history]
  MCU --> CDC[USB CDC setup/recovery/export]
  MCU --> HTTP[Local HTTP JSON + SSE]
  HTTP --> WEB[Device-hosted accessible web app]
```

Only observation data crosses the sensor boundary. There is no actuator output.

## Power domains

```mermaid
flowchart TD
  SUPPLY[Suitable enclosed USB supply] --> CABLE[Data-capable USB cable]
  CABLE --> VBUS[Protected USB VBUS test point]
  VBUS --> REG[3.3 V regulator]
  REG --> V33[3V3 test point]
  V33 --> RADIO[ESP32-C3]
  V33 --> DIGITAL[Environmental sensor]
  V33 --> ANALOG[Load-cell ADC/bridge excitation]
  VBUS --> GND[Common SELV ground/test point]
  V33 --> GND
```

Issue #2 must replace generic blocks with datasheet-backed parts and document worst-case/inrush current with 20% path margin. The PCB shall keep load-cell analog routing/return away from radio and digital switching.

## Data and ownership

```mermaid
flowchart LR
  SENSOR[Sensor samples] --> VALIDATE[Range/status validation]
  VALIDATE --> FILTER[Bounded filters + stability]
  FILTER --> OBS[Versioned observation]
  OBS --> LIVE[LAN SSE / USB snapshot]
  OBS --> AGG[Hourly aggregate]
  AGG --> STORE[Bounded local history]
  EVENT[User events/metadata] --> STORE
  STORE --> EXPORT[User-triggered CSV/JSON export]
  STORE --> BACKUP[Versioned backup]
  BACKUP --> CHECK[Validate + preview]
  CHECK -->|explicit commit| STORE
  DELETE[Confirmed scoped deletion] --> STORE
```

The device is authoritative. The browser may cache static assets and ephemeral preferences, not authoritative history or credentials. Restore never mutates before validation and explicit commit.

## Provisioning and recovery

```mermaid
sequenceDiagram
  actor User
  participant Button as Physical button
  participant Device
  participant Client as LAN client or USB host
  User->>Button: Hold to request presence session
  Button->>Device: Local hardware event
  Device-->>Client: Short-lived random session capability
  Client->>Device: Provision/configure with bounded request
  Device-->>Client: Result without echoing credentials
  Device->>Device: Expire session automatically
  alt Wi-Fi unavailable or invalid
    User->>Device: Connect USB CDC
    Client->>Device: status/snapshot/diagnostics/export
    User->>Button: Confirm destructive recovery if needed
    Client->>Device: provision/delete/reset
  end
```

LAN HTTP assumes a trusted network and is not exposed through a cloud relay, port mapping, or internet discovery. USB is the mandatory offline recovery path.

## Mechanical force path

```mermaid
flowchart TD
  SPOOL[One spool: centered working load ≤ 2.5 kg] --> TOP[Removable platform]
  TOP --> MOUNT_A[Specified load-introduction fasteners]
  MOUNT_A --> CELL[Replaceable ≥5 kg bar load cell]
  CELL --> MOUNT_B[Fixed-end fasteners + spacers]
  MOUNT_B --> BASE[Rigid base/enclosure]
  BASE --> FEET[Stable support surface]
  PCB[Electronics carrier] -. mechanically isolated .-> BASE
  ENV[Ventilated environmental sensor] -. away from MCU/regulator .-> BASE
```

The moving platform may not contact the PCB or enclosure over the working range. Off-center and overload behavior remain bench-verification items; the design shall not claim universal spool fit.

## Responsibility boundaries

| Boundary | Owns | Does not own |
|---|---|---|
| Hardware | protected SELV input, sensors, controller, test/debug access, carrier, force path | mains/battery/charging, actuation, safety interlocks |
| Firmware | sampling, quality/fault state, calibration, bounded storage, protocol, provisioning/recovery | cloud service, print prediction, certified measurement |
| Companion | accessible local workflows, history, events, export/restore/deletion | authoritative storage, phone permissions, cloud sync |
| Protocol | versioned bounded LAN/USB semantics and fixtures | hostile-internet security, remote access |

## Dependency gates

1. This requirements/architecture baseline gates exact part selection (#2), host-domain firmware (#4), and fixture-first companion work (#5).
2. Exact hardware drivers wait for the datasheet-backed pins and interfaces from #2.
3. PCB/mechanics (#3) wait for the reviewed schematic and exact parts.
4. Bench claims and release outputs wait for an assembled prototype (#6/#7).
