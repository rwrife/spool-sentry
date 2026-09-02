# Local web companion plan

## Responsibilities

The responsive device-hosted TypeScript/Vite app will provide:

- proof-of-presence setup and local connection status
- live humidity, temperature, gross mass, net estimate, freshness, calibration, and fault states
- guided tare/reference-mass calibration with confirmation and validation
- spool label/material notes and empty-spool mass (user-entered, not inferred)
- local history charts and desiccant/spool/calibration event annotations
- versioned CSV/JSON export, JSON backup/restore preview, retention, deletion, and factory-reset guidance
- diagnostics suitable for redacted support without credentials or stable network identifiers

## Setup flow

1. User physically enables onboarding on the device.
2. Phone/desktop opens the short-lived local setup page or USB workflow.
3. User reviews network/security limits and submits local Wi-Fi settings.
4. Device confirms connection without echoing secrets; setup mode expires.
5. User completes sensor status check, tare/reference calibration, and optional spool metadata.

## Data ownership and permissions

All data remains on the device until the user exports it. No cloud account, analytics, advertising, third-party API, or internet connection is required. The baseline web app requests no location, contacts, camera, microphone, Bluetooth, notification, or background-sensor permissions. Browser storage may cache static assets and ephemeral preferences only; authoritative history/configuration stays on the device. Export, restore, retention, and deletion are explicit user actions.

## Accessibility

Primary flows must work with keyboard navigation, TalkBack/VoiceOver-compatible browser semantics, 200% text scaling, visible focus, reduced motion, high contrast, and non-color-only stale/fault/calibration indicators. Charts require textual summaries and data tables.

## Protocol boundary

`docs/protocol.md` is the versioned boundary. The app consumes fixtures in tests and must tolerate additive fields, reject unsupported major versions, validate imports before mutation, and show offline/stale states instead of silently reusing old readings.

## Test strategy

- Unit tests for units/formatting, uncertainty display, reducers, migrations, import validation, and deletion/retention decisions
- Component and accessibility tests for setup, calibration, fault, history, and export flows
- Protocol contract tests against firmware-generated fixtures and fault replays
- Production build, asset-size budget, and device-host serving test
- Browser smoke tests on current Android Chrome, iOS Safari, Windows Edge, and macOS Safari before compatibility claims
