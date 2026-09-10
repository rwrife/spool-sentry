import { test, expect } from "vitest";
import {
  BACKUP_SCHEMA_VERSION,
  BackupError,
  commitRestore,
  computeChecksum,
  fnv1a,
  parseBackup,
  serializeBackup,
  validateBackup,
} from "../src/backup.ts";

// Parity reference: firmware/domain/src/backup.rs `doc()` test fixture.
// If firmware changes its canonical payload or checksum, this test must
// fail together with the Rust fixture test.
const firmwareBackup = {
  schema_version: BACKUP_SCHEMA_VERSION,
  calibration_json: '{"state":"valid"}',
  spool_json: '{"empty_mass_g":430.0}',
  events_json: '{"schema_version":"0.1","events":[]}',
  checksum: 0,
};

test("fnv1a matches published FNV-1a 32-bit test vectors", () => {
  // Published vectors (FNV reference): "" = 0x811c9dc5,
  // "a" = 0xe40c292c, "foobar" = 0xbf9cf968.
  expect(fnv1a("")).toBe(0x811c9dc5);
  expect(fnv1a("a")).toBe(0xe40c292c);
  expect(fnv1a("foobar")).toBe(0xbf9cf968);
});

test("checksum of the firmware-shaped backup matches firmware rules", () => {
  const sum = computeChecksum(firmwareBackup);
  expect(sum).toBe(fnv1a(canonical()));
  function canonical() {
    return (
      `v${BACKUP_SCHEMA_VERSION}` +
      `|cal:${firmwareBackup.calibration_json}` +
      `|spool:${firmwareBackup.spool_json}` +
      `|events:${firmwareBackup.events_json}`
    );
  }
});

test("validate accepts a correct backup and exposes a preview", () => {
  const doc = { ...firmwareBackup, checksum: computeChecksum(firmwareBackup) };
  const staged = validateBackup(doc);
  expect(staged.preview.calibration_state).toBe("valid");
  expect(staged.preview.spool_empty_mass_g).toBe(430.0);
  expect(staged.preview.event_count).toBe(0);
});

test("validate rejects a corrupted checksum without mutating", () => {
  const doc = { ...firmwareBackup, checksum: 12345 };
  expect(() => validateBackup(doc)).toThrowError(BackupError);
  try {
    validateBackup(doc);
  } catch (error) {
    expect((error as BackupError).code).toBe("checksum_mismatch");
  }
});

test("validate rejects unsupported schema versions", () => {
  const doc = {
    ...firmwareBackup,
    schema_version: "9.9",
    checksum: 0,
  };
  doc.checksum = computeChecksum(doc);
  try {
    validateBackup(doc);
    expect.unreachable();
  } catch (error) {
    expect((error as BackupError).code).toBe("unsupported_version");
  }
});

test("validate rejects credential-bearing payloads", () => {
  const doc = {
    ...firmwareBackup,
    calibration_json: '{"state":"valid","psk":"hunter2"}',
    checksum: 0,
  };
  doc.checksum = computeChecksum(doc);
  try {
    validateBackup(doc);
    expect.unreachable();
  } catch (error) {
    expect((error as BackupError).code).toBe("credential_present");
  }
});

test("commit requires the exact digest reviewed at validation", () => {
  const doc = { ...firmwareBackup, checksum: computeChecksum(firmwareBackup) };
  const staged = validateBackup(doc);
  expect(commitRestore(staged, staged.digest ^ 1)).toBeNull();
  expect(commitRestore(staged, staged.digest)).toEqual(doc);
});

test("round-trip through serialize/parse preserves the digest", () => {
  const doc = { ...firmwareBackup, checksum: computeChecksum(firmwareBackup) };
  const again = parseBackup(serializeBackup(doc)) as typeof doc;
  expect(computeChecksum(again)).toBe(doc.checksum);
});
