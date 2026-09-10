// Backup validate -> preview -> commit helpers (DATA-003).
//
// Parity with firmware/domain/src/backup.rs: canonical payload is the
// exact byte string `v{schema}|cal:{calibration_json}|spool:{spool_json}
// |events:{events_json}` (see canonical_payload there) hashed with FNV-1a
// over UTF-8 bytes. Validation performs no mutation; the caller commits
// only with a matching digest.

export const BACKUP_SCHEMA_VERSION = "0.1";

export interface BackupDoc {
  schema_version: string;
  calibration_json: string;
  spool_json: string;
  events_json: string;
  checksum: number;
}

export class BackupError extends Error {
  constructor(
    public code:
      | "unsupported_version"
      | "checksum_mismatch"
      | "malformed"
      | "credential_present",
    message: string,
  ) {
    super(message);
    this.name = "BackupError";
  }
}

/** FNV-1a 32-bit over UTF-8 bytes; must match firmware store::fnv1a. */
export function fnv1a(text: string): number {
  let hash = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(text)) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash >>> 0;
}

export function canonicalPayload(doc: BackupDoc): string {
  return `v${doc.schema_version}|cal:${doc.calibration_json}|spool:${doc.spool_json}|events:${doc.events_json}`;
}

export function computeChecksum(doc: BackupDoc): number {
  return fnv1a(canonicalPayload(doc));
}

/** Wi-Fi credentials must never appear in backups (protocol rule 6). */
const CREDENTIAL_KEYS = ["psk", "password", "passphrase", "ssid", "credential"];

function containsCredentialKey(raw: string): boolean {
  const lowered = raw.toLowerCase();
  return CREDENTIAL_KEYS.some((key) =>
    new RegExp(`"${key}"\\s*:`).test(lowered),
  );
}

/** A preview prepared by validation; commit requires the same digest. */
export interface StagedRestore {
  doc: BackupDoc;
  digest: number;
  preview: {
    calibration_state: string | null;
    spool_empty_mass_g: number | null;
    event_count: number;
  };
}

/**
 * Validate a backup without mutating local state. Throws BackupError on
 * any rule violation; the previous committed state stays in place.
 */
export function validateBackup(doc: unknown): StagedRestore {
  if (typeof doc !== "object" || doc === null) {
    throw new BackupError("malformed", "Backup is not an object");
  }
  const candidate = doc as Partial<BackupDoc>;
  if (
    typeof candidate.schema_version !== "string" ||
    typeof candidate.calibration_json !== "string" ||
    typeof candidate.spool_json !== "string" ||
    typeof candidate.events_json !== "string" ||
    typeof candidate.checksum !== "number"
  ) {
    throw new BackupError("malformed", "Backup envelope has missing fields");
  }
  if (candidate.schema_version !== BACKUP_SCHEMA_VERSION) {
    throw new BackupError(
      "unsupported_version",
      `Backup schema ${candidate.schema_version} is not supported`,
    );
  }
  const doc_: BackupDoc = {
    schema_version: candidate.schema_version,
    calibration_json: candidate.calibration_json,
    spool_json: candidate.spool_json,
    events_json: candidate.events_json,
    checksum: candidate.checksum,
  };
  if (computeChecksum(doc_) !== doc_.checksum >>> 0) {
    throw new BackupError(
      "checksum_mismatch",
      "Backup integrity check failed; the file may be corrupted",
    );
  }
  for (const field of [
    doc_.calibration_json,
    doc_.spool_json,
    doc_.events_json,
  ]) {
    const opens = (field.match(/{/g) ?? []).length;
    const closes = (field.match(/}/g) ?? []).length;
    if (opens !== closes) {
      throw new BackupError("malformed", "Backup payload has unbalanced JSON");
    }
    if (containsCredentialKey(field)) {
      throw new BackupError(
        "credential_present",
        "Backup contains credential-like fields and was rejected",
      );
    }
  }
  let calibrationState: string | null = null;
  let spoolEmpty: number | null = null;
  let eventCount = 0;
  try {
    const cal = JSON.parse(doc_.calibration_json) as { state?: unknown };
    calibrationState = typeof cal.state === "string" ? cal.state : null;
    const spool = JSON.parse(doc_.spool_json) as { empty_mass_g?: unknown };
    spoolEmpty =
      typeof spool.empty_mass_g === "number" ? spool.empty_mass_g : null;
    const events = JSON.parse(doc_.events_json) as { events?: unknown };
    eventCount = Array.isArray(events.events) ? events.events.length : 0;
  } catch {
    throw new BackupError("malformed", "Backup payload is not valid JSON");
  }
  return {
    doc: doc_,
    digest: doc_.checksum >>> 0,
    preview: {
      calibration_state: calibrationState,
      spool_empty_mass_g: spoolEmpty,
      event_count: eventCount,
    },
  };
}

/**
 * Commit a staged restore. The confirm digest must match the digest the
 * user reviewed; a mismatch means stale confirmation and changes nothing.
 */
export function commitRestore(
  staged: StagedRestore | null,
  confirmDigest: number,
): BackupDoc | null {
  if (staged === null) return null;
  if (staged.digest !== confirmDigest) return null;
  return staged.doc;
}

export function serializeBackup(doc: BackupDoc): string {
  return JSON.stringify(doc, null, 2);
}

export function parseBackup(text: string): unknown {
  return JSON.parse(text) as unknown;
}
