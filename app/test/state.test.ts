import { test, expect } from "vitest";
import {
  HISTORY_LIMIT,
  applyError,
  applyObservation,
  commitImport,
  initialState,
  markOffline,
} from "../src/state.ts";
import { parseExportCsv } from "../src/csvio.ts";
import type { Observation } from "../src/protocol.ts";

const HEADER =
  "schema_version,time_basis,sampled_at,sequence,temperature_c,relative_humidity_pct,env_state,env_sample_age_ms,gross_mass_g,net_mass_estimate_g,mass_state,mass_sample_age_ms,mass_stable,mass_uncertainty_g,calibration_state,faults";

function observation(overrides: Partial<Observation> = {}): Observation {
  return {
    protocol_version: "0.1",
    device_id: "device-a",
    sequence: 1,
    sampled_at: "2026-09-01T12:00:00Z",
    temperature_c: 23.4,
    relative_humidity_pct: 18.2,
    gross_mass_g: 812.0,
    net_mass_estimate_g: 574.0,
    stable: true,
    calibration_state: "valid",
    quality: {
      environment: { state: "fresh", sample_age_ms: 700 },
      mass: { state: "fresh", sample_age_ms: 250, uncertainty_g: null },
    },
    faults: ["uncertainty_not_characterized"],
    ...overrides,
  };
}

test("first observation connects and enters history", () => {
  const next = applyObservation(
    initialState(),
    observation(),
    1_700_000_000_000,
  );
  expect(next.link.kind).toBe("connected");
  expect(next.currentIsStale).toBe(false);
  expect(next.history).toHaveLength(1);
});

test("stale ages mark the reading stale but keep it visible", () => {
  const stale = observation({
    quality: {
      environment: { state: "stale", sample_age_ms: 130_000 },
      mass: { state: "fresh", sample_age_ms: 250, uncertainty_g: null },
    },
  });
  const next = applyObservation(initialState(), stale, 1_700_000_000_000);
  expect(next.current).not.toBeNull(); // preserved, labeled — not dropped
  expect(next.currentIsStale).toBe(true);
});

test("offline marks the last reading stale instead of clearing it", () => {
  let state = applyObservation(
    initialState(),
    observation(),
    1_700_000_000_000,
  );
  state = markOffline(state);
  expect(state.link.kind).toBe("offline");
  expect(state.current).not.toBeNull();
  expect(state.currentIsStale).toBe(true);
});

test("unsupported major version switches link state and touches no data", () => {
  const bad = observation({ protocol_version: "1.0" });
  const base = applyObservation(
    initialState(),
    observation(),
    1_700_000_000_000,
  );
  const next = applyObservation(base, bad, 1_700_000_000_000);
  expect(next.link.kind).toBe("version_mismatch");
  expect(next.history).toHaveLength(1); // only the earlier valid one
});

test("additive minor version is accepted", () => {
  const minor = observation({
    protocol_version: "0.2",
    // additive field must not break parsing
    ...({ extra_device_field: "ignored" } as object),
  });
  const next = applyObservation(initialState(), minor, 1_700_000_000_000);
  expect(next.link.kind).toBe("connected");
});

test("duplicate or out-of-order sequences do not corrupt history", () => {
  let state = applyObservation(
    initialState(),
    observation({ sequence: 10 }),
    1_700_000_000_000,
  );
  state = applyObservation(
    state,
    observation({ sequence: 10 }),
    1_700_000_000_000,
  );
  state = applyObservation(
    state,
    observation({ sequence: 9 }),
    1_700_000_000_000,
  );
  expect(state.history).toHaveLength(1);
});

test("history view is bounded", () => {
  let state = initialState();
  for (let seq = 0; seq < HISTORY_LIMIT + 10; seq += 1) {
    state = applyObservation(
      state,
      observation({ sequence: seq, sampled_at: null }),
      1_700_000_000_000,
    );
  }
  expect(state.history.length).toBeLessThanOrEqual(HISTORY_LIMIT);
});

test("validated CSV import merges in sequence order", () => {
  const csv = `${HEADER}\n0.1,epoch_rfc3339,2026-09-01T11:00:00Z,5,22.0,17.0,fresh,500,700.0,500.0,fresh,200,true,,valid,uncertainty_not_characterized\n`;
  const rows = parseExportCsv(csv);
  let state = applyObservation(
    initialState(),
    observation({ sequence: 10 }),
    1_700_000_000_000,
  );
  state = commitImport(state, rows);
  expect(state.history).toHaveLength(2);
  expect(state.history[0]?.sequence).toBe(5);
  expect(state.history[1]?.sequence).toBe(10);
});

test("errors are surfaced textually, never silently", () => {
  const state = applyError(initialState(), "rate_limited");
  expect(state.lastError).not.toBeNull();
});
