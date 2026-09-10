import { test, expect } from "vitest";
import { ImportError, parseExportCsv, parseCsvRows } from "../src/csvio.ts";

const HEADER =
  "schema_version,time_basis,sampled_at,sequence,temperature_c,relative_humidity_pct,env_state,env_sample_age_ms,gross_mass_g,net_mass_estimate_g,mass_state,mass_sample_age_ms,mass_stable,mass_uncertainty_g,calibration_state,faults";

function row(overrides: Partial<Record<string, string>> = {}): string {
  const base = [
    "0.1",
    "epoch_rfc3339",
    "2026-09-01T12:00:00Z",
    "42",
    "23.4",
    "18.2",
    "fresh",
    "700",
    "812.0",
    "574.0",
    "fresh",
    "250",
    "true",
    "",
    "valid",
    "uncertainty_not_characterized",
  ];
  const keys = HEADER.split(",");
  const merged = keys.map((key, index) => overrides[key] ?? base[index]);
  return merged.join(",");
}

test("parses a valid exported file", () => {
  const text = `${HEADER}\n${row()}\n`;
  const rows = parseExportCsv(text);
  expect(rows).toHaveLength(1);
  const first = rows[0] as (typeof rows)[number];
  expect(first.sequence).toBe(42);
  expect(first.temperature_c).toBeCloseTo(23.4);
  expect(first.mass_uncertainty_g).toBeNull(); // empty field = unavailable
  expect(first.faults).toEqual(["uncertainty_not_characterized"]);
});

test("empty optional fields stay null, never zero", () => {
  const text = `${HEADER}\n${row({ gross_mass_g: "", net_mass_estimate_g: "" })}\n`;
  const rows = parseExportCsv(text);
  expect((rows[0] as (typeof rows)[number]).gross_mass_g).toBeNull();
});

test("rejects wrong header", () => {
  expect(() => parseExportCsv("a,b,c\n1,2,3\n")).toThrowError(ImportError);
});

test("rejects rows from a future schema version", () => {
  const text = `${HEADER}\n${row({ schema_version: "0.2" })}\n`;
  try {
    parseExportCsv(text);
    expect.unreachable();
  } catch (error) {
    expect((error as ImportError).code).toBe("bad_version");
    expect((error as ImportError).row).toBe(2);
  }
});

test("rejects impossible humidity", () => {
  const text = `${HEADER}\n${row({ relative_humidity_pct: "140" })}\n`;
  expect(() => parseExportCsv(text)).toThrowError(ImportError);
});

test("rejects unknown state enums", () => {
  const text = `${HEADER}\n${row({ mass_state: "exploded" })}\n`;
  expect(() => parseExportCsv(text)).toThrowError(ImportError);
});

test("handles quoted fault lists and embedded commas", () => {
  const quoted = row({ faults: '"storage_fault calibration_expired"' });
  const rows = parseExportCsv(`${HEADER}\n${quoted}\n`);
  expect((rows[0] as (typeof rows)[number]).faults).toEqual([
    "storage_fault",
    "calibration_expired",
  ]);
});

test("RFC4180 parser handles embedded newlines and escaped quotes", () => {
  const rows = parseCsvRows('a,"b""q",c\nd,"e\nf",g\n');
  expect(rows[0]).toEqual(["a", 'b"q', "c"]);
  expect(rows[1]).toEqual(["d", "e\nf", "g"]);
});
