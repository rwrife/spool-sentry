// Versioned CSV import with explicit schema validation (DATA-002/003).
//
// Column order matches firmware/domain/src/csv.rs CSV_HEADER exactly:
// schema_version,time_basis,sampled_at,sequence,temperature_c,
// relative_humidity_pct,env_state,env_sample_age_ms,gross_mass_g,
// net_mass_estimate_g,mass_state,mass_sample_age_ms,mass_stable,
// mass_uncertainty_g,calibration_state,faults
//
// Imports validate every row before any caller mutates history; a single
// malformed or wrong-version row rejects the whole file.

export const CSV_HEADER =
  "schema_version,time_basis,sampled_at,sequence,temperature_c,relative_humidity_pct,env_state,env_sample_age_ms,gross_mass_g,net_mass_estimate_g,mass_state,mass_sample_age_ms,mass_stable,mass_uncertainty_g,calibration_state,faults";

const ENV_STATES = [
  "fresh",
  "stale",
  "disconnected",
  "out_of_range",
  "invalid",
];
const MASS_STATES = [
  "fresh",
  "settling",
  "stale",
  "disconnected",
  "saturated",
  "out_of_range",
  "invalid",
];
const CAL_STATES = ["valid", "uncalibrated", "invalid", "expired"];

export interface CsvRow {
  schema_version: string;
  time_basis: string;
  sampled_at: string | null;
  sequence: number;
  temperature_c: number | null;
  relative_humidity_pct: number | null;
  env_state: string;
  env_sample_age_ms: number | null;
  gross_mass_g: number | null;
  net_mass_estimate_g: number | null;
  mass_state: string;
  mass_sample_age_ms: number | null;
  mass_stable: boolean;
  mass_uncertainty_g: number | null;
  calibration_state: string;
  faults: string[];
}

export class ImportError extends Error {
  constructor(
    public code: "bad_header" | "bad_version" | "bad_row" | "bad_time_basis",
    public row: number | null,
    message: string,
  ) {
    super(message);
    this.name = "ImportError";
  }
}

/** RFC 4180 parser: quoted fields with "" escaping, CRLF tolerant. */
export function parseCsvRows(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;
  let i = 0;
  const pushField = () => {
    row.push(field);
    field = "";
  };
  const pushRow = () => {
    pushField();
    rows.push(row);
    row = [];
  };
  while (i < text.length) {
    const ch = text[i];
    if (inQuotes) {
      if (ch === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i += 2;
          continue;
        }
        inQuotes = false;
        i += 1;
        continue;
      }
      field += ch;
      i += 1;
      continue;
    }
    if (ch === '"') {
      inQuotes = true;
      i += 1;
      continue;
    }
    if (ch === ",") {
      pushField();
      i += 1;
      continue;
    }
    if (ch === "\r") {
      i += 1;
      continue;
    }
    if (ch === "\n") {
      pushRow();
      i += 1;
      continue;
    }
    field += ch;
    i += 1;
  }
  if (field.length > 0 || row.length > 0) pushRow();
  return rows;
}

function toNumber(value: string, row: number, col: string): number | null {
  if (value === "") return null;
  const n = Number(value);
  if (!Number.isFinite(n)) {
    throw new ImportError("bad_row", row, `${col} is not a number: ${value}`);
  }
  return n;
}

/**
 * Validate an exported CSV file. Returns parsed rows on success; throws
 * ImportError with the exact offending row otherwise. Performs no
 * mutation — the caller merges rows only after a clean parse.
 */
export function parseExportCsv(text: string): CsvRow[] {
  const rows = parseCsvRows(text);
  if (rows.length === 0 || rows[0] === undefined) {
    throw new ImportError("bad_header", null, "File is empty");
  }
  const header = rows[0];
  if (header.join(",") !== CSV_HEADER) {
    throw new ImportError(
      "bad_header",
      null,
      "CSV header does not match the v0.1 export contract",
    );
  }
  const out: CsvRow[] = [];
  for (let index = 1; index < rows.length; index += 1) {
    const raw = rows[index];
    if (raw === undefined || (raw.length === 1 && raw[0] === "")) continue;
    const rowNum = index + 1;
    if (raw.length !== 16) {
      throw new ImportError(
        "bad_row",
        rowNum,
        `Expected 16 columns, found ${raw.length}`,
      );
    }
    const [
      schemaVersion,
      timeBasis,
      sampledAt,
      sequence,
      temperature,
      humidity,
      envState,
      envAge,
      gross,
      net,
      massState,
      massAge,
      stable,
      uncertainty,
      calState,
      faults,
    ] = raw as string[];
    if (schemaVersion !== "0.1") {
      throw new ImportError(
        "bad_version",
        rowNum,
        `Row schema ${schemaVersion} is not 0.1`,
      );
    }
    if (timeBasis !== "epoch_rfc3339") {
      throw new ImportError(
        "bad_time_basis",
        rowNum,
        `Unknown basis ${timeBasis}`,
      );
    }
    if (!ENV_STATES.includes(envState ?? "")) {
      throw new ImportError("bad_row", rowNum, `Unknown env state ${envState}`);
    }
    if (!MASS_STATES.includes(massState ?? "")) {
      throw new ImportError(
        "bad_row",
        rowNum,
        `Unknown mass state ${massState}`,
      );
    }
    if (!CAL_STATES.includes(calState ?? "")) {
      throw new ImportError(
        "bad_row",
        rowNum,
        `Unknown calibration state ${calState}`,
      );
    }
    if (stable !== "true" && stable !== "false") {
      throw new ImportError("bad_row", rowNum, `stable must be true/false`);
    }
    const seq = toNumber(sequence ?? "", rowNum, "sequence");
    if (seq === null || !Number.isInteger(seq) || seq < 0) {
      throw new ImportError("bad_row", rowNum, "sequence must be >= 0");
    }
    const humidityValue = toNumber(
      humidity ?? "",
      rowNum,
      "relative_humidity_pct",
    );
    if (humidityValue !== null && (humidityValue < 0 || humidityValue > 100)) {
      throw new ImportError(
        "bad_row",
        rowNum,
        `humidity ${humidityValue} outside 0..100`,
      );
    }
    out.push({
      schema_version: schemaVersion,
      time_basis: timeBasis,
      sampled_at:
        sampledAt === "" || sampledAt === undefined ? null : sampledAt,
      sequence: seq,
      temperature_c: toNumber(temperature ?? "", rowNum, "temperature_c"),
      relative_humidity_pct: humidityValue,
      env_state: envState as string,
      env_sample_age_ms: toNumber(envAge ?? "", rowNum, "env_sample_age_ms"),
      gross_mass_g: toNumber(gross ?? "", rowNum, "gross_mass_g"),
      net_mass_estimate_g: toNumber(net ?? "", rowNum, "net_mass_estimate_g"),
      mass_state: massState as string,
      mass_sample_age_ms: toNumber(massAge ?? "", rowNum, "mass_sample_age_ms"),
      mass_stable: stable === "true",
      mass_uncertainty_g: toNumber(
        uncertainty ?? "",
        rowNum,
        "mass_uncertainty_g",
      ),
      calibration_state: calState as string,
      faults: (faults ?? "").split(" ").filter((f) => f !== ""),
    });
  }
  return out;
}
