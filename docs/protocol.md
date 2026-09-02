# Device/companion protocol draft

**Status:** planning contract; no implementation or interoperability evidence exists yet.

## Transport

- Local HTTP plus optional WebSocket/SSE on a trusted home/workshop LAN; exact transport is unresolved.
- USB CDC provides newline-delimited request/response frames for provisioning, recovery, diagnostics, and current snapshot.
- No cloud relay, public internet endpoint, or remote-access feature.

## Versioning

Every payload includes `protocol_version` (`major.minor`), `device_id` (random/resettable, not a hardware serial), and `request_id` where applicable. Clients reject unsupported major versions, ignore documented additive fields, and never mutate state from an invalid payload.

## Core observation shape

```json
{
  "protocol_version": "0.1",
  "sequence": 42,
  "sampled_at_ms": 123456,
  "temperature_c": 23.4,
  "relative_humidity_pct": 18.2,
  "gross_mass_g": 812,
  "net_mass_estimate_g": 574,
  "stable": true,
  "calibration_state": "valid",
  "quality": {
    "environment": "fresh",
    "mass": "fresh",
    "uncertainty_g": 8
  },
  "faults": []
}
```

Numbers may be `null` when unavailable. Quality/fault fields are mandatory so stale, settling, disconnected, saturated, uncalibrated, storage-failed, and out-of-range states cannot masquerade as valid zeroes.

## Planned endpoints/commands

| Operation | Purpose | Mutation protection |
|---|---|---|
| `GET /api/v1/status` | Device/build/protocol/capability and current quality summary | Read-only |
| `GET /api/v1/observations` | Bounded aggregates by time range | Read-only, range limits |
| `POST /api/v1/events` | Add spool/desiccant/calibration annotation | Active local session |
| `POST /api/v1/calibration/tare` | Begin/confirm tare workflow | Physical-presence session + confirmation |
| `POST /api/v1/calibration/reference` | Apply known reference mass | Physical-presence session + validation |
| `GET /api/v1/export` | User-requested versioned CSV/JSON | Active local session |
| `POST /api/v1/restore/validate` | Parse and preview backup without mutation | Active local session |
| `POST /api/v1/restore/commit` | Commit validated backup | Fresh confirmation token |
| `DELETE /api/v1/data` | Delete history/configuration by explicit scope | Physical presence + fresh confirmation |
| USB `status`, `snapshot`, `provision`, `reset` | Wi-Fi-free setup/recovery subset | Cable access; destructive actions require button confirmation |

## Security assumptions

- The device is used on a trusted local network. Local HTTP, if retained, does not protect contents from hostile LAN peers.
- Onboarding and sensitive mutation require a short-lived random session initiated by a physical button; tokens are never placed in URLs, logs, or exports.
- Bind to local interfaces only, reject unexpected origins/hosts, set tight body/rate/time limits, and avoid default passwords.
- Wi-Fi credentials are write-only after provisioning and excluded from diagnostics, backups, and factory exports.
- Device IDs are random/resettable; no raw chip serial, MAC address, SSID, or IP address appears in default exports.
- Remote access, cloud relays, automatic port mapping, and internet discovery are out of scope.

## Persistence and export

History is a bounded append-only sequence of validated observations/aggregates plus user events. Backups carry schema/protocol versions and checksums. Restore is validate/preview/commit, preserving current data until commit succeeds. CSV exports use explicit units and quality columns. Retention and deletion behavior must be testable under interrupted writes.
