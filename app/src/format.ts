// Display formatting: explicit units, nulls never rendered as zero, and
// textual (non-color-only) state labels (UX-001/UX-002, MEAS-004/005).

import type { CalibrationState, Observation } from "./protocol.ts";

/** Render an optional gram value with unit; null renders the explicit word. */
export function formatGrams(value: number | null, digits = 1): string {
  if (value === null || !Number.isFinite(value)) return "no reading";
  return `${value.toFixed(digits)} g`;
}

export function formatCelsius(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return "no reading";
  return `${value.toFixed(1)} \u00B0C`;
}

export function formatPercent(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return "no reading";
  return `${value.toFixed(1)} % RH`;
}

/** Uncertainty text: null with the companion fault is explicit, not hidden. */
export function formatUncertainty(
  uncertaintyG: number | null,
  faults: string[],
): string {
  if (uncertaintyG !== null && Number.isFinite(uncertaintyG)) {
    return `± ${uncertaintyG.toFixed(2)} g`;
  }
  if (faults.includes("uncertainty_not_characterized")) {
    return "uncertainty not characterized";
  }
  return "uncertainty unavailable";
}

export function formatAge(ageMs: number | null): string {
  if (ageMs === null) return "age unknown";
  if (ageMs < 1000) return `${ageMs} ms`;
  const seconds = Math.round(ageMs / 1000);
  if (seconds < 120) return `${seconds} s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 120) return `${minutes} min`;
  return `${Math.round(minutes / 60)} h`;
}

export function formatTimestamp(iso: string | null): string {
  return iso ?? "device clock not set";
}

/** Plain-language state sentence for screen readers (never color alone). */
export function describeState(observation: Observation): string {
  const parts: string[] = [];
  const env = observation.quality.environment;
  const mass = observation.quality.mass;
  parts.push(`environment ${env.state} (${formatAge(env.sample_age_ms)} old)`);
  parts.push(
    `mass ${mass.state}${observation.stable ? ", stable" : ", not stable"} (${formatAge(mass.sample_age_ms)} old)`,
  );
  parts.push(
    `calibration ${describeCalibration(observation.calibration_state)}`,
  );
  for (const fault of observation.faults) {
    parts.push(describeFault(fault));
  }
  return parts.join(". ") + ".";
}

export function describeCalibration(state: CalibrationState): string {
  switch (state) {
    case "valid":
      return "valid";
    case "uncalibrated":
      return "not calibrated yet — tare and reference mass required";
    case "invalid":
      return "invalid — calibrate again before trusting masses";
    case "expired":
      return "expired — calibrate again";
  }
}

const FAULT_TEXT: Record<string, string> = {
  mass_disconnected: "mass sensor is disconnected",
  sensor_disconnected: "environment sensor is disconnected",
  calibration_invalid: "calibration is invalid",
  calibration_expired: "calibration has expired",
  storage_fault: "device storage reported a fault",
  storage_full: "device storage is full; oldest data may be evicted",
  uncertainty_not_characterized: "mass uncertainty is not characterized yet",
};

export function describeFault(code: string): string {
  return FAULT_TEXT[code] ?? `active fault: ${code}`;
}

/** Textual chart summary + table caption (no color-only charts). */
export function summarizeHistory(rows: Observation[]): string {
  if (rows.length === 0) return "No stored observations yet.";
  const first = rows[0] as Observation;
  const last = rows[rows.length - 1] as Observation;
  return `Stored history covers ${rows.length} records from sequence ${first.sequence} to ${last.sequence}.`;
}

/** Deletion scopes from protocol.md rule 4. */
export const DELETION_SCOPES = [
  "history",
  "spool_metadata",
  "calibration",
  "network",
  "all_user_data",
] as const;

export type DeletionScope = (typeof DELETION_SCOPES)[number];

export const SCOPE_TEXT: Record<DeletionScope, string> = {
  history: "stored observation history",
  spool_metadata: "spool labels, material notes, and empty-spool mass",
  calibration: "tare and reference calibration",
  network: "saved Wi-Fi credentials",
  all_user_data: "all user data (full factory scope)",
};

export function deletionConfirmationText(scope: DeletionScope): string {
  return `Delete ${SCOPE_TEXT[scope]}? This cannot be undone unless you exported a backup.`;
}
