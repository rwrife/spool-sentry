/// <reference types="vite/client" />

// UI wiring. Vanilla DOM keeps the bundle small and dependency-free so it
// can be device-hosted on flash. All state transitions are textual (no
// color-only cues); focus order follows document order; live regions use
// role=status with aria-live=polite.

import { DeviceClient } from "./client.ts";
import {
  BackupError,
  commitRestore,
  parseBackup,
  validateBackup,
  type StagedRestore,
} from "./backup.ts";
import { ImportError } from "./csvio.ts";
import {
  describeCalibration,
  describeFault,
  describeState,
  formatAge,
  formatCelsius,
  formatGrams,
  formatPercent,
  formatTimestamp,
  formatUncertainty,
  summarizeHistory,
  DELETION_SCOPES,
  SCOPE_TEXT,
  deletionConfirmationText,
} from "./format.ts";
import type { Observation } from "./protocol.ts";
import {
  applyError,
  applyNotice,
  applyObservation,
  initialState,
  markOffline,
  type AppState,
} from "./state.ts";

const client = new DeviceClient();

let state: AppState = initialState();
/** Restore staged client-side after local validation (preview stage). */
let stagedRestore: StagedRestore | null = null;

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, string> = {},
  children: (Node | string)[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (key === "textContent") {
      node.textContent = value;
    } else {
      node.setAttribute(key, value);
    }
  }
  for (const child of children) {
    node.append(child);
  }
  return node;
}

function termRow(term: string, value: string): DocumentFragment {
  const frag = document.createDocumentFragment();
  frag.append(
    el("dt", { textContent: term }),
    el("dd", { textContent: value }),
  );
  return frag;
}

function render(): void {
  const main = document.getElementById("main");
  const statusRegion = document.getElementById("status-region");
  if (!main || !statusRegion) return;
  main.replaceChildren();
  statusRegion.replaceChildren(
    el("p", {
      textContent: [
        linkText(state),
        state.lastError ? ` — ${errorText(state)}` : "",
        state.notice ? ` — ${state.notice}` : "",
      ].join(""),
    }),
  );

  main.append(renderConnection());
  main.append(renderObservation());
  main.append(renderCalibration());
  main.append(renderSpool());
  main.append(renderEvents());
  main.append(renderHistory());
  main.append(renderDataOwnership());
}

function errorText(s: AppState): string {
  if (s.lastError === null) return "";
  return typeof s.lastError === "string" ? s.lastError : s.lastError.message;
}

function linkText(s: AppState): string {
  switch (s.link.kind) {
    case "offline":
      return "Status: offline — no device connection yet";
    case "connecting":
      return "Status: connecting to device…";
    case "connected":
      return s.currentIsStale
        ? "Status: connected, but the last reading is STALE"
        : "Status: connected and fresh";
    case "version_mismatch":
      return `Status: unsupported device protocol ${s.link.device_version} — update the companion or reflash the device`;
  }
}

function renderConnection(): HTMLElement {
  const section = el("section", { "aria-labelledby": "conn-h" }, [
    el("h2", { id: "conn-h", textContent: "Connection" }),
  ]);
  const list = el("dl");
  if (state.link.kind === "connected") {
    list.append(termRow("Device", state.link.device_id));
  }
  list.append(
    termRow("Transport", "local network or USB cable only — no internet"),
  );
  section.append(list);
  const retry = el("button", {
    type: "button",
    textContent: "Refresh status",
  });
  retry.addEventListener("click", () => void refresh());
  const provision = el("p", {}, [
    "Wi-Fi setup: press the device button to open onboarding, then follow the USB instructions in the docs. Credentials stay on the device.",
  ]);
  section.append(retry, provision);
  return section;
}

function renderObservation(): HTMLElement {
  const section = el("section", { "aria-labelledby": "obs-h" }, [
    el("h2", { id: "obs-h", textContent: "Current reading" }),
  ]);
  const obs = state.current;
  if (obs === null) {
    section.append(
      el("p", {
        textContent:
          "No data yet. Connect over the local network or USB and press Refresh.",
      }),
    );
    return section;
  }
  const badge = el("span", {
    class: "state-badge",
    textContent: state.currentIsStale ? "STALE" : "FRESH",
  });
  const list = el("dl");
  list.append(
    termRow(
      "State",
      state.currentIsStale
        ? "stale reading — do not use for decisions"
        : "fresh",
    ),
    termRow("Sampled at", formatTimestamp(obs.sampled_at)),
    termRow("Temperature", formatCelsius(obs.temperature_c)),
    termRow("Humidity", formatPercent(obs.relative_humidity_pct)),
    termRow(
      "Environment age",
      formatAge(obs.quality.environment.sample_age_ms),
    ),
    termRow("Gross mass", formatGrams(obs.gross_mass_g)),
    termRow("Net filament estimate", formatGrams(obs.net_mass_estimate_g)),
    termRow(
      "Mass state",
      `${obs.quality.mass.state}, ${obs.stable ? "stable" : "not stable"}`,
    ),
    termRow(
      "Uncertainty",
      formatUncertainty(obs.quality.mass.uncertainty_g, obs.faults),
    ),
    termRow("Calibration", describeCalibration(obs.calibration_state)),
  );
  section.append(badge, list);
  if (obs.faults.length > 0) {
    const faults = el("ul", { "aria-label": "Active faults" });
    for (const fault of obs.faults) {
      faults.append(el("li", { textContent: describeFault(fault) }));
    }
    section.append(el("h3", { textContent: "Active faults" }), faults);
  }
  section.append(
    el("p", { class: "sr-summary", textContent: describeState(obs) }),
  );
  return section;
}

function renderCalibration(): HTMLElement {
  const section = el("section", { "aria-labelledby": "cal-h" }, [
    el("h2", { id: "cal-h", textContent: "Calibration" }),
    el("p", {
      textContent:
        "Guided two-step calibration: press the device button first, tare with an empty platform, then place a known reference mass and confirm. Masses are labeled estimates, not certified measurements.",
    }),
  ]);
  const tare = el("button", { type: "button", textContent: "Start tare step" });
  tare.addEventListener("click", () => void runCalibration("tare", {}));
  const massInput = el("input", {
    id: "reference-g",
    type: "number",
    min: "10",
    max: "2500",
    step: "0.1",
    inputmode: "decimal",
  });
  const reference = el("button", {
    type: "button",
    textContent: "Start reference-mass step",
  });
  reference.addEventListener("click", () => {
    const grams = Number(massInput.value);
    if (!Number.isFinite(grams) || grams < 10 || grams > 2500) {
      state = applyError(
        state,
        "Enter a reference mass between 10 g and 2500 g",
      );
      render();
      return;
    }
    void runCalibration("reference", { reference_g: grams });
  });
  section.append(
    tare,
    el("fieldset", {}, [
      el("legend", { textContent: "Reference mass" }),
      el("label", {
        textContent: "Reference mass in grams",
        for: "reference-g",
      }),
      massInput,
      reference,
    ]),
  );
  return section;
}

async function runCalibration(
  step: "tare" | "reference",
  params: Record<string, unknown>,
): Promise<void> {
  try {
    const result = await client.calibrationStep(step, params);
    state = applyNotice(
      state,
      `${step} step armed${"token" in result ? " — confirm on the device" : ""}`,
    );
  } catch (error) {
    state = applyError(state, describeClientError(error));
  }
  render();
}

function renderSpool(): HTMLElement {
  const section = el("section", { "aria-labelledby": "spool-h" }, [
    el("h2", { id: "spool-h", textContent: "Spool details" }),
    el("p", {
      textContent:
        "Net filament mass = gross platform mass minus the empty-spool mass you enter here. It is never inferred automatically.",
    }),
    el("label", { textContent: "Empty spool mass (g)", for: "empty-mass" }),
  ]);
  const input = el("input", {
    id: "empty-mass",
    type: "number",
    min: "0",
    max: "5000",
    step: "1",
    inputmode: "numeric",
  });
  const button = el("button", {
    type: "button",
    textContent: "Save empty mass",
  });
  button.addEventListener("click", () => {
    const value = Number(input.value);
    if (!Number.isFinite(value) || value < 0) {
      state = applyError(state, "Enter a non-empty spool mass in grams");
      render();
    }
  });
  section.append(input, button);
  return section;
}

function renderEvents(): HTMLElement {
  const section = el("section", { "aria-labelledby": "events-h" }, [
    el("h2", { id: "events-h", textContent: "Event notes" }),
    el("p", {
      textContent:
        "Annotate the history: a desiccant recharge, a spool swap, or a calibration. Events need an active presence session (press the device button first).",
    }),
  ]);
  const kindSelect = el("select", { id: "event-kind" });
  const kinds: Array<[string, string]> = [
    ["desiccant", "Desiccant recharge or check"],
    ["spool", "Spool swap or metadata change"],
    ["calibration", "Calibration performed"],
    ["note", "General note"],
  ];
  for (const [value, label] of kinds) {
    kindSelect.append(el("option", { value, textContent: label }));
  }
  const detailInput = el("input", {
    id: "event-detail",
    type: "text",
    maxlength: "96",
  });
  const addButton = el("button", { type: "button", textContent: "Add event" });
  addButton.addEventListener("click", () => {
    const detail = detailInput.value.trim();
    if (detail === "") {
      state = applyError(state, "Enter a short event note first");
      render();
      return;
    }
    void client
      .addEvent(kindSelect.value, detail)
      .then(() => {
        state = applyNotice(state, "Event recorded on the device");
      })
      .catch((error: unknown) => {
        state = applyError(state, describeClientError(error));
      })
      .finally(() => render());
  });
  section.append(
    el("fieldset", {}, [
      el("legend", { textContent: "New event" }),
      el("label", { textContent: "Event type", for: "event-kind" }),
      kindSelect,
      el("label", {
        textContent: "Note (up to 96 characters)",
        for: "event-detail",
      }),
      detailInput,
      addButton,
    ]),
  );
  return section;
}

function renderHistory(): HTMLElement {
  const section = el("section", { "aria-labelledby": "hist-h" }, [
    el("h2", { id: "hist-h", textContent: "History" }),
  ]);
  section.append(el("p", { textContent: summarizeHistory(state.history) }));
  if (state.history.length === 0) return section;
  const table = el("table");
  const head = el("tr");
  for (const label of [
    "Sampled at",
    "Temp (°C)",
    "RH (%)",
    "Gross (g)",
    "Net (g)",
    "Mass state",
  ]) {
    head.append(el("th", { scope: "col", textContent: label }));
  }
  table.append(el("thead", {}, [head]));
  const body = el("tbody");
  for (const row of state.history.slice(-50)) {
    body.append(
      el("tr", {}, [
        el("td", { textContent: formatTimestamp(row.sampled_at) }),
        el("td", {
          textContent:
            row.temperature_c === null ? "—" : row.temperature_c.toFixed(1),
        }),
        el("td", {
          textContent:
            row.relative_humidity_pct === null
              ? "—"
              : row.relative_humidity_pct.toFixed(1),
        }),
        el("td", {
          textContent:
            row.gross_mass_g === null ? "—" : row.gross_mass_g.toFixed(1),
        }),
        el("td", {
          textContent:
            row.net_mass_estimate_g === null
              ? "—"
              : row.net_mass_estimate_g.toFixed(1),
        }),
        el("td", { textContent: row.quality.mass.state }),
      ]),
    );
  }
  table.append(body);
  section.append(
    el("p", {
      textContent:
        "A chart shows the same series; the table below is its text equivalent (also used by screen readers).",
    }),
    table,
  );
  return section;
}

function renderDataOwnership(): HTMLElement {
  const section = el("section", { "aria-labelledby": "data-h" }, [
    el("h2", {
      id: "data-h",
      textContent: "Your data: export, backup, restore, delete",
    }),
    el("p", {
      textContent:
        "Everything stays on the device until you export it. No cloud account, no telemetry.",
    }),
  ]);
  const csv = el("a", {
    href: client.exportUrl("csv"),
    textContent: "Export CSV",
  });
  const json = el("a", {
    href: client.exportUrl("json"),
    textContent: "Export JSON",
  });
  csv.setAttribute("download", "");
  json.setAttribute("download", "");
  section.append(el("p", {}, [csv, document.createTextNode(" "), json]));

  const deleteSelect = el("select", { id: "delete-scope" });
  for (const scope of DELETION_SCOPES) {
    deleteSelect.append(
      el("option", { value: scope, textContent: SCOPE_TEXT[scope] }),
    );
  }
  const deleteButton = el("button", {
    type: "button",
    textContent: "Delete selected scope",
  });
  deleteButton.addEventListener("click", () => {
    const scope = deleteSelect.value as (typeof DELETION_SCOPES)[number];
    if (!window.confirm(deletionConfirmationText(scope))) return;
    state = applyNotice(
      state,
      `Deletion of "${SCOPE_TEXT[scope]}" must be confirmed on the device (press the button, then confirm).`,
    );
    render();
  });
  section.append(
    el("fieldset", {}, [
      el("legend", { textContent: "Scoped deletion" }),
      el("label", { textContent: "Scope", for: "delete-scope" }),
      deleteSelect,
      deleteButton,
    ]),
  );
  section.append(renderRestoreFlow());
  return section;
}

/** Local-first restore preview: validate file locally, then commit. */
function renderRestoreFlow(): HTMLElement {
  const fieldset = el("fieldset", {}, [
    el("legend", { textContent: "Restore from backup" }),
    el("p", {
      textContent:
        "Restore validates the backup file locally first and shows a preview. Nothing changes until you confirm, and the device keeps current data until the commit succeeds.",
    }),
  ]);
  const fileInput = el("input", {
    id: "restore-file",
    type: "file",
    accept: "application/json,.json",
  });
  fieldset.append(
    el("label", { textContent: "Backup JSON file", for: "restore-file" }),
    fileInput,
  );
  const previewBox = el("div", {
    id: "restore-preview",
    role: "group",
    "aria-label": "Restore preview",
  });
  const validateButton = el("button", {
    type: "button",
    textContent: "Validate and preview",
  });
  validateButton.addEventListener("click", async () => {
    const file = fileInput.files?.[0];
    if (!file) {
      state = applyError(state, "Choose a backup JSON file first");
      render();
      return;
    }
    try {
      const text = await file.text();
      stagedRestore = validateBackup(parseBackup(text));
      const preview = stagedRestore.preview;
      previewBox.replaceChildren(
        el("p", {
          textContent:
            `Preview: calibration ${preview.calibration_state ?? "unknown"}, ` +
            `empty spool mass ${preview.spool_empty_mass_g ?? "not set"}, ` +
            `${preview.event_count} events. Digest ${stagedRestore.digest}.`,
        }),
      );
      state = applyNotice(
        state,
        "Backup validated — review the preview, then confirm",
      );
    } catch (error) {
      stagedRestore = null;
      previewBox.replaceChildren();
      state = applyError(
        state,
        error instanceof BackupError || error instanceof ImportError
          ? error.message
          : describeClientError(error),
      );
    }
    render();
  });
  const commitButton = el("button", {
    type: "button",
    textContent: "Confirm restore (device commit)",
    disabled: "",
  });
  if (stagedRestore) commitButton.removeAttribute("disabled");
  commitButton.addEventListener("click", () => {
    if (!stagedRestore) return;
    if (!window.confirm("Replace current device data with this backup?"))
      return;
    const committed = commitRestore(stagedRestore, stagedRestore.digest);
    if (committed === null) {
      state = applyError(
        state,
        "Restore digest did not match; nothing changed",
      );
    } else {
      stagedRestore = null;
      state = applyNotice(
        state,
        "Restore confirmed locally — commit completes on the device during an active presence session",
      );
      void client
        .restoreCommit(committed.checksum)
        .then(() => {
          state = applyNotice(state, "Device confirmed the restore commit");
        })
        .catch(() => {
          state = applyError(
            state,
            "Device commit pending: connect over the local network and retry the confirmation",
          );
        })
        .finally(() => render());
    }
    render();
  });
  fieldset.append(previewBox, validateButton, commitButton);
  return fieldset;
}

function describeClientError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return "The device could not complete the request.";
}

async function refresh(): Promise<void> {
  state = { ...state, link: { kind: "connecting" } };
  render();
  try {
    const observation = await client.snapshot();
    state = applyObservation(state, observation, Date.now());
  } catch (error) {
    state = applyError(markOffline(state), describeClientError(error));
  }
  render();
}

if (!import.meta.env || import.meta.env.MODE !== "test") {
  void (async () => {
    await refresh();
    const close = client.openStream(
      (observation) => {
        state = applyObservation(state, observation, Date.now());
        render();
      },
      () => {
        state = applyError(
          markOffline(state),
          "Live stream interrupted; using on-demand refresh",
        );
        render();
      },
    );
    if (close === null) {
      // No EventSource: poll bounded, only while a tab is visible.
      setInterval(() => {
        if (document.visibilityState === "visible") void refresh();
      }, 10_000);
    }
  })();
}

export { render, refresh };
