# Local web companion

Device-hosted accessible companion for Spool Sentry. Built status (issue #5):
implemented and host-tested; not yet exercised on real browsers or a
physical device.

## Responsibilities (implemented)

- connection status and proof-of-presence guidance (onboarding flow text;
  the live onboarding route lands with the firmware LAN adapter in #6)
- live humidity, temperature, gross mass, net estimate, freshness,
  calibration, and fault states from protocol v0.1 snapshot/SSE payloads
- guided tare/reference-mass calibration with confirmation and validation
- spool metadata entry and empty-spool mass (user-entered, never inferred)
- local history view with textual chart summaries and data tables, plus
  desiccant/spool/calibration event annotations
- versioned CSV/JSON export, JSON backup validate → preview → commit,
  retention mirror, scoped deletion, and factory-reset guidance
- diagnostics suitable for redacted support without credentials or stable
  network identifiers

## Layout

- `src/protocol.ts` — v0.1 types, major-version gate, bounded request ids
- `src/state.ts` — link/freshness/history state machine (stale stays visible)
- `src/backup.ts` — backup envelope + FNV-1a parity with firmware
- `src/csvio.ts` — RFC 4180 parser + frozen v0.1 export-header validation
- `src/format.ts` — units, uncertainty text, state/fault sentences
- `src/client.ts` — LAN HTTP/SSE client, same-origin only, injectable
- `src/main.ts` — accessible vanilla-DOM UI rendered into `index.html`
- `fixtures/` — fault/stale/disconnected/saturated/uncalibrated cases
- `scripts/serve_smoke.mjs` — loopback serve + size-budget smoke test

## Commands (from `app/`)

```sh
npm ci
npm run lint    # tsc --noEmit + prettier --check
npm test        # vitest (host; jsdom for structure tests)
npm run build   # tsc --noEmit + vite build -> dist/ (device-hostable)
npm run smoke   # serve dist/ on loopback and check bundle + mock API
```

`./scripts/verify.sh` from the repository root runs all of these in CI.

## Data ownership and permissions

All data remains on the device until the user exports it. No cloud account,
analytics, advertising, third-party API, or internet connection is required.
The app requests no location, contacts, camera, microphone, Bluetooth,
notification, or background-sensor permissions. Browser storage caches
static assets only; authoritative history/configuration stays on the device.
Export, restore, retention, and deletion are explicit user actions. The UI
never fabricates device confirmation tokens; destructive operations guide
the user through on-device presence confirmation.

## Accessibility

Implemented: semantic landmarks, skip link, labelled form controls,
`role=status`/`aria-live` announcements, text-first state descriptions
(never color-only), visible focus styles, `prefers-reduced-motion` and
`prefers-contrast` handling, zoom-friendly layout, and text data tables for
history. Verified by host tests (`test/ui.test.ts`) inside jsdom only.
Manual screen-reader, 200% zoom, and real-browser checks (Android Chrome,
iOS Safari, Windows Edge, macOS Safari) are **not yet performed** and stay
tracked in the verification matrix (UX-001) for issue #6.

## Protocol boundary

`docs/protocol.md` is the versioned boundary and its
"Implementation reconciliation" section records the exact payload shapes
this app implements and contract-tests. The app tolerates additive minor
fields, rejects unsupported majors before touching stored data, validates
imports before mutation, and shows offline/stale states instead of silently
reusing old readings.

## Test strategy (status)

- unit tests for formatting/uncertainty/state machine/import validation —
  done (host)
- contract tests against the canonical schema and firmware-shaped backup
  envelopes — done (host)
- structure/label/live-region component tests — done in jsdom only
- production build, asset-size budget, loopback device-host serve test —
  done (Linux/node)
- browser smoke tests on real Android/iOS/Windows/macOS browsers — not
  performed; required before any compatibility claim (issue #6)
