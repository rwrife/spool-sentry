# Device/companion protocol v0.1

**Status:** frozen design contract; schema/fixture validation exists, but no firmware/app interoperability or physical-device evidence exists yet. Requirements: [PROTO-001 through PROTO-003](requirements.md).

## Transport decision

- **LAN:** HTTP/1.1 JSON request/response plus Server-Sent Events (SSE) for one-way live observations.
- **USB:** CDC newline-delimited JSON request/response using the same operation names, payload schemas, and error codes where applicable.
- **Not v0.1:** WebSocket, cloud relay, public endpoint, automatic port mapping, internet discovery, remote administration, or TLS claims.

Unencrypted LAN HTTP assumes a trusted home/workshop network. The implementation must bind only intended local interfaces, reject unexpected Host/Origin values, and bound body size, parser depth, request/event rate, and timeouts. Exact limits become fixture-tested implementation constants in issue #4.

## Version and identity

- Every response/event includes `protocol_version: "0.1"`.
- Clients reject unsupported major versions and ignore documented additive fields from compatible minor versions.
- Request/response operations use a bounded opaque `request_id`; tokens and credentials never appear in URLs.
- `device_id` is random, resettable, and not derived from chip serial, MAC, SSID, or IP. Default exports omit it.
- Sequence numbers are unsigned monotonic counters within one boot/session. Reconnect always obtains a fresh status/snapshot; v0.1 does not promise durable event replay.

## Core observation

Canonical schema: [`schemas/observation-v0.1.schema.json`](schemas/observation-v0.1.schema.json)

Valid fixture: [`fixtures/observation-valid-v0.1.json`](fixtures/observation-valid-v0.1.json)

Unavailable numbers are `null`, never a reassuring zero. Each channel includes state and sample age. Environmental data becomes stale after 120 seconds without a valid sample; mass data becomes stale after 10 seconds without a valid sample. Disconnect, saturation, invalid calibration, impossible values, storage faults, and explicit out-of-range conditions remain distinct fault codes.

```json
{
  "protocol_version": "0.1",
  "device_id": "random-resettable-id",
  "sequence": 42,
  "sampled_at": null,
  "temperature_c": 23.4,
  "relative_humidity_pct": 18.2,
  "gross_mass_g": 812.0,
  "net_mass_estimate_g": 574.0,
  "stable": true,
  "calibration_state": "valid",
  "quality": {
    "environment": {"state": "fresh", "sample_age_ms": 700},
    "mass": {"state": "fresh", "sample_age_ms": 250, "uncertainty_g": null}
  },
  "faults": ["uncertainty_not_characterized"]
}
```

`sampled_at` is RFC 3339 UTC when trusted wall time exists and `null` otherwise. Sample ages use monotonic time. `uncertainty_g: null` requires the explicit `uncertainty_not_characterized` fault until bench characterization supports a value.

## LAN resources and USB operations

| Semantic operation | LAN | USB CDC | Protection |
|---|---|---|---|
| capability/status | `GET /api/v1/status` | `status` | read-only |
| current observation | `GET /api/v1/snapshot` | `snapshot` | read-only |
| live observations | `GET /api/v1/stream` (SSE) | not required | active local session; bounded rate |
| bounded aggregates | `GET /api/v1/observations` | `observations` | bounded range/page |
| add event/metadata | `POST /api/v1/events` | `event` | active presence session |
| tare/reference calibration | `POST /api/v1/calibration/{step}` | `calibrate` | active presence + explicit confirmation |
| export CSV/JSON | `GET /api/v1/export` | `export` | active local/cable session |
| backup | `GET /api/v1/backup` | `backup` | active local/cable session; credentials excluded |
| restore validation | `POST /api/v1/restore/validate` | `restore_validate` | bounded parse; no mutation |
| restore commit | `POST /api/v1/restore/commit` | `restore_commit` | fresh confirmation tied to validated digest |
| provision/recover Wi-Fi | onboarding route | `provision` | recent physical presence; credentials write-only |
| scoped deletion | `DELETE /api/v1/data` | `delete` | recent presence + fresh scope confirmation |
| factory reset | documented recovery route | `factory_reset` | cable + physical button confirmation |

## Mutation and error rules

1. The button opens a short-lived random presence session; expiry is automatic and monotonic.
2. Validation tokens bind operation, normalized payload digest, scope, and expiry. They are single-use.
3. Restore follows validate → preview → explicit commit. Current committed data remains available until the replacement transaction commits successfully.
4. Deletion names an explicit scope: `history`, `spool_metadata`, `calibration`, `network`, or `all_user_data`.
5. Errors use stable machine codes plus safe user text. Invalid/oversized/malformed input, unsupported versions, stale confirmations, and storage failures cause no mutation.
6. Wi-Fi credentials are write-only and excluded from status, diagnostics, logs, backups, exports, and support data.

## Persistence/export contract

- Store bounded validated hourly aggregates and user events; default target is rolling 90 days and the minimum is 30 complete days.
- CSV includes schema version, time basis, explicit units, quality, age, calibration state, uncertainty, and faults.
- JSON export and backup are distinct: export is portable observation/event data; backup additionally contains restorable user configuration/calibration but never network credentials.
- Integrity metadata detects accidental corruption; it is not a cryptographic authenticity claim.
- Retention and storage-full behavior are explicit and testable. Silent destruction of current committed data is forbidden.
