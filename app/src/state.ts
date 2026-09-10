// App state machine: connection view, freshness/fault handling, bounded
// history with retention, and import validation before mutation.
//
// The companion never invents readings: before the first observation the
// UI shows an explicit "no data yet" state, and stale observations remain
// visible but labeled stale instead of being silently reused as fresh.

import {
  ENV_STALE_MS,
  MASS_STALE_MS,
  ProtocolError,
  assertMinorCompatible,
  isFresh,
  type DeviceError,
  type Observation,
} from "./protocol.ts";
import type { CsvRow } from "./csvio.ts";

export const HISTORY_LIMIT = 2000; // bounded view; device owns retention
/** Local retention horizon mirror of the device default (90 days). */
export const RETENTION_MS = 90 * 24 * 3600_000;

export type LinkState =
  | { kind: "offline" }
  | { kind: "connecting" }
  | { kind: "connected"; device_id: string }
  | { kind: "version_mismatch"; device_version: string };

export interface AppState {
  link: LinkState;
  /** Most recent observation, fresh or stale, with derived staleness. */
  current: Observation | null;
  currentIsStale: boolean;
  lastError: DeviceError | string | null;
  /** Neutral confirmation message shown in the same live region. */
  notice: string | null;
  history: Observation[];
  /** Locally cached import candidates awaiting explicit commit. */
  stagedImport: CsvRow[] | null;
}

export function initialState(): AppState {
  return {
    link: { kind: "offline" },
    current: null,
    currentIsStale: false,
    lastError: null,
    notice: null,
    history: [],
    stagedImport: null,
  };
}

/**
 * Apply a snapshot/SSE observation. Version-gates first: an unsupported
 * major version switches the link state and discards the payload without
 * touching history.
 */
export function applyObservation(
  state: AppState,
  observation: Observation,
  receivedAtMs: number,
): AppState {
  let parsed: Observation;
  try {
    assertMinorCompatible(observation.protocol_version);
    parsed = observation;
  } catch (error) {
    if (
      error instanceof ProtocolError &&
      error.code === "unsupported_version"
    ) {
      return {
        ...state,
        link: {
          kind: "version_mismatch",
          device_version: observation.protocol_version,
        },
        lastError: "Device protocol version is not supported",
      };
    }
    throw error;
  }
  const stale =
    !isFresh(parsed.quality.environment.sample_age_ms, ENV_STALE_MS) ||
    !isFresh(parsed.quality.mass.sample_age_ms, MASS_STALE_MS);
  const history = appendBounded(state.history, parsed, receivedAtMs);
  return {
    ...state,
    link: { kind: "connected", device_id: parsed.device_id },
    current: parsed,
    currentIsStale: stale,
    lastError: null,
    // a new reading does not erase a just-confirmed action notice
    history,
  };
}

function appendBounded(
  history: Observation[],
  observation: Observation,
  receivedAtMs: number,
): Observation[] {
  // Deduplicate SSE resends/reconnects by sequence within a session.
  const last = history[history.length - 1];
  if (
    last !== undefined &&
    last.device_id === observation.device_id &&
    observation.sequence <= last.sequence
  ) {
    return history;
  }
  const next = [...history, observation];
  // Retention: drop entries older than the horizon (explicitly bounded),
  // then enforce the view budget by evicting the oldest.
  const horizon = receivedAtMs - RETENTION_MS;
  const kept = next.filter(
    (row) => row.sampled_at === null || Date.parse(row.sampled_at) >= horizon,
  );
  return kept.length > HISTORY_LIMIT
    ? kept.slice(kept.length - HISTORY_LIMIT)
    : kept;
}

export function applyError(
  state: AppState,
  error: DeviceError | string,
): AppState {
  return { ...state, lastError: error, notice: null };
}

export function applyNotice(state: AppState, notice: string): AppState {
  return { ...state, notice, lastError: null };
}

export function markOffline(state: AppState): AppState {
  return {
    ...state,
    link: { kind: "offline" },
    // current is preserved but flagged stale — never silently reused fresh
    currentIsStale: state.current !== null ? true : false,
  };
}

function csvRowToObservation(row: CsvRow): Observation {
  return {
    protocol_version: "0.1",
    device_id: "imported",
    sequence: row.sequence,
    sampled_at: row.sampled_at,
    temperature_c: row.temperature_c,
    relative_humidity_pct: row.relative_humidity_pct,
    gross_mass_g: row.gross_mass_g,
    net_mass_estimate_g: row.net_mass_estimate_g,
    stable: row.mass_stable,
    calibration_state:
      row.calibration_state as Observation["calibration_state"],
    quality: {
      environment: {
        state: row.env_state as Observation["quality"]["environment"]["state"],
        sample_age_ms: row.env_sample_age_ms,
      },
      mass: {
        state: row.mass_state as Observation["quality"]["mass"]["state"],
        sample_age_ms: row.mass_sample_age_ms,
        uncertainty_g: row.mass_uncertainty_g,
      },
    },
    faults: row.faults,
  };
}

/** Merge validated CSV import rows into history (commit step only). */
export function commitImport(state: AppState, rows: CsvRow[]): AppState {
  const incoming = rows.map(csvRowToObservation);
  const merged = [...state.history, ...incoming].sort(
    (a, b) => a.sequence - b.sequence,
  );
  const trimmed =
    merged.length > HISTORY_LIMIT
      ? merged.slice(merged.length - HISTORY_LIMIT)
      : merged;
  return { ...state, stagedImport: null, history: trimmed };
}
