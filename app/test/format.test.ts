import { test, expect } from "vitest";
import {
  deletionConfirmationText,
  describeCalibration,
  describeFault,
  describeState,
  formatAge,
  formatGrams,
  formatTimestamp,
  formatUncertainty,
  summarizeHistory,
} from "../src/format.ts";
import type { Observation } from "../src/protocol.ts";

const base: Observation = {
  protocol_version: "0.1",
  device_id: "device-a",
  sequence: 1,
  sampled_at: "2026-09-01T12:00:00Z",
  temperature_c: 23.4,
  relative_humidity_pct: 18.2,
  gross_mass_g: 812,
  net_mass_estimate_g: 574,
  stable: true,
  calibration_state: "valid",
  quality: {
    environment: { state: "fresh", sample_age_ms: 700 },
    mass: { state: "fresh", sample_age_ms: 250, uncertainty_g: null },
  },
  faults: [],
};

test("nulls never render as a reassuring zero", () => {
  expect(formatGrams(null)).toBe("no reading");
  expect(formatGrams(812)).toBe("812.0 g");
});

test("uncertainty null shows the companion fault text", () => {
  expect(formatUncertainty(null, ["uncertainty_not_characterized"])).toBe(
    "uncertainty not characterized",
  );
  expect(formatUncertainty(2.5, [])).toBe("± 2.50 g");
});

test("calibration states are actionable sentences", () => {
  expect(describeCalibration("uncalibrated")).toContain("tare");
  expect(describeCalibration("expired")).toContain("again");
});

test("unknown fault codes still render textually", () => {
  expect(describeFault("brand_new_fault")).toContain("brand_new_fault");
});

test("state summary mentions every channel in words", () => {
  const text = describeState(base);
  expect(text).toContain("environment fresh");
  expect(text).toContain("mass fresh");
  expect(text).toContain("calibration valid");
});

test("missing wall clock is explicit", () => {
  expect(formatTimestamp(null)).toContain("clock");
});

test("age formatting is human readable", () => {
  expect(formatAge(900)).toBe("900 ms");
  expect(formatAge(65_000)).toBe("65 s");
  expect(formatAge(600_000)).toBe("10 min");
});

test("history summary is empty-safe", () => {
  expect(summarizeHistory([])).toContain("No stored");
  expect(summarizeHistory([base])).toContain("1 records");
});

test("deletion confirmations name the exact scope", () => {
  expect(deletionConfirmationText("history")).toContain(
    "stored observation history",
  );
  expect(deletionConfirmationText("all_user_data")).toContain("all user data");
});
